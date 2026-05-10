use std::time::Duration;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::{RetryError, RetryTransientMiddleware};
use crate::error::{from_http_status, KagiError, KagiErrorResponse};
use crate::types::{ExtractRequest, ExtractResponse, SearchRequest, SearchResponse};
use crate::KagiApi;

const DEFAULT_BASE_URL: &str = "https://kagi.com/api";
const DEFAULT_USER_AGENT: &str = concat!("kagi-api/", env!("CARGO_PKG_VERSION"), " (github.com/Silvenga/kagi-mcp)");
const DEFAULT_TIMEOUT_SECS: f64 = 10.0;
const DEFAULT_RETRIES: u32 = 3;

/// Client for the Kagi Search and Extract APIs.
#[derive(Debug, Clone)]
pub struct KagiClient {
    client: ClientWithMiddleware,
    base_url: String,
    api_key: String,
}

/// Builder for [`KagiClient`].
#[derive(Debug, Clone)]
pub struct KagiClientBuilder {
    api_key: Option<String>,
    base_url: String,
    user_agent: String,
    timeout: Duration,
    retries: u32,
}

impl KagiClientBuilder {
    pub fn new() -> Self {
        Self {
            api_key: None,
            base_url: DEFAULT_BASE_URL.to_string(),
            user_agent: DEFAULT_USER_AGENT.to_string(),
            timeout: Duration::from_secs_f64(DEFAULT_TIMEOUT_SECS),
            retries: DEFAULT_RETRIES,
        }
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn user_agent(mut self, agent: impl Into<String>) -> Self {
        self.user_agent = agent.into();
        self
    }

    pub fn timeout(mut self, seconds: f64) -> Self {
        self.timeout = Duration::from_secs_f64(seconds);
        self
    }

    pub fn retries(mut self, count: u32) -> Self {
        self.retries = count;
        self
    }

    pub fn build(self) -> Result<KagiClient, KagiError> {
        let api_key = self.api_key.ok_or_else(|| KagiError::Api {
            status: 0,
            message: "API key is required".to_string(),
        })?;

        let reqwest_client = reqwest::Client::builder()
            .timeout(self.timeout)
            .user_agent(self.user_agent)
            .build()
            .map_err(|e| KagiError::Network { source: e })?;

        let retry_policy = ExponentialBackoff::builder()
            .retry_bounds(Duration::from_secs(1), Duration::from_secs(30))
            .build_with_max_retries(self.retries);

        let client = ClientBuilder::new(reqwest_client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        Ok(KagiClient {
            client,
            base_url: self.base_url,
            api_key,
        })
    }
}

impl Default for KagiClientBuilder {
    fn default() -> Self {
        Self::new()
    }
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

async fn handle_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, KagiError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn when_default_builder_then_should_have_default_base_url() {
        let builder = KagiClientBuilder::new();

        assert_eq!(builder.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn when_default_builder_then_should_have_default_user_agent() {
        let builder = KagiClientBuilder::new();

        assert_eq!(builder.user_agent, DEFAULT_USER_AGENT);
    }

    #[test]
    fn when_default_builder_then_should_have_default_timeout() {
        let builder = KagiClientBuilder::new();

        assert_eq!(
            builder.timeout,
            Duration::from_secs_f64(DEFAULT_TIMEOUT_SECS)
        );
    }

    #[test]
    fn when_default_builder_then_should_have_default_retries() {
        let builder = KagiClientBuilder::new();

        assert_eq!(builder.retries, DEFAULT_RETRIES);
    }

    #[test]
    fn when_default_builder_then_should_have_no_api_key() {
        let builder = KagiClientBuilder::new();

        assert!(builder.api_key.is_none());
    }

    #[test]
    fn when_custom_base_url_then_should_override_default() {
        let builder = KagiClientBuilder::new().base_url("https://custom.example.com/api");

        assert_eq!(builder.base_url, "https://custom.example.com/api");
    }

    #[test]
    fn when_custom_user_agent_then_should_override_default() {
        let builder = KagiClientBuilder::new().user_agent("my-app/1.0");

        assert_eq!(builder.user_agent, "my-app/1.0");
    }

    #[test]
    fn when_custom_timeout_then_should_override_default() {
        let builder = KagiClientBuilder::new().timeout(5.0);

        assert_eq!(builder.timeout, Duration::from_secs_f64(5.0));
    }

    #[test]
    fn when_custom_retries_then_should_override_default() {
        let builder = KagiClientBuilder::new().retries(5);

        assert_eq!(builder.retries, 5);
    }

    #[test]
    fn when_api_key_set_then_should_be_some() {
        let builder = KagiClientBuilder::new().api_key("test-key");

        assert_eq!(builder.api_key, Some("test-key".to_string()));
    }
}
