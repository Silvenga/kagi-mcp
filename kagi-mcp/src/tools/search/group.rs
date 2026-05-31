use crate::tools::domain::extract_registrable_domain;

/// Extract a grouping key for a search result.
///
/// Prefers the `group_id` field from the result's `props` metadata.
/// Falls back to extracting the eTLD+1 (registrable domain) from the
/// result's URL using the Public Suffix List.
///
/// Returns `None` when the URL is malformed or has no host.
pub fn extract_group_key(result: &kagi_api::SearchResult) -> Option<String> {
    if let Some(props) = &result.props {
        if let Some(group_id) = props.get("group_id").and_then(|v| v.as_str()) {
            if !group_id.is_empty() {
                return Some(group_id.to_owned());
            }
        }
    }

    extract_registrable_domain(&result.url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::SearchResult;
    use std::fs;
    use std::path::Path;

    #[test]
    fn when_props_has_group_id_then_extract_should_return_it() {
        let result = SearchResult {
            url: "https://www.example.com/page".into(),
            title: None,
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
            title: None,
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
            title: None,
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
            title: None,
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
            title: None,
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
            title: None,
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
            title: None,
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
            title: None,
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
            title: None,
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
            title: None,
            snippet: None,
            time: None,
            image: None,
            props: None,
        };
        let result_b = SearchResult {
            url: "https://www.example.com/post2".into(),
            title: None,
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
            title: None,
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
