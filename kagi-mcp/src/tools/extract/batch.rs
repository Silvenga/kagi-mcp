use crate::tools::extract::errors::kagi_error_to_extract_error;
use crate::tools::extract::pipeline::{ExtractFatalError, ExtractUrlResult};
use kagi_api::{ExtractError, ExtractPage, ExtractRequest, KagiApi, KagiError};
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use std::sync::Arc;

pub async fn extract_batch(
    client: Arc<dyn KagiApi>,
    ctx: &RequestContext<RoleServer>,
    extract_timeout: f64,
    pages: Vec<ExtractPage>,
) -> Result<Vec<ExtractUrlResult>, ExtractFatalError> {
    let request = ExtractRequest::new(pages.clone())
        .with_format("json".to_owned())
        .with_timeout_seconds(extract_timeout);

    let result = tokio::select! {
        _ = ctx.ct.cancelled() => {
            return Err(ExtractFatalError::Cancelled);
        }
        result = client.extract(request) => result,
    };

    match result {
        Ok(response) => {
            let mut results = Vec::new();
            for page in &pages {
                let matched = response.data.as_ref().and_then(|data| {
                    data.iter()
                        .find(|d| d.url.trim_end_matches('/') == page.url.trim_end_matches('/'))
                });
                if let Some(data) = matched {
                    match (&data.markdown, &data.error) {
                        (Some(md), _) => {
                            results.push(ExtractUrlResult::Ok {
                                url: page.url.clone(),
                                markdown: Some(md.clone()),
                            });
                        }
                        (None, Some(err)) => {
                            results.push(ExtractUrlResult::Err {
                                url: page.url.clone(),
                                error: ExtractError {
                                    url: page.url.clone(),
                                    code: "extract_failed".to_owned(),
                                    message: Some(err.clone()),
                                },
                            });
                        }
                        (None, None) => {
                            results.push(ExtractUrlResult::Ok {
                                url: page.url.clone(),
                                markdown: None,
                            });
                        }
                    }
                } else {
                    results.push(ExtractUrlResult::Err {
                        url: page.url.clone(),
                        error: kagi_error_to_extract_error(&page.url, &KagiError::ServerError),
                    });
                }
            }
            Ok(results)
        }
        Err(kagi_error) => Err(ExtractFatalError::Api(kagi_error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::KagiMcpServer;
    use kagi_api::{ExtractData, ExtractResponse, Meta, MockKagiApi};
    use rmcp::model::{ClientInfo, RequestId};
    use rmcp::service::serve_directly_with_ct;
    use std::sync::Arc;
    use tokio::io::duplex;
    use tokio_util::sync::CancellationToken;

    async fn fake_request_context() -> RequestContext<RoleServer> {
        let (server_transport, client_transport) = duplex(4096);
        drop(client_transport);

        let server = KagiMcpServer::with_client(Arc::new(MockKagiApi::new()));
        let server_svc = serve_directly_with_ct(
            server,
            server_transport,
            None::<ClientInfo>,
            CancellationToken::new(),
        );

        let peer = server_svc.peer().clone();
        drop(server_svc);

        RequestContext::new(RequestId::Number(1), peer)
    }

    #[tokio::test]
    async fn when_multi_url_succeeds_then_extract_batch_returns_all_ok() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![
                    ExtractData {
                        url: "https://a.com".to_owned(),
                        markdown: Some("content a".to_owned()),
                        error: None,
                    },
                    ExtractData {
                        url: "https://b.com".to_owned(),
                        markdown: Some("content b".to_owned()),
                        error: None,
                    },
                ]),
                errors: None,
            })
        });

        let ctx = fake_request_context().await;
        let pages = vec![
            ExtractPage {
                url: "https://a.com".to_owned(),
            },
            ExtractPage {
                url: "https://b.com".to_owned(),
            },
        ];

        let result = extract_batch(Arc::new(mock), &ctx, 10.0, pages).await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 2);
        assert!(matches!(&results[0], ExtractUrlResult::Ok { url, .. } if url == "https://a.com"));
        assert!(matches!(&results[1], ExtractUrlResult::Ok { url, .. } if url == "https://b.com"));
    }

    #[tokio::test]
    async fn when_api_fails_then_extract_batch_returns_fatal_api_error() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract()
            .times(1)
            .returning(|_| Err(kagi_api::KagiError::ServerError));

        let ctx = fake_request_context().await;
        let pages = vec![ExtractPage {
            url: "https://example.com".to_owned(),
        }];

        let result = extract_batch(Arc::new(mock), &ctx, 10.0, pages).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ExtractFatalError::Api(_)));
    }

    #[tokio::test]
    async fn when_trailing_slash_differs_then_correlation_works() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://example.com".to_owned(),
                    markdown: Some("content".to_owned()),
                    error: None,
                }]),
                errors: None,
            })
        });

        let ctx = fake_request_context().await;
        let pages = vec![ExtractPage {
            url: "https://example.com/".to_owned(),
        }];

        let result = extract_batch(Arc::new(mock), &ctx, 10.0, pages).await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert!(
            matches!(&results[0], ExtractUrlResult::Ok { url, .. } if url == "https://example.com/")
        );
    }

    #[tokio::test]
    async fn when_url_missing_from_response_then_extract_batch_returns_err() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://present.com".to_owned(),
                    markdown: Some("content".to_owned()),
                    error: None,
                }]),
                errors: None,
            })
        });

        let ctx = fake_request_context().await;
        let pages = vec![
            ExtractPage {
                url: "https://present.com".to_owned(),
            },
            ExtractPage {
                url: "https://missing.com".to_owned(),
            },
        ];

        let result = extract_batch(Arc::new(mock), &ctx, 10.0, pages).await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 2);
        assert!(
            matches!(&results[0], ExtractUrlResult::Ok { url, .. } if url == "https://present.com")
        );
        assert!(
            matches!(&results[1], ExtractUrlResult::Err { url, .. } if url == "https://missing.com")
        );
    }

    #[tokio::test]
    async fn when_extract_data_has_markdown_and_error_then_markdown_wins() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://example.com".to_owned(),
                    markdown: Some("content".to_owned()),
                    error: Some("ignored".to_owned()),
                }]),
                errors: None,
            })
        });

        let ctx = fake_request_context().await;
        let pages = vec![ExtractPage {
            url: "https://example.com".to_owned(),
        }];

        let result = extract_batch(Arc::new(mock), &ctx, 10.0, pages).await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert!(
            matches!(&results[0], ExtractUrlResult::Ok { url, markdown } if url == "https://example.com" && markdown == &Some("content".to_owned()))
        );
    }

    #[tokio::test]
    async fn when_extract_data_has_error_then_should_return_err() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://example.com".to_owned(),
                    markdown: None,
                    error: Some("extract.invalid_url".to_owned()),
                }]),
                errors: None,
            })
        });

        let ctx = fake_request_context().await;
        let pages = vec![ExtractPage {
            url: "https://example.com".to_owned(),
        }];

        let result = extract_batch(Arc::new(mock), &ctx, 10.0, pages).await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert!(
            matches!(&results[0], ExtractUrlResult::Err { url, error } if url == "https://example.com" && error.code == "extract_failed" && error.message == Some("extract.invalid_url".to_owned()))
        );
    }

    #[tokio::test]
    async fn when_extract_data_has_neither_markdown_nor_error_then_returns_ok_empty() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(1).returning(|_| {
            Ok(ExtractResponse {
                meta: Meta {
                    trace: "test".to_owned(),
                    node: None,
                    ms: None,
                },
                data: Some(vec![ExtractData {
                    url: "https://example.com".to_owned(),
                    markdown: None,
                    error: None,
                }]),
                errors: None,
            })
        });

        let ctx = fake_request_context().await;
        let pages = vec![ExtractPage {
            url: "https://example.com".to_owned(),
        }];

        let result = extract_batch(Arc::new(mock), &ctx, 10.0, pages).await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert!(
            matches!(&results[0], ExtractUrlResult::Ok { url, markdown } if url == "https://example.com" && markdown.is_none())
        );
    }
}
