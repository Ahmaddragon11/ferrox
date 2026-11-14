// SPDX-License-Identifier: MIT OR Apache-2.0

use ferrox_core::SearchEngineId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchProvider {
    pub engine: SearchEngineId,
    pub name: &'static str,
    pub bang: &'static str,
    pub search_url: &'static str,
}

pub const PROVIDERS: &[SearchProvider] = &[
    SearchProvider {
        engine: SearchEngineId::DuckDuckGo,
        name: "DuckDuckGo",
        bang: "ddg",
        search_url: "https://duckduckgo.com/?q={query}",
    },
    SearchProvider {
        engine: SearchEngineId::Google,
        name: "Google",
        bang: "g",
        search_url: "https://www.google.com/search?q={query}",
    },
    SearchProvider {
        engine: SearchEngineId::Brave,
        name: "Brave Search",
        bang: "b",
        search_url: "https://search.brave.com/search?q={query}",
    },
    SearchProvider {
        engine: SearchEngineId::Startpage,
        name: "Startpage",
        bang: "sp",
        search_url: "https://www.startpage.com/do/search?query={query}",
    },
    SearchProvider {
        engine: SearchEngineId::Bing,
        name: "Bing",
        bang: "bing",
        search_url: "https://www.bing.com/search?q={query}",
    },
];

/// Non-engine shortcuts that jump to popular destinations directly.
pub const DIRECT_BANGS: &[(&str, &str)] = &[
    ("yt", "https://www.youtube.com/results?search_query={query}"),
    ("gh", "https://github.com/search?q={query}"),
    ("w", "https://en.wikipedia.org/w/index.php?search={query}"),
    ("rs", "https://docs.rs/releases/search?query={query}"),
    ("maps", "https://www.openstreetmap.org/search?query={query}"),
];

pub fn provider_for_engine(engine: SearchEngineId) -> &'static SearchProvider {
    PROVIDERS
        .iter()
        .find(|p| p.engine == engine)
        .unwrap_or(&PROVIDERS[0])
}

pub fn provider_for_bang(bang: &str) -> Option<&'static SearchProvider> {
    PROVIDERS.iter().find(|p| p.bang == bang)
}

pub fn direct_bang_url(bang: &str, query: &str) -> Option<String> {
    DIRECT_BANGS
        .iter()
        .find(|(b, _)| *b == bang)
        .map(|(_, url)| {
            let encoded = ferrox_core::encode_query_component(query);
            url.replace("{query}", &encoded)
        })
}

pub fn build_search_url(engine: SearchEngineId, query: &str) -> String {
    let encoded = ferrox_core::encode_query_component(query);
    provider_for_engine(engine)
        .search_url
        .replace("{query}", &encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query_safely() {
        let url = build_search_url(SearchEngineId::Google, "rust & web browser");
        assert!(url.starts_with("https://www.google.com/search?q="));
        assert!(url.contains("rust%20%26%20web%20browser"));
    }

    #[test]
    fn direct_bangs_work() {
        let url = direct_bang_url("gh", "servo engine").unwrap();
        assert_eq!(url, "https://github.com/search?q=servo%20engine");
        assert!(direct_bang_url("nope", "x").is_none());
    }
}
