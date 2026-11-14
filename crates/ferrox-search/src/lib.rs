// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod calculator;
pub mod omnibox;
pub mod providers;

pub use calculator::{CalcError, evaluate, try_evaluate};
pub use omnibox::{OmniboxAction, OmniboxResolver, Suggestion, SuggestionKind};
pub use providers::{PROVIDERS, SearchProvider, build_search_url, provider_for_engine};
