use kagi_api::{ExtractData, Filters, Lens, Personalizations, SearchRequest, SearchResponse};
use serde::{Deserialize, Serialize};

/// Cache key for search requests, containing only result-impacting fields.
///
/// Fields that do not affect the API response (format, timeout_seconds, extract)
/// are intentionally excluded so that requests differing only in those fields
/// share the same cache entry.
#[derive(Debug, Clone, Serialize)]
pub struct SearchCacheKey {
    pub query: String,
    pub workflow: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub safe_search: Option<bool>,
    pub region: Option<String>,
    pub filters: Option<Filters>,
    pub lens_id: Option<String>,
    pub lens: Option<Lens>,
    pub personalizations: Option<Personalizations>,
}

impl SearchCacheKey {
    /// Construct a cache key from a [`SearchRequest`], keeping only fields that
    /// affect the API response. Fields `format`, `timeout_seconds`, and
    /// `extract` are explicitly omitted.
    pub fn from_request(request: &SearchRequest) -> Self {
        Self {
            query: request.query().to_owned(),
            workflow: request.workflow().map(ToOwned::to_owned),
            page: request.page(),
            limit: request.limit(),
            safe_search: request.safe_search(),
            region: request.region().map(ToOwned::to_owned),
            filters: request.filters().cloned(),
            lens_id: request.lens_id().map(ToOwned::to_owned),
            lens: request.lens().cloned(),
            personalizations: request.personalizations().cloned(),
        }
    }
}

/// Cached search response, wrapping the full [`SearchResponse`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCachedResult {
    pub response: SearchResponse,
}

/// Cache key for extract requests, keyed by URL only.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractCacheKey {
    pub url: String,
}

/// Cached extract result, wrapping the [`ExtractData`] for a single URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractCachedResult {
    pub data: ExtractData,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::generate_cid;
    use kagi_api::{
        DomainKind, Filters, Lens, PersonalizationDomain, Personalizations, SearchExtractConfig,
    };

    /// A fully-populated SearchRequest used as the "base" for equality tests.
    fn base_request() -> SearchRequest {
        SearchRequest::new("rust programming")
            .with_workflow("search")
            .with_page(2)
            .with_limit(20)
            .with_safe_search(false)
            .with_region("us")
            .with_filters(Filters {
                after: Some("2024-01-01".to_owned()),
                before: None,
                region: None,
            })
            .with_lens_id("my-lens")
            .with_lens(Lens {
                sites_included: Some(vec!["example.com".to_owned()]),
                sites_excluded: None,
                keywords_included: None,
                keywords_excluded: None,
                file_type: None,
                time_after: None,
                time_before: None,
                time_relative: None,
                search_region: None,
            })
            .with_personalizations(Personalizations {
                domains: Some(vec![PersonalizationDomain {
                    domain: "example.com".to_owned(),
                    kind: DomainKind::Raise,
                }]),
                regexes: None,
            })
    }

    #[test]
    fn when_format_differs_then_cache_key_should_be_equal() {
        let req1 = base_request().with_format("markdown");
        let req2 = base_request().with_format("json");

        let key1 = SearchCacheKey::from_request(&req1);
        let key2 = SearchCacheKey::from_request(&req2);

        assert_eq!(generate_cid(&key1), generate_cid(&key2));
    }

    #[test]
    fn when_timeout_differs_then_cache_key_should_be_equal() {
        let req1 = base_request().with_timeout_seconds(2.0);
        let req2 = base_request().with_timeout_seconds(10.0);

        let key1 = SearchCacheKey::from_request(&req1);
        let key2 = SearchCacheKey::from_request(&req2);

        assert_eq!(generate_cid(&key1), generate_cid(&key2));
    }

    #[test]
    fn when_extract_differs_then_cache_key_should_be_equal() {
        let req1 = base_request().with_extract(SearchExtractConfig {
            count: Some(3),
            timeout: None,
        });
        let req2 = base_request().with_extract(SearchExtractConfig {
            count: Some(5),
            timeout: Some(2.0),
        });

        let key1 = SearchCacheKey::from_request(&req1);
        let key2 = SearchCacheKey::from_request(&req2);

        assert_eq!(generate_cid(&key1), generate_cid(&key2));
    }

    #[test]
    fn when_query_differs_then_cid_should_differ() {
        let req1 = base_request();
        let req2 = SearchRequest::new("python programming")
            .with_workflow("search")
            .with_page(2)
            .with_limit(20)
            .with_safe_search(false)
            .with_region("us")
            .with_filters(Filters {
                after: Some("2024-01-01".to_owned()),
                before: None,
                region: None,
            })
            .with_lens_id("my-lens")
            .with_lens(Lens {
                sites_included: Some(vec!["example.com".to_owned()]),
                sites_excluded: None,
                keywords_included: None,
                keywords_excluded: None,
                file_type: None,
                time_after: None,
                time_before: None,
                time_relative: None,
                search_region: None,
            })
            .with_personalizations(Personalizations {
                domains: Some(vec![PersonalizationDomain {
                    domain: "example.com".to_owned(),
                    kind: DomainKind::Raise,
                }]),
                regexes: None,
            });

        assert_ne!(
            generate_cid(&SearchCacheKey::from_request(&req1)),
            generate_cid(&SearchCacheKey::from_request(&req2))
        );
    }

    #[test]
    fn when_workflow_differs_then_cid_should_differ() {
        let req1 = base_request();
        let req2 = base_request().with_workflow("news");

        assert_ne!(
            generate_cid(&SearchCacheKey::from_request(&req1)),
            generate_cid(&SearchCacheKey::from_request(&req2))
        );
    }

    #[test]
    fn when_page_differs_then_cid_should_differ() {
        let req1 = base_request();
        let req2 = base_request().with_page(3);

        assert_ne!(
            generate_cid(&SearchCacheKey::from_request(&req1)),
            generate_cid(&SearchCacheKey::from_request(&req2))
        );
    }

    #[test]
    fn when_limit_differs_then_cid_should_differ() {
        let req1 = base_request();
        let req2 = base_request().with_limit(10);

        assert_ne!(
            generate_cid(&SearchCacheKey::from_request(&req1)),
            generate_cid(&SearchCacheKey::from_request(&req2))
        );
    }

    #[test]
    fn when_safe_search_differs_then_cid_should_differ() {
        let req1 = base_request();
        let req2 = base_request().with_safe_search(true);

        assert_ne!(
            generate_cid(&SearchCacheKey::from_request(&req1)),
            generate_cid(&SearchCacheKey::from_request(&req2))
        );
    }

    #[test]
    fn when_region_differs_then_cid_should_differ() {
        let req1 = base_request();
        let req2 = base_request().with_region("de");

        assert_ne!(
            generate_cid(&SearchCacheKey::from_request(&req1)),
            generate_cid(&SearchCacheKey::from_request(&req2))
        );
    }

    #[test]
    fn when_filters_differs_then_cid_should_differ() {
        let req1 = base_request();
        let req2 = base_request().with_filters(Filters {
            after: Some("2023-01-01".to_owned()),
            before: None,
            region: None,
        });

        assert_ne!(
            generate_cid(&SearchCacheKey::from_request(&req1)),
            generate_cid(&SearchCacheKey::from_request(&req2))
        );
    }

    #[test]
    fn when_lens_id_differs_then_cid_should_differ() {
        let req1 = base_request();
        let req2 = base_request().with_lens_id("other-lens");

        assert_ne!(
            generate_cid(&SearchCacheKey::from_request(&req1)),
            generate_cid(&SearchCacheKey::from_request(&req2))
        );
    }

    #[test]
    fn when_lens_differs_then_cid_should_differ() {
        let req1 = base_request();
        let req2 = base_request().with_lens(Lens {
            sites_included: Some(vec!["other.com".to_owned()]),
            sites_excluded: None,
            keywords_included: None,
            keywords_excluded: None,
            file_type: None,
            time_after: None,
            time_before: None,
            time_relative: None,
            search_region: None,
        });

        assert_ne!(
            generate_cid(&SearchCacheKey::from_request(&req1)),
            generate_cid(&SearchCacheKey::from_request(&req2))
        );
    }

    #[test]
    fn when_personalizations_differs_then_cid_should_differ() {
        let req1 = base_request();
        let req2 = base_request().with_personalizations(Personalizations {
            domains: Some(vec![PersonalizationDomain {
                domain: "other.com".to_owned(),
                kind: DomainKind::Lower,
            }]),
            regexes: None,
        });

        assert_ne!(
            generate_cid(&SearchCacheKey::from_request(&req1)),
            generate_cid(&SearchCacheKey::from_request(&req2))
        );
    }

    #[test]
    fn when_same_url_then_extract_cid_should_be_equal() {
        let key1 = ExtractCacheKey {
            url: "https://example.com".to_owned(),
        };
        let key2 = ExtractCacheKey {
            url: "https://example.com".to_owned(),
        };

        assert_eq!(generate_cid(&key1), generate_cid(&key2));
    }

    #[test]
    fn when_different_urls_then_extract_cid_should_differ() {
        let key1 = ExtractCacheKey {
            url: "https://example.com".to_owned(),
        };
        let key2 = ExtractCacheKey {
            url: "https://other.com".to_owned(),
        };

        assert_ne!(generate_cid(&key1), generate_cid(&key2));
    }

    #[test]
    fn when_search_cached_result_then_should_roundtrip_through_json() {
        let original = SearchCachedResult {
            response: SearchResponse {
                meta: kagi_api::Meta {
                    trace: "trace-123".to_owned(),
                    node: Some("node-1".to_owned()),
                    ms: Some(42),
                },
                data: kagi_api::SearchData {
                    search: None,
                    image: None,
                    video: None,
                    podcast: None,
                    podcast_creator: None,
                    news: None,
                    adjacent_question: None,
                    direct_answer: None,
                    interesting_news: None,
                    interesting_finds: None,
                    infobox: None,
                    code: None,
                    package_tracking: None,
                    public_records: None,
                    weather: None,
                    related_search: None,
                    listicle: None,
                    web_archive: None,
                },
            },
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: SearchCachedResult = serde_json::from_str(&json).unwrap();

        assert_eq!(
            original.response.meta.trace,
            deserialized.response.meta.trace
        );
    }

    #[test]
    fn when_extract_cached_result_then_should_roundtrip_through_json() {
        let original = ExtractCachedResult {
            data: ExtractData {
                url: "https://example.com".to_owned(),
                markdown: Some("# Hello\n\nWorld.".to_owned()),
                error: None,
            },
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ExtractCachedResult = serde_json::from_str(&json).unwrap();

        assert_eq!(original.data.url, deserialized.data.url);
        assert_eq!(original.data.markdown, deserialized.data.markdown);
    }
}
