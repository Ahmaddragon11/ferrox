// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::calculator;
use crate::providers;
use ferrox_core::{Result, SearchEngineId};
use ferrox_storage::StorageDb;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmniboxAction {
    Navigate(String),
    Search { engine: SearchEngineId, query: String },
    Internal(String),
    ShowCalculation { expression: String, result: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionKind {
    History,
    Bookmark,
    Search,
    Url,
    Calculation,
}

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub kind: SuggestionKind,
    pub title: String,
    pub url: String,
    pub relevance: u32,
}

pub struct OmniboxResolver {
    default_engine: SearchEngineId,
}

impl OmniboxResolver {
    pub fn new(default_engine: SearchEngineId) -> Self {
        Self { default_engine }
    }

    pub fn default_engine(&self) -> SearchEngineId {
        self.default_engine
    }

    pub fn set_default_engine(&mut self, engine: SearchEngineId) {
        self.default_engine = engine;
    }

    pub fn resolve(&self, raw_input: &str) -> OmniboxAction {
        let input = raw_input.trim();
        if input.is_empty() {
            return OmniboxAction::Internal("ferrox://newtab".to_string());
        }

        if let Some(rest) = input.strip_prefix('=') {
            let trimmed = rest.trim();
            if let Ok(value) = calculator::evaluate(trimmed) {
                let result = calculator::try_evaluate(trimmed)
                    .unwrap_or_else(|| format!("{value}"));
                return OmniboxAction::ShowCalculation {
                    expression: trimmed.to_string(),
                    result,
                };
            }
        }

        if let Some(action) = self.resolve_bang(input) {
            return action;
        }

        if input.starts_with("ferrox://") {
            return OmniboxAction::Internal(input.to_string());
        }

        if looks_like_url(input) {
            let normalized = if input.contains("://") {
                input.to_string()
            } else {
                format!("https://{input}")
            };
            return OmniboxAction::Navigate(normalized);
        }

        OmniboxAction::Search {
            engine: self.default_engine,
            query: input.to_string(),
        }
    }

    fn resolve_bang(&self, input: &str) -> Option<OmniboxAction> {
        let rest = input.strip_prefix('!')?;
        let mut parts = rest.splitn(2, char::is_whitespace);
        let bang = parts.next()?.to_ascii_lowercase();
        let query = parts.next().unwrap_or("").trim();
        if query.is_empty() {
            return None;
        }

        if let Some(provider) = providers::provider_for_bang(&bang) {
            return Some(OmniboxAction::Navigate(providers::build_search_url(
                provider.engine,
                query,
            )));
        }

        providers::direct_bang_url(&bang, query).map(OmniboxAction::Navigate)
    }

    /// Combines live local history/bookmark matches with the resolved action.
    pub fn suggestions(&self, db: &StorageDb, input: &str, limit: usize) -> Result<Vec<Suggestion>> {
        let mut out = Vec::new();
        let input = input.trim();
        if input.is_empty() {
            return Ok(out);
        }

        if let Some(result) = calculator::try_evaluate(input) {
            out.push(Suggestion {
                kind: SuggestionKind::Calculation,
                title: format!("= {result}"),
                url: result,
                relevance: 1000,
            });
        }

        for hit in db.search_history(input, limit)? {
            out.push(Suggestion {
                kind: SuggestionKind::History,
                title: if hit.title.is_empty() {
                    hit.url.clone()
                } else {
                    hit.title
                },
                url: hit.url,
                relevance: 500,
            });
        }

        for hit in db.search_bookmarks(input, limit)? {
            out.push(Suggestion {
                kind: SuggestionKind::Bookmark,
                title: hit.title,
                url: hit.url,
                relevance: 700,
            });
        }

        match self.resolve(input) {
            OmniboxAction::Navigate(url) => out.push(Suggestion {
                kind: SuggestionKind::Url,
                title: format!("Open {url}"),
                url,
                relevance: 400,
            }),
            OmniboxAction::Search { engine, query } => out.push(Suggestion {
                kind: SuggestionKind::Search,
                title: format!("Search {} for \"{query}\"", engine.display_name()),
                url: providers::build_search_url(engine, &query),
                relevance: 300,
            }),
            OmniboxAction::Internal(url) => out.push(Suggestion {
                kind: SuggestionKind::Url,
                title: url.clone(),
                url,
                relevance: 350,
            }),
            OmniboxAction::ShowCalculation { result, .. } => out.push(Suggestion {
                kind: SuggestionKind::Calculation,
                title: format!("= {result}"),
                url: result,
                relevance: 900,
            }),
        }

        out.sort_by(|a, b| b.relevance.cmp(&a.relevance));
        out.truncate(limit.max(1));
        Ok(out)
    }
}

fn looks_like_url(input: &str) -> bool {
    if input.contains(' ') {
        return false;
    }
    if input.starts_with("localhost") {
        return true;
    }
    if input.contains("://") {
        return true;
    }
    // IPv4 style
    if input.split('.').count() == 4 && input.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return true;
    }
    // Domain-like: at least one dot with a valid TLD-ish suffix.
    if let Some((_, tld)) = input.rsplit_once('.') {
        let tld = tld.split('/').next().unwrap_or(tld);
        return !tld.is_empty() && tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic());
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolver() -> OmniboxResolver {
        OmniboxResolver::new(SearchEngineId::DuckDuckGo)
    }

    #[test]
    fn resolves_urls_and_searches() {
        assert_eq!(
            resolver().resolve("example.com"),
            OmniboxAction::Navigate("https://example.com".into())
        );
        assert_eq!(
            resolver().resolve("ferrox://settings"),
            OmniboxAction::Internal("ferrox://settings".into())
        );
        assert_eq!(
            resolver().resolve("how to learn rust"),
            OmniboxAction::Search {
                engine: SearchEngineId::DuckDuckGo,
                query: "how to learn rust".into()
            }
        );
    }

    #[test]
    fn resolves_bangs() {
        assert_eq!(
            resolver().resolve("!g rust async"),
            OmniboxAction::Navigate("https://www.google.com/search?q=rust%20async".into())
        );
        assert_eq!(
            resolver().resolve("!gh tokio-rs"),
            OmniboxAction::Navigate("https://github.com/search?q=tokio-rs".into())
        );
    }

    #[test]
    fn resolves_calculations() {
        match resolver().resolve("= 3 * (4 + 5)") {
            OmniboxAction::ShowCalculation { result, .. } => assert_eq!(result, "27"),
            other => panic!("expected calculation, got {other:?}"),
        }
    }

    #[test]
    fn suggestions_merge_local_and_remote() {
        let db = StorageDb::open_in_memory().unwrap();
        db.record_visit("https://rust-lang.org", "Rust Language").unwrap();
        db.add_bookmark("https://doc.rust-lang.org", "Rust Docs", "Dev")
            .unwrap();

        let suggestions = resolver().suggestions(&db, "rust", 10).unwrap();
        assert!(suggestions.iter().any(|s| s.kind == SuggestionKind::History));
        assert!(suggestions.iter().any(|s| s.kind == SuggestionKind::Bookmark));
        assert!(suggestions.iter().any(|s| s.kind == SuggestionKind::Search));
    }
}
