//! Domain-level types and grouping semantics for the Kagi MCP server.
//!
//! # The `props` field on `SearchResult`
//!
//! Every `SearchResult` carries an optional `props: Option<serde_json::Value>`
//! field (see `kagi_api::types::SearchResult`). This opaque JSON object is
//! populated by the Kagi API with per-result metadata that varies across
//! result categories (`search`, `video`, `adjacent_question`, etc.).
//!
//! ## `group_id` — Grouping key
//!
//! **Field name:** `"group_id"`
//!
//! **Type:** JSON string whose value is typically the authoritative domain
//! for the result.
//!
//! **Examples:**
//! - `"rust-lang.org"`
//! - `"en.wikipedia.org"`
//! - `"youtube.com"`
//! - `"github.com"`
//!
//! **Where it appears:**
//! - `search` results — nearly always present in `props`.
//! - `video` results — present; usually the video platform domain (e.g.
//!   `"youtube.com"`, `"vimeo.com"`).
//! - `adjacent_question` results — **absent** from `props`. These carry
//!   a `"question"` key instead.
//!
//! **Purpose:** Multiple search results can originate from the same domain.
//! `group_id` is the signal that allows callers to group / deduplicate
//! results by source domain in post-processing.
//!
//! ## Companion fields (NOT used for grouping)
//!
//! The following fields sometimes coexist with `group_id` inside `props` but
//! are **not** used for any grouping or classification logic. They are
//! documented here for completeness so readers are not surprised when they
//! observe extra keys via introspection or debugging.
//!
//! | Field                 | Type    | Example    | Notes                                      |
//! |-----------------------|---------|------------|--------------------------------------------|
//! | `language`            | string  | `"en"`     | ISO 639-1 language code. Present on most   |
//! |                       |         |            | results.                                    |
//! | `language_probability`| float   | `0.98`     | Confidence of the language detection.      |
//! |                       |         |            | Sometimes absent when confidence is low.   |
//!
//! ## Example `props` JSON (sanitized)
//!
//! ```json
//! {
//!   "group_id": "rust-lang.org",
//!   "language": "en",
//!   "language_probability": 0.99
//! }
//! ```

use std::str::from_utf8;
use std::sync::LazyLock;

use publicsuffix::{List, Psl};
use url::Url;

/// Minimal public suffix data for eTLD+1 extraction.
///
/// Covers common TLDs and well-known second-level public suffixes so that
/// registrable domain extraction works correctly for URLs from popular
/// registries (`.uk`, `.au`, `.jp`, etc.) without shipping the full PSL.
const PSL_DATA: &[u8] = b"\
// BEGIN ICANN DOMAINS\n\
com\norg\nnet\ngov\nedu\nmil\n\
uk\nde\njp\nfr\nau\nbr\n\
co.uk\norg.uk\nac.uk\ngov.uk\nme.uk\nnet.uk\nsch.uk\n\
com.au\nnet.au\norg.au\n\
co.jp\nne.jp\nor.jp\n\
co.nz\nnet.nz\norg.nz\n";

static PSL_LIST: LazyLock<List> =
    LazyLock::new(|| List::from_bytes(PSL_DATA).expect("embedded PSL data is valid"));

/// Extract a grouping key for a search result.
///
/// Prefers the `group_id` field from the result's `props` metadata.
/// Falls back to extracting the eTLD+1 (registrable domain) from the
/// result's URL using the Public Suffix List.
///
/// Returns `None` when the URL is malformed or has no host.
pub(crate) fn extract_group_key(result: &kagi_api::SearchResult) -> Option<String> {
    if let Some(props) = &result.props {
        if let Some(group_id) = props.get("group_id").and_then(|v| v.as_str()) {
            if !group_id.is_empty() {
                return Some(group_id.to_owned());
            }
        }
    }

    let url = Url::parse(&result.url).ok()?;
    let host = url.host()?;

    match host {
        url::Host::Ipv4(addr) => Some(addr.to_string()),
        url::Host::Ipv6(addr) => Some(addr.to_string()),
        url::Host::Domain(domain) => {
            let list = &*PSL_LIST;
            match list.domain(domain.as_bytes()) {
                Some(parsed) => {
                    let registrable = from_utf8(parsed.as_bytes())
                        .expect("domain from URL parsing is valid UTF-8");
                    Some(registrable.to_owned())
                }
                None => {
                    let labels: Vec<&str> = domain.split('.').collect();
                    Some(if labels.len() <= 2 {
                        domain.to_owned()
                    } else {
                        labels[labels.len() - 2..].join(".")
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use kagi_api::SearchResult;

    use super::*;

    #[test]
    fn when_props_has_group_id_then_extract_should_return_it() {
        let result = SearchResult {
            url: "https://www.example.com/page".into(),
            title: String::new(),
            snippet: None,
            time: None,
            image: None,
            props: Some(serde_json::json!({"group_id": "custom-key"})),
        };

        let key = extract_group_key(&result);

        assert_eq!(key.as_deref(), Some("custom-key"));
    }

    #[test]
    fn when_props_group_id_is_empty_string_then_extract_should_fallback_to_etld1() {
        let result = SearchResult {
            url: "https://www.example.com/page".into(),
            title: String::new(),
            snippet: None,
            time: None,
            image: None,
            props: Some(serde_json::json!({"group_id": ""})),
        };

        let key = extract_group_key(&result);

        assert_eq!(key.as_deref(), Some("example.com"));
    }

    #[test]
    fn when_props_group_id_is_non_string_then_extract_should_fallback_to_etld1() {
        let result = SearchResult {
            url: "https://www.example.com/page".into(),
            title: String::new(),
            snippet: None,
            time: None,
            image: None,
            props: Some(serde_json::json!({"group_id": 42})),
        };

        let key = extract_group_key(&result);

        assert_eq!(key.as_deref(), Some("example.com"));
    }

    #[test]
    fn when_props_is_none_then_extract_should_fallback_to_etld1() {
        let result = SearchResult {
            url: "https://www.example.com/page".into(),
            title: String::new(),
            snippet: None,
            time: None,
            image: None,
            props: None,
        };

        let key = extract_group_key(&result);

        assert_eq!(key.as_deref(), Some("example.com"));
    }

    #[test]
    fn when_url_has_subdomain_then_extract_should_return_etld1() {
        let result = SearchResult {
            url: "https://sub.www.example.com/page".into(),
            title: String::new(),
            snippet: None,
            time: None,
            image: None,
            props: None,
        };

        let key = extract_group_key(&result);

        assert_eq!(key.as_deref(), Some("example.com"));
    }

    #[test]
    fn when_url_has_no_host_then_extract_should_return_none() {
        let result = SearchResult {
            url: "mailto:user@example.com".into(),
            title: String::new(),
            snippet: None,
            time: None,
            image: None,
            props: None,
        };

        let key = extract_group_key(&result);

        assert!(key.is_none());
    }

    #[test]
    fn when_url_is_ip_address_then_extract_should_return_host() {
        let result = SearchResult {
            url: "http://192.0.2.1/index.html".into(),
            title: String::new(),
            snippet: None,
            time: None,
            image: None,
            props: None,
        };

        let key = extract_group_key(&result);

        assert_eq!(key.as_deref(), Some("192.0.2.1"));
    }

    #[test]
    fn when_url_is_ipv6_address_then_extract_should_return_host() {
        let result = SearchResult {
            url: "http://[::1]/index.html".into(),
            title: String::new(),
            snippet: None,
            time: None,
            image: None,
            props: None,
        };

        let key = extract_group_key(&result);

        assert_eq!(key.as_deref(), Some("::1"));
    }

    #[test]
    fn when_url_is_malformed_then_extract_should_return_none() {
        let result = SearchResult {
            url: "not-a-valid-url".into(),
            title: String::new(),
            snippet: None,
            time: None,
            image: None,
            props: None,
        };

        let key = extract_group_key(&result);

        assert!(key.is_none());
    }

    #[test]
    fn when_two_subdomains_of_same_etld1_then_both_keys_should_be_equal() {
        let result_a = SearchResult {
            url: "https://blog.example.com/post1".into(),
            title: String::new(),
            snippet: None,
            time: None,
            image: None,
            props: None,
        };
        let result_b = SearchResult {
            url: "https://www.example.com/post2".into(),
            title: String::new(),
            snippet: None,
            time: None,
            image: None,
            props: None,
        };

        let key_a = extract_group_key(&result_a);
        let key_b = extract_group_key(&result_b);

        assert_eq!(key_a, key_b);
        assert_eq!(key_a.as_deref(), Some("example.com"));
    }

    #[test]
    fn when_url_uses_public_suffix_like_co_uk_then_extract_should_handle_correctly() {
        let result = SearchResult {
            url: "https://sub.example.co.uk/page".into(),
            title: String::new(),
            snippet: None,
            time: None,
            image: None,
            props: None,
        };

        let key = extract_group_key(&result);

        assert_eq!(key.as_deref(), Some("example.co.uk"));
    }

    #[test]
    fn when_loading_fixture_then_all_records_should_have_a_group_key() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixture_path = manifest_dir.join("tests/fixtures/search_response_with_group_id.json");
        let data = fs::read_to_string(fixture_path).expect("fixture file exists");
        let response: kagi_api::SearchResponse =
            serde_json::from_str(&data).expect("fixture is valid JSON");

        let all_results = response
            .data
            .search
            .into_iter()
            .flatten()
            .chain(response.data.video.into_iter().flatten())
            .chain(response.data.adjacent_question.into_iter().flatten());

        for result in all_results {
            assert!(
                extract_group_key(&result).is_some(),
                "expected a group key for URL: {}",
                result.url,
            );
        }
    }
}
