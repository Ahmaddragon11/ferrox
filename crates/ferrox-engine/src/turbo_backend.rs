// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::internal_pages::escape_html;
use ferrox_core::{FerroxError, Result};
use ferrox_net::FastNetClient;
use scraper::{ElementRef, Html, Selector};
use url::Url;

#[derive(Debug, Clone)]
pub struct TurboDocument {
    pub final_url: String,
    pub title: String,
    pub site_name: String,
    pub html: String,
    pub excerpt: String,
    pub word_count: usize,
    pub read_minutes: usize,
    pub load_ms: u64,
    pub from_cache: bool,
    pub blocks: usize,
}

/// Pure-Rust readability engine: fetches HTML, extracts the main content, and
/// renders an extremely light document with zero JavaScript execution.
#[derive(Clone)]
pub struct TurboReader {
    net: FastNetClient,
}

impl TurboReader {
    pub fn new(net: FastNetClient) -> Self {
        Self { net }
    }

    pub async fn fetch(&self, url: &str) -> Result<TurboDocument> {
        let started = std::time::Instant::now();
        let fetched = self.net.fetch_cached(url, true).await?;

        let looks_html = fetched.content_type.contains("html")
            || fetched.content_type.is_empty()
            || fetched.body.starts_with(b"<!")
            || fetched.body.starts_with(b"<html")
            || fetched.body.starts_with(b"<");
        if !looks_html {
            return Err(FerroxError::Engine(format!(
                "not an HTML document ({}); falling back to full mode",
                fetched.content_type
            )));
        }

        let raw = String::from_utf8_lossy(&fetched.body).to_string();
        let mut doc = Self::render_document(&fetched.url, &raw);
        doc.load_ms = started.elapsed().as_millis() as u64;
        doc.from_cache = fetched.from_cache;
        Ok(doc)
    }

    pub fn render_document(final_url: &str, raw_html: &str) -> TurboDocument {
        let document = Html::parse_document(raw_html);
        let base = Url::parse(final_url).ok();

        let title = extract_title(&document, base.as_ref());
        let site_name = meta_content(&document, "meta[property=\"og:site_name\"]")
            .or_else(|| base.as_ref().and_then(|u| u.host_str().map(str::to_string)))
            .unwrap_or_else(|| "Web".to_string());
        let excerpt = meta_content(&document, "meta[name=\"description\"]")
            .or_else(|| meta_content(&document, "meta[property=\"og:description\"]"))
            .unwrap_or_default();

        let content = Selector::parse("article")
            .ok()
            .and_then(|s| document.select(&s).next())
            .or_else(|| {
                Selector::parse("main")
                    .ok()
                    .and_then(|s| document.select(&s).next())
            })
            .or_else(|| {
                Selector::parse("body")
                    .ok()
                    .and_then(|s| document.select(&s).next())
            });

        let block_selector = Selector::parse("h1,h2,h3,h4,h5,h6,p,li,blockquote,pre").ok();
        let image_selector = Selector::parse("article img, main img").ok();

        let mut blocks_html = String::new();
        let mut text_accum = String::new();
        let mut blocks = 0usize;
        let mut last_text = String::new();

        if let (Some(root), Some(selector)) = (content, block_selector.as_ref()) {
            for element in root.select(selector) {
                if blocks >= 420 || text_accum.len() > 220_000 {
                    break;
                }
                let tag = element.value().name();
                let text = collapse_whitespace(&element.text().collect::<String>());
                if text.is_empty() {
                    continue;
                }
                if text.len() < 18 && !tag.starts_with('h') {
                    continue;
                }
                if text == last_text {
                    continue;
                }
                last_text = text.clone();

                match tag {
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        let level = tag[1..].parse::<u8>().unwrap_or(2).clamp(2, 4);
                        blocks_html
                            .push_str(&format!("<h{level}>{}</h{level}>", escape_html(&text)));
                    }
                    "blockquote" => {
                        blocks_html
                            .push_str(&format!("<blockquote>{}</blockquote>", escape_html(&text)));
                    }
                    "pre" => {
                        blocks_html.push_str(&format!("<pre>{}</pre>", escape_html(&text)));
                    }
                    "li" => {
                        blocks_html
                            .push_str(&format!("<p class=\"li\">{}</p>", escape_html(&text)));
                    }
                    _ => {
                        blocks_html.push_str(&format!("<p>{}</p>", escape_html(&text)));
                    }
                }
                text_accum.push_str(&text);
                text_accum.push(' ');
                blocks += 1;
            }
        }

        if blocks == 0 {
            let fallback = collapse_whitespace(&document.root_element().text().collect::<String>());
            if !fallback.is_empty() {
                blocks_html.push_str(&format!("<p>{}</p>", escape_html(&fallback)));
                text_accum = fallback;
                blocks = 1;
            }
        }

        // Inline up to six images from the main content, resolved to absolute URLs.
        let mut images_html = String::new();
        if let (Some(root), Some(selector), Some(base)) =
            (content, image_selector.as_ref(), base.as_ref())
        {
            for element in root.select(selector).take(6) {
                if let Some(src) = element.value().attr("src")
                    && let Ok(resolved) = base.join(src)
                {
                    let alt = element.value().attr("alt").unwrap_or("").to_string();
                    images_html.push_str(&format!(
                        "<figure><img loading=\"lazy\" src=\"{}\" alt=\"{}\"></figure>",
                        escape_html(resolved.as_str()),
                        escape_html(&alt)
                    ));
                }
            }
        }

        let word_count = text_accum.split_whitespace().count();
        let read_minutes = (word_count / 215).max(1);

        let html = reader_shell(&ReaderShell {
            title: &title,
            site_name: &site_name,
            url: final_url,
            excerpt: &excerpt,
            read_minutes,
            word_count,
            images: &images_html,
            content: &blocks_html,
        });

        TurboDocument {
            final_url: final_url.to_string(),
            title,
            site_name,
            html,
            excerpt,
            word_count,
            read_minutes,
            load_ms: 0,
            from_cache: false,
            blocks,
        }
    }
}

/// Everything the reader template needs to render a page.
struct ReaderShell<'a> {
    title: &'a str,
    site_name: &'a str,
    url: &'a str,
    excerpt: &'a str,
    read_minutes: usize,
    word_count: usize,
    images: &'a str,
    content: &'a str,
}

fn reader_shell(page: &ReaderShell<'_>) -> String {
    let ReaderShell {
        title,
        site_name,
        url,
        excerpt,
        read_minutes,
        word_count,
        images,
        content,
    } = *page;
    let url_js = escape_html(&serde_json::to_string(url).unwrap_or_else(|_| "\"\"".to_string()));
    format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title}</title>
<style>
:root{{--bg:#fbfaf7;--text:#1d2320;--muted:#68736d;--line:#e4e1d8;--accent:#c2410c;--panel:#f4f2ec;--code:#f0ede4}}
@media (prefers-color-scheme: dark){{
  :root{{--bg:#0d1117;--text:#e6e9ee;--muted:#8b96a7;--line:#232a35;--accent:#ff8a5c;--panel:#161b23;--code:#131922}}
}}
*{{box-sizing:border-box}}
body{{margin:0;background:var(--bg);color:var(--text);
  font-family:Charter,"Iowan Old Style","Source Serif 4",Georgia,serif;
  font-size:19px;line-height:1.72;-webkit-font-smoothing:antialiased}}
.bar{{position:sticky;top:0;z-index:5;background:color-mix(in srgb,var(--bg) 86%,transparent);
  backdrop-filter:blur(14px);border-bottom:1px solid var(--line);
  font-family:Inter,-apple-system,"Segoe UI",system-ui,sans-serif;font-size:13px}}
.bar .in{{max-width:820px;margin:0 auto;padding:10px 22px;display:flex;align-items:center;gap:12px}}
.badge{{background:var(--panel);border:1px solid var(--line);border-radius:999px;
  padding:4px 10px;color:var(--muted);font-size:11px;letter-spacing:.8px;text-transform:uppercase;white-space:nowrap}}
.bar .spacer{{flex:1}}
.bar button{{font:inherit;background:var(--panel);border:1px solid var(--line);color:var(--text);
  border-radius:9px;padding:7px 12px;cursor:pointer}}
.bar button:hover{{border-color:var(--accent);color:var(--accent)}}
main{{max-width:760px;margin:0 auto;padding:34px 22px 90px}}
h1.title{{font-family:Inter,-apple-system,"Segoe UI",system-ui,sans-serif;
  font-size:clamp(28px,4.4vw,44px);line-height:1.14;letter-spacing:-1px;margin:8px 0 10px}}
.site{{font-family:Inter,system-ui,sans-serif;color:var(--muted);font-size:12px;
  letter-spacing:1px;text-transform:uppercase}}
.desc{{font-family:Inter,system-ui,sans-serif;color:var(--muted);font-size:15px;
  line-height:1.6;margin:0 0 26px;padding-bottom:22px;border-bottom:1px solid var(--line)}}
article h2,article h3,article h4{{font-family:Inter,-apple-system,"Segoe UI",system-ui,sans-serif;
  line-height:1.25;letter-spacing:-.4px;margin:1.9em 0 .5em}}
article h2{{font-size:28px}} article h3{{font-size:23px}} article h4{{font-size:19px}}
article p{{margin:0 0 1.15em}}
article p.li{{position:relative;padding-left:1.3em}}
article p.li::before{{content:"";position:absolute;left:.2em;top:.62em;width:7px;height:7px;
  border-radius:50%;background:var(--accent)}}
blockquote{{margin:1.6em 0;padding:10px 20px;border-left:3px solid var(--accent);
  background:var(--panel);border-radius:0 10px 10px 0;color:var(--muted);font-style:italic}}
pre{{background:var(--code);border:1px solid var(--line);border-radius:12px;padding:16px;
  overflow:auto;font-family:ui-monospace,Consolas,monospace;font-size:13px;line-height:1.55}}
figure{{margin:1.6em 0}}figure img{{width:100%;border-radius:14px;border:1px solid var(--line);display:block}}
.foot{{max-width:760px;margin:0 auto;padding:0 22px 60px;font-family:Inter,system-ui,sans-serif;
  color:var(--muted);font-size:12px;display:flex;justify-content:space-between;gap:14px;
  border-top:1px solid var(--line);padding-top:18px}}
</style></head>
<body>
<div class="bar"><div class="in">
  <span class="badge">Turbo reading</span>
  <span class="badge">{words} words · {minutes} min</span>
  <span class="spacer"></span>
  <button onclick="ferrox('full_mode',{{url:{url_js}}})">Open full site</button>
</div></div>
<main>
  <div class="site">{site}</div>
  <h1 class="title">{title}</h1>
  <p class="desc">{excerpt}</p>
  {images}
  <article>{content}</article>
</main>
<div class="foot"><span>Rendered by Ferrox Turbo · zero JavaScript</span><span>{site}</span></div>
<script>
function ferrox(cmd,args){{try{{window.ipc.postMessage(JSON.stringify(Object.assign({{cmd:cmd}},args||{{}})));}}catch(e){{}}}}
</script>
</body></html>"#,
        title = escape_html(title),
        site = escape_html(site_name),
        url_js = url_js,
        excerpt = escape_html(excerpt),
        words = word_count,
        minutes = read_minutes,
        images = images,
        content = content,
    )
}

fn extract_title(document: &Html, base: Option<&Url>) -> String {
    if let Some(title) = meta_content(document, "meta[property=\"og:title\"]") {
        return title;
    }
    if let Some(selector) = Selector::parse("title").ok()
        && let Some(el) = document.select(&selector).next()
    {
        let raw = collapse_whitespace(&el.text().collect::<String>());
        if !raw.is_empty() {
            let cleaned = raw.split(" | ").next().unwrap_or(&raw).trim().to_string();
            return if cleaned.is_empty() { raw } else { cleaned };
        }
    }
    if let Some(selector) = Selector::parse("h1").ok()
        && let Some(el) = document.select(&selector).next()
    {
        let raw = collapse_whitespace(&el.text().collect::<String>());
        if !raw.is_empty() {
            return raw;
        }
    }
    base.and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| "Untitled".to_string())
}

fn meta_content(document: &Html, selector: &str) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
    let el: ElementRef<'_> = document.select(&selector).next()?;
    let content = el.value().attr("content")?.trim();
    if content.is_empty() {
        None
    } else {
        Some(collapse_whitespace(content))
    }
}

fn collapse_whitespace(raw: &str) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<!doctype html><html><head>
        <title>Understanding Rust Ownership | Ferrox Blog</title>
        <meta name="description" content="A practical guide to ownership and borrowing.">
        <meta property="og:site_name" content="Ferrox Blog">
    </head><body>
        <nav>Home About</nav>
        <article>
          <h1>Understanding Rust Ownership</h1>
          <p>Ownership is the most distinctive feature of Rust and enables memory safety without a garbage collector.</p>
          <p>Borrowing lets you reference values without taking ownership, while the borrow checker validates lifetimes at compile time.</p>
          <blockquote>Zero-cost abstractions with compile-time guarantees.</blockquote>
          <ul><li>Each value has a single owner.</li><li>References must always be valid.</li></ul>
        </article>
        <footer>Copyright</footer>
    </body></html>"#;

    #[test]
    fn extracts_article_and_strips_chrome() {
        let doc = TurboReader::render_document("https://blog.example.com/rust", SAMPLE);
        assert_eq!(doc.title, "Understanding Rust Ownership");
        assert_eq!(doc.site_name, "Ferrox Blog");
        assert!(doc.word_count > 20);
        assert!(doc.read_minutes >= 1);
        assert!(doc.blocks >= 5);
        assert!(doc.html.contains("borrow checker"));
        assert!(!doc.html.contains("Copyright"));
        assert!(doc.html.contains("Turbo reading"));

        // The extracted article body must never carry page scripts or handlers.
        let content = doc
            .html
            .split_once("<article>")
            .and_then(|(_, rest)| rest.split_once("</article>"))
            .map(|(content, _)| content)
            .expect("reader output contains an article body");
        assert!(!content.contains("<script"));
        assert!(!content.contains("onclick="));
        assert!(!content.contains("<iframe"));
    }

    #[test]
    fn escapes_embedded_markup() {
        let html = r#"<html><body><article><p>Hello &lt;script&gt;alert(1)&lt;/script&gt; world, this is a test paragraph.</p></article></body></html>"#;
        let doc = TurboReader::render_document("https://example.com", html);
        assert!(!doc.html.contains("<script>alert(1)</script>"));
    }
}
