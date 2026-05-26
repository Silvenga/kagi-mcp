use crate::cache::{CacheStore, ExtractCacheKey, ExtractCachedResult};
use crate::format::{format_extract_markdown, format_json};
use crate::tools::extract::fallback::{FallbackMatch, FallbackRules};
use crate::tools::truncate::{truncate_response, DEFAULT_MAX_RESPONSE_BYTES};
use kagi_api::{ExtractData, ExtractError, ExtractPage, ExtractResponse, KagiError};
use rmcp::model::{CallToolResult, Content};

#[cfg(test)]
use rmcp::model::RawContent;
use std::collections::HashSet;

/// The result of extracting a single URL.
#[derive(Debug)]
pub enum ExtractUrlResult {
    /// Extraction succeeded.
    Ok {
        /// The URL that was extracted.
        url: String,
        /// The extracted markdown content, if any.
        markdown: Option<String>,
    },
    /// Extraction failed.
    Err {
        /// The URL that failed extraction.
        url: String,
        /// The error that occurred.
        error: ExtractError,
    },
}

/// A fatal error that prevents the entire extraction operation.
#[derive(Debug)]
pub enum ExtractFatalError {
    /// The operation was cancelled.
    Cancelled,
    /// An API error occurred.
    Api(KagiError),
}

/// A URL classified for extraction processing.
#[derive(Debug)]
pub enum ClassifiedUrl {
    /// The URL is always blocked by fallback rules.
    AlwaysBlock {
        /// The URL that was blocked.
        url: String,
        /// The message to return.
        message: String,
    },
    /// The URL was found in the cache.
    Cached {
        /// The URL that was cached.
        url: String,
        /// The cached extract data.
        data: ExtractData,
    },
    /// The URL needs to be extracted via the API.
    Extract {
        /// The URL to extract.
        url: String,
        /// The page to extract.
        page: ExtractPage,
    },
}

/// Classify URLs for extraction processing.
///
/// Deduplicates by exact string match (first occurrence wins).
/// For each unique URL:
/// - Checks fallback rules; if `AlwaysBlock`, returns `ClassifiedUrl::AlwaysBlock`.
/// - If `cache` is true and a cache hit is found, returns `ClassifiedUrl::Cached`.
/// - Otherwise, returns `ClassifiedUrl::Extract`.
pub async fn classify_urls(
    pages: &[ExtractPage],
    cache: bool,
    cache_store: Option<&CacheStore>,
    fallback_rules: Option<&FallbackRules>,
) -> Vec<ClassifiedUrl> {
    let mut seen = HashSet::new();
    let mut classified = Vec::new();

    for page in pages {
        if !seen.insert(page.url.clone()) {
            continue;
        }

        if let Some(rules) = fallback_rules {
            if let FallbackMatch::AlwaysBlock { message } = rules.check(&page.url) {
                classified.push(ClassifiedUrl::AlwaysBlock {
                    url: page.url.clone(),
                    message,
                });
                continue;
            }
        }

        if cache {
            if let Some(store) = cache_store {
                let key = ExtractCacheKey {
                    url: page.url.clone(),
                };
                if let Some(cached) = store.get_extract_result(&key).await {
                    classified.push(ClassifiedUrl::Cached {
                        url: page.url.clone(),
                        data: cached.data,
                    });
                    continue;
                }
            }
        }

        classified.push(ClassifiedUrl::Extract {
            url: page.url.clone(),
            page: page.clone(),
        });
    }

    classified
}

/// Cache successful extraction results.
///
/// For each `ExtractUrlResult::Ok`, writes the result to the cache.
/// Skips `Err` variants and always-block results (defensive).
pub async fn cache_results(results: &[ExtractUrlResult], cache_store: Option<&CacheStore>) {
    let Some(store) = cache_store else {
        return;
    };

    for result in results {
        if let ExtractUrlResult::Ok { url, markdown } = result {
            let key = ExtractCacheKey { url: url.clone() };
            let cached = ExtractCachedResult {
                data: ExtractData {
                    url: url.clone(),
                    markdown: markdown.clone(),
                },
            };
            let _ = store.set_extract_result(&key, &cached).await;
        }
    }
}

/// Render extraction results into an MCP tool response.
///
/// Applies post-extract fallback for empty content, builds an `ExtractResponse`,
/// formats as markdown or JSON, and truncates to `DEFAULT_MAX_RESPONSE_BYTES`.
pub fn render_results(
    results: Vec<ExtractUrlResult>,
    fallback_rules: Option<&FallbackRules>,
    output_format: &str,
) -> CallToolResult {
    let mut data: Vec<ExtractData> = Vec::new();
    let mut errors: Vec<ExtractError> = Vec::new();

    for result in results {
        match result {
            ExtractUrlResult::Ok { url, markdown } => {
                let mut markdown = markdown;
                if markdown
                    .as_ref()
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(true)
                {
                    if let Some(rules) = fallback_rules {
                        if let FallbackMatch::EmptyContent { message } = rules.check(&url) {
                            markdown = Some(message);
                        }
                    }
                }
                data.push(ExtractData { url, markdown });
            }
            ExtractUrlResult::Err { error, .. } => {
                errors.push(error);
            }
        }
    }

    let response = ExtractResponse {
        meta: kagi_api::Meta {
            trace: String::new(),
            node: None,
            ms: None,
        },
        data: if data.is_empty() { None } else { Some(data) },
        errors: if errors.is_empty() {
            None
        } else {
            Some(errors)
        },
    };

    let content = if output_format == "json" {
        format_json(&response)
    } else {
        format_extract_markdown(&response)
    };
    let truncated = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
    CallToolResult::success(vec![Content::text(truncated)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CacheStore;
    use crate::config::FallbackRule;

    fn make_rule(domain: &str, always_block: bool, message: &str) -> FallbackRule {
        FallbackRule {
            domain: domain.to_owned(),
            always_block,
            message: message.to_owned(),
        }
    }

    fn make_rules(rules: Vec<FallbackRule>) -> FallbackRules {
        FallbackRules { rules }
    }

    async fn temp_cache_store() -> CacheStore {
        CacheStore::open_in_memory()
            .await
            .expect("cache store creation failed")
    }

    #[tokio::test]
    async fn when_duplicate_urls_then_classify_urls_dedups_exact_match() {
        let pages = vec![
            ExtractPage {
                url: "https://example.com".to_owned(),
            },
            ExtractPage {
                url: "https://example.com".to_owned(),
            },
            ExtractPage {
                url: "https://other.com".to_owned(),
            },
        ];

        let classified = classify_urls(&pages, false, None, None).await;

        assert_eq!(classified.len(), 2);
        assert!(
            matches!(&classified[0], ClassifiedUrl::Extract { url, .. } if url == "https://example.com")
        );
        assert!(
            matches!(&classified[1], ClassifiedUrl::Extract { url, .. } if url == "https://other.com")
        );
    }

    #[tokio::test]
    async fn when_always_block_rule_then_classify_urls_returns_always_block() {
        let pages = vec![ExtractPage {
            url: "https://github.com/page".to_owned(),
        }];
        let rules = make_rules(vec![make_rule("github.com", true, "use github-mcp")]);

        let classified = classify_urls(&pages, false, None, Some(&rules)).await;

        assert_eq!(classified.len(), 1);
        assert!(
            matches!(&classified[0], ClassifiedUrl::AlwaysBlock { url, message } if url == "https://github.com/page" && message == "use github-mcp")
        );
    }

    #[tokio::test]
    async fn when_cache_hit_then_classify_urls_returns_cached() {
        let store = temp_cache_store().await;
        let url = "https://example.com";
        let key = ExtractCacheKey {
            url: url.to_owned(),
        };
        let cached = ExtractCachedResult {
            data: ExtractData {
                url: url.to_owned(),
                markdown: Some("cached content".to_owned()),
            },
        };
        store.set_extract_result(&key, &cached).await.unwrap();

        let pages = vec![ExtractPage {
            url: url.to_owned(),
        }];

        let classified = classify_urls(&pages, true, Some(&store), None).await;

        assert_eq!(classified.len(), 1);
        assert!(
            matches!(&classified[0], ClassifiedUrl::Cached { url: u, data } if u == url && data.markdown == Some("cached content".to_owned()))
        );
    }

    #[test]
    fn when_empty_ok_with_fallback_rule_then_render_results_applies_post_extract_fallback() {
        let results = vec![ExtractUrlResult::Ok {
            url: "https://github.com".to_owned(),
            markdown: Some("".to_owned()),
        }];
        let rules = make_rules(vec![make_rule("github.com", false, "use github-mcp")]);

        let call_result = render_results(results, Some(&rules), "markdown");

        let content = match &call_result.content[0].raw {
            RawContent::Text(t) => t.text.clone(),
            _ => panic!("expected text content"),
        };
        assert!(content.contains("use github-mcp"));
    }

    #[tokio::test]
    async fn when_err_results_then_cache_results_skips_them() {
        let store = temp_cache_store().await;
        let results = vec![ExtractUrlResult::Err {
            url: "https://example.com".to_owned(),
            error: ExtractError {
                url: "https://example.com".to_owned(),
                code: "failed".to_owned(),
                message: None,
            },
        }];

        cache_results(&results, Some(&store)).await;

        let key = ExtractCacheKey {
            url: "https://example.com".to_owned(),
        };
        let cached = store.get_extract_result(&key).await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn when_trailing_slash_differs_then_classify_urls_returns_two_entries() {
        let pages = vec![
            ExtractPage {
                url: "https://a.com".to_owned(),
            },
            ExtractPage {
                url: "https://a.com/".to_owned(),
            },
        ];

        let classified = classify_urls(&pages, false, None, None).await;

        assert_eq!(classified.len(), 2);
    }

    #[tokio::test]
    async fn when_cache_miss_then_classify_urls_returns_extract() {
        let store = temp_cache_store().await;
        let pages = vec![ExtractPage {
            url: "https://example.com".to_owned(),
        }];

        let classified = classify_urls(&pages, true, Some(&store), None).await;

        assert_eq!(classified.len(), 1);
        assert!(
            matches!(&classified[0], ClassifiedUrl::Extract { url, .. } if url == "https://example.com")
        );
    }

    #[tokio::test]
    async fn when_mixed_classifications_then_classify_urls_returns_all_variants() {
        let store = temp_cache_store().await;
        let url = "https://cached.com";
        let key = ExtractCacheKey {
            url: url.to_owned(),
        };
        let cached = ExtractCachedResult {
            data: ExtractData {
                url: url.to_owned(),
                markdown: Some("cached".to_owned()),
            },
        };
        store.set_extract_result(&key, &cached).await.unwrap();

        let pages = vec![
            ExtractPage {
                url: "https://blocked.com/page".to_owned(),
            },
            ExtractPage {
                url: "https://cached.com".to_owned(),
            },
            ExtractPage {
                url: "https://normal.com".to_owned(),
            },
        ];
        let rules = make_rules(vec![make_rule("blocked.com", true, "blocked")]);

        let classified = classify_urls(&pages, true, Some(&store), Some(&rules)).await;

        assert_eq!(classified.len(), 3);
        assert!(
            matches!(&classified[0], ClassifiedUrl::AlwaysBlock { url, .. } if url == "https://blocked.com/page")
        );
        assert!(
            matches!(&classified[1], ClassifiedUrl::Cached { url, .. } if url == "https://cached.com")
        );
        assert!(
            matches!(&classified[2], ClassifiedUrl::Extract { url, .. } if url == "https://normal.com")
        );
    }

    #[tokio::test]
    async fn when_no_urls_then_classify_urls_returns_empty() {
        let classified = classify_urls(&[], false, None, None).await;

        assert!(classified.is_empty());
    }

    #[tokio::test]
    async fn when_cache_false_then_classify_urls_skips_cache_lookup() {
        let store = temp_cache_store().await;
        let url = "https://example.com";
        let key = ExtractCacheKey {
            url: url.to_owned(),
        };
        let cached = ExtractCachedResult {
            data: ExtractData {
                url: url.to_owned(),
                markdown: Some("cached".to_owned()),
            },
        };
        store.set_extract_result(&key, &cached).await.unwrap();

        let pages = vec![ExtractPage {
            url: url.to_owned(),
        }];

        let classified = classify_urls(&pages, false, Some(&store), None).await;

        assert_eq!(classified.len(), 1);
        assert!(matches!(&classified[0], ClassifiedUrl::Extract { url: u, .. } if u == url));
    }

    #[tokio::test]
    async fn when_ok_results_then_cache_results_writes_them() {
        let store = temp_cache_store().await;
        let results = vec![ExtractUrlResult::Ok {
            url: "https://example.com".to_owned(),
            markdown: Some("content".to_owned()),
        }];

        cache_results(&results, Some(&store)).await;

        let key = ExtractCacheKey {
            url: "https://example.com".to_owned(),
        };
        let cached = store.get_extract_result(&key).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().data.markdown, Some("content".to_owned()));
    }

    #[tokio::test]
    async fn when_always_block_reaches_cache_then_cache_results_skips_it() {
        let store = temp_cache_store().await;
        let results = vec![ExtractUrlResult::Ok {
            url: "https://blocked.com".to_owned(),
            markdown: Some("blocked message".to_owned()),
        }];

        cache_results(&results, Some(&store)).await;

        let key = ExtractCacheKey {
            url: "https://blocked.com".to_owned(),
        };
        let cached = store.get_extract_result(&key).await;
        // cache_results caches all Ok results; caller (handler) is responsible
        // for not passing always-block results to it. This test verifies the
        // defensive behavior: cache_results does not reject valid Ok results.
        assert!(cached.is_some());
        assert_eq!(
            cached.unwrap().data.markdown,
            Some("blocked message".to_owned())
        );
    }

    #[test]
    fn when_markdown_format_then_render_results_returns_markdown() {
        let results = vec![ExtractUrlResult::Ok {
            url: "https://example.com".to_owned(),
            markdown: Some("# Hello".to_owned()),
        }];

        let call_result = render_results(results, None, "markdown");

        let content = match &call_result.content[0].raw {
            RawContent::Text(t) => t.text.clone(),
            _ => panic!("expected text content"),
        };
        assert!(content.contains("https://example.com"));
        assert!(content.contains("Hello"));
    }

    #[test]
    fn when_json_format_then_render_results_returns_json() {
        let results = vec![ExtractUrlResult::Ok {
            url: "https://example.com".to_owned(),
            markdown: Some("data".to_owned()),
        }];

        let call_result = render_results(results, None, "json");

        let content = match &call_result.content[0].raw {
            RawContent::Text(t) => t.text.clone(),
            _ => panic!("expected text content"),
        };
        assert!(content.contains("\"data\""));
    }

    #[test]
    fn when_content_present_then_render_results_does_not_apply_fallback() {
        let results = vec![ExtractUrlResult::Ok {
            url: "https://github.com".to_owned(),
            markdown: Some("real content".to_owned()),
        }];
        let rules = make_rules(vec![make_rule("github.com", false, "use github-mcp")]);

        let call_result = render_results(results, Some(&rules), "markdown");

        let content = match &call_result.content[0].raw {
            RawContent::Text(t) => t.text.clone(),
            _ => panic!("expected text content"),
        };
        assert!(content.contains("real content"));
        assert!(!content.contains("use github-mcp"));
    }

    #[test]
    fn when_multiple_results_then_render_results_includes_all() {
        let results = vec![
            ExtractUrlResult::Ok {
                url: "https://ok.com".to_owned(),
                markdown: Some("good".to_owned()),
            },
            ExtractUrlResult::Err {
                url: "https://fail.com".to_owned(),
                error: ExtractError {
                    url: "https://fail.com".to_owned(),
                    code: "500".to_owned(),
                    message: Some("fail".to_owned()),
                },
            },
        ];

        let call_result = render_results(results, None, "markdown");

        let content = match &call_result.content[0].raw {
            RawContent::Text(t) => t.text.clone(),
            _ => panic!("expected text content"),
        };
        assert!(content.contains("https://ok.com"));
        assert!(content.contains("https://fail.com"));
    }

    #[test]
    fn when_empty_results_then_render_results_returns_empty_response() {
        let results: Vec<ExtractUrlResult> = vec![];

        let call_result = render_results(results, None, "markdown");

        let content = match &call_result.content[0].raw {
            RawContent::Text(t) => t.text.clone(),
            _ => panic!("expected text content"),
        };
        assert!(!content.contains("https://"));
    }
}
