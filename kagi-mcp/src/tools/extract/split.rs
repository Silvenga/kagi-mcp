use crate::tools::extract::errors::kagi_error_to_extract_error;
use crate::tools::extract::pipeline::{ExtractFatalError, ExtractUrlResult};
use kagi_api::{ExtractPage, ExtractRequest, KagiApi};
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use std::sync::Arc;
use tokio::task::JoinSet;

pub async fn extract_split(
    client: Arc<dyn KagiApi>,
    ctx: &RequestContext<RoleServer>,
    extract_timeout: f64,
    pages: Vec<ExtractPage>,
) -> Result<Vec<ExtractUrlResult>, ExtractFatalError> {
    let total_pages = pages.len();

    tracing::info!(total_pages = total_pages, "extract split started");

    if ctx.ct.is_cancelled() {
        return Err(ExtractFatalError::Cancelled);
    }

    let mut set = JoinSet::new();
    for page in &pages {
        if ctx.ct.is_cancelled() {
            return Err(ExtractFatalError::Cancelled);
        }
        let client = Arc::clone(&client);
        let page = page.clone();
        let single_req = ExtractRequest::new(vec![page])
            .with_format("json")
            .with_timeout_seconds(extract_timeout);

        set.spawn(async move {
            let result = client.extract(single_req).await;
            result
        });
    }

    let mut results = Vec::with_capacity(total_pages);
    let mut idx = 0usize;
    while let Some(join_result) = set.join_next().await {
        if ctx.ct.is_cancelled() {
            set.abort_all();
            return Err(ExtractFatalError::Cancelled);
        }

        let page_url = &pages[idx].url;

        match join_result {
            Ok(Ok(api_response)) => {
                let mut found = false;
                if let Some(data_vec) = &api_response.data {
                    if let Some(extracted_data) = data_vec
                        .iter()
                        .find(|d| d.url.trim_end_matches('/') == page_url.trim_end_matches('/'))
                    {
                        results.push(ExtractUrlResult::Ok {
                            url: page_url.clone(),
                            markdown: extracted_data.markdown.clone(),
                        });
                        found = true;
                    }
                }
                if !found {
                    // If we can't correlate, treat as error
                    results.push(ExtractUrlResult::Err {
                        url: page_url.clone(),
                        error: kagi_api::ExtractError {
                            url: page_url.clone(),
                            code: "missing_data".to_owned(),
                            message: Some("Response did not contain data for this URL".to_owned()),
                        },
                    });
                }
                tracing::info!(url = %page_url, "page extracted");
            }
            Ok(Err(kagi_err)) => {
                let extract_err = kagi_error_to_extract_error(page_url, &kagi_err);
                results.push(ExtractUrlResult::Err {
                    url: page_url.clone(),
                    error: extract_err,
                });
                match &kagi_err {
                    kagi_api::KagiError::Unauthorized
                    | kagi_api::KagiError::InvalidRequest { .. } => {
                        tracing::error!(url = %page_url, error = %kagi_err, "page extraction failed");
                    }
                    _ => {
                        tracing::warn!(url = %page_url, error = %kagi_err, "page extraction failed");
                    }
                }
            }
            Err(_join_err) => {
                return Err(ExtractFatalError::Cancelled);
            }
        }
        idx += 1;
    }

    tracing::info!(
        success_count = results
            .iter()
            .filter(|r| matches!(r, ExtractUrlResult::Ok { .. }))
            .count(),
        error_count = results
            .iter()
            .filter(|r| matches!(r, ExtractUrlResult::Err { .. }))
            .count(),
        "extract split completed"
    );

    Ok(results)
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
    async fn when_single_url_succeeds_then_extract_split_returns_ok() {
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
                }]),
                errors: None,
            })
        });

        let ctx = fake_request_context().await;
        let pages = vec![ExtractPage {
            url: "https://example.com".to_owned(),
        }];

        let result = extract_split(Arc::new(mock), &ctx, 10.0, pages).await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert!(
            matches!(&results[0], ExtractUrlResult::Ok { url, markdown } if url == "https://example.com" && markdown == &Some("content".to_owned()))
        );
    }

    #[tokio::test]
    async fn when_single_url_fails_then_extract_split_returns_err() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract()
            .times(1)
            .returning(|_| Err(kagi_api::KagiError::ServerError));

        let ctx = fake_request_context().await;
        let pages = vec![ExtractPage {
            url: "https://example.com".to_owned(),
        }];

        let result = extract_split(Arc::new(mock), &ctx, 10.0, pages).await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert!(
            matches!(&results[0], ExtractUrlResult::Err { url, error } if url == "https://example.com" && error.code == "server_error")
        );
    }

    #[tokio::test]
    async fn when_mixed_results_then_extract_split_returns_both() {
        let mut mock = MockKagiApi::new();
        mock.expect_extract().times(2).returning(|req| {
            let url = &req.pages()[0].url;
            if url == "https://fail.com" {
                Err(kagi_api::KagiError::ServerError)
            } else {
                Ok(ExtractResponse {
                    meta: Meta {
                        trace: "test".to_owned(),
                        node: None,
                        ms: None,
                    },
                    data: Some(vec![ExtractData {
                        url: url.clone(),
                        markdown: Some("content".to_owned()),
                    }]),
                    errors: None,
                })
            }
        });

        let ctx = fake_request_context().await;
        let pages = vec![
            ExtractPage {
                url: "https://ok.com".to_owned(),
            },
            ExtractPage {
                url: "https://fail.com".to_owned(),
            },
        ];

        let result = extract_split(Arc::new(mock), &ctx, 10.0, pages).await;

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 2);
        assert!(matches!(&results[0], ExtractUrlResult::Ok { url, .. } if url == "https://ok.com"));
        assert!(
            matches!(&results[1], ExtractUrlResult::Err { url, .. } if url == "https://fail.com")
        );
    }
}
