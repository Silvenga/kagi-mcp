use crate::error::{from_http_status, KagiError, KagiErrorResponse};
use crate::types::{ExtractRequest, ExtractResponse, SearchRequest, SearchResponse};
use crate::KagiApi;
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::RetryError;
use serde::de::DeserializeOwned;

/// Client for the Kagi Search and Extract APIs.
#[derive(Debug, Clone)]
pub struct KagiClient {
    pub(crate) client: ClientWithMiddleware,
    pub(crate) base_url: String,
    pub(crate) api_key: String,
}

#[async_trait::async_trait]
impl KagiApi for KagiClient {
    async fn search(&self, request: SearchRequest) -> Result<SearchResponse, KagiError> {
        KagiClient::search(self, request).await
    }

    async fn extract(&self, request: ExtractRequest) -> Result<ExtractResponse, KagiError> {
        KagiClient::extract(self, request).await
    }
}

impl KagiClient {
    /// Perform a search via the Kagi Search API.
    #[tracing::instrument(skip(self, request), fields(endpoint = "search"))]
    pub async fn search(&self, request: SearchRequest) -> Result<SearchResponse, KagiError> {
        let url = format!("{}/v1/search", self.base_url);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(map_middleware_error)?;

        handle_response(response).await
    }

    /// Extract markdown content from URLs via the Kagi Extract API.
    #[tracing::instrument(skip(self, request), fields(endpoint = "extract"))]
    pub async fn extract(&self, request: ExtractRequest) -> Result<ExtractResponse, KagiError> {
        let url = format!("{}/v1/extract", self.base_url);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(map_middleware_error)?;

        handle_response(response).await
    }
}

async fn handle_response<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, KagiError> {
    let status = response.status();

    if status.is_success() {
        let body: T = response
            .json()
            .await
            .map_err(|e| KagiError::Network { source: e })?;
        Ok(body)
    } else {
        let error_body = response.json::<KagiErrorResponse>().await.ok();
        Err(from_http_status(status, error_body))
    }
}

fn map_middleware_error(error: reqwest_middleware::Error) -> KagiError {
    match error {
        reqwest_middleware::Error::Reqwest(e) => KagiError::Network { source: e },
        reqwest_middleware::Error::Middleware(e) => {
            let message = e.to_string();
            // reqwest-retry wraps transport errors in RetryError (via anyhow).
            // Try to extract the underlying reqwest::Error via downcast.
            if let Ok(retry_err) = e.downcast::<RetryError>() {
                let inner_err = match retry_err {
                    RetryError::WithRetries { err, .. } => err,
                    RetryError::Error(err) => err,
                };
                if let reqwest_middleware::Error::Reqwest(reqwest_err) = inner_err {
                    return KagiError::Network {
                        source: reqwest_err,
                    };
                }
            }
            KagiError::Api { status: 0, message }
        }
    }
}

