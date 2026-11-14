// SPDX-License-Identifier: MIT OR Apache-2.0

//! Percent-encoding helpers shared by the omnibox and the search providers.
//!
//! `url::form_urlencoded` follows the `application/x-www-form-urlencoded`
//! rules where a space becomes `+`. That is legal for HTML form submissions
//! but several search engines treat `+` literally, so Ferrox encodes query
//! components with the stricter RFC 3986 rules instead.

use percent_encoding::{AsciiSet, CONTROLS, NON_ALPHANUMERIC};

/// Everything that may stay untouched inside a query component.
const QUERY_COMPONENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'=')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'|')
    .add(b'+')
    .add(b'&')
    .add(b'$')
    .add(b',')
    .add(b'%')
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Percent-encodes a query string component, using `%20` for spaces.
pub fn encode_query_component(raw: &str) -> String {
    percent_encoding::utf8_percent_encode(raw, QUERY_COMPONENT).to_string()
}

/// Percent-encodes a path segment, using `%20` for spaces.
pub fn encode_path_segment(raw: &str) -> String {
    percent_encoding::utf8_percent_encode(raw, NON_ALPHANUMERIC).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaces_use_percent_twenty() {
        assert_eq!(encode_query_component("rust async"), "rust%20async");
        assert_eq!(encode_query_component("a & b"), "a%20%26%20b");
    }

    #[test]
    fn keeps_unreserved_characters() {
        assert_eq!(
            encode_query_component("docs.rs-2024_edition~1"),
            "docs.rs-2024_edition~1"
        );
    }

    #[test]
    fn encodes_unicode() {
        assert_eq!(encode_query_component("café"), "caf%C3%A9");
    }
}
