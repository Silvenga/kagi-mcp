use crate::{KagiClient, KagiError};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://kagi.com/api";
const DEFAULT_USER_AGENT: &str = concat!(
    "kagi-api/",
    env!("CARGO_PKG_VERSION"),
    " (github.com/Silvenga/kagi-mcp)"
);
const DEFAULT_TIMEOUT_SECS: f64 = 32.0;
const DEFAULT_RETRIES: u32 = 3;

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
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            api_key: None,
            base_url: DEFAULT_BASE_URL.to_owned(),
            user_agent: DEFAULT_USER_AGENT.to_owned(),
            timeout: Duration::from_secs_f64(DEFAULT_TIMEOUT_SECS),
            retries: DEFAULT_RETRIES,
        }
    }

    /// Set the Kagi API key.
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the base URL for the Kagi API.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set a custom User-Agent header.
    pub fn with_user_agent(mut self, agent: impl Into<String>) -> Self {
        self.user_agent = agent.into();
        self
    }

    /// Set the request timeout in seconds.
    pub fn with_timeout_seconds(mut self, seconds: f64) -> Self {
        self.timeout = Duration::from_secs_f64(seconds);
        self
    }

    /// Set the number of retries for transient failures.
    pub fn with_retries(mut self, count: u32) -> Self {
        self.retries = count;
        self
    }

    /// Build and return a configured [`KagiClient`].
    pub fn build(self) -> Result<KagiClient, KagiError> {
        let api_key = self.api_key.ok_or_else(|| KagiError::Api {
            status: 0,
            message: "API key is required".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn when_default_builder_then_all_defaults_should_apply() {
        let builder = KagiClientBuilder::new();

        assert_eq!(builder.base_url, DEFAULT_BASE_URL);
        assert_eq!(builder.user_agent, DEFAULT_USER_AGENT);
        assert_eq!(
            builder.timeout,
            Duration::from_secs_f64(DEFAULT_TIMEOUT_SECS)
        );
        assert_eq!(builder.retries, DEFAULT_RETRIES);
        assert!(builder.api_key.is_none());
    }

    #[test]
    fn when_custom_base_url_then_should_override_default() {
        let builder = KagiClientBuilder::new().with_base_url("https://custom.example.com/api");

        assert_eq!(builder.base_url, "https://custom.example.com/api");
    }

    #[test]
    fn when_custom_user_agent_then_should_override_default() {
        let builder = KagiClientBuilder::new().with_user_agent("my-app/1.0");

        assert_eq!(builder.user_agent, "my-app/1.0");
    }

    #[test]
    fn when_custom_timeout_then_should_override_default() {
        let builder = KagiClientBuilder::new().with_timeout_seconds(5.0);

        assert_eq!(builder.timeout, Duration::from_secs_f64(5.0));
    }

    #[test]
    fn when_custom_retries_then_should_override_default() {
        let builder = KagiClientBuilder::new().with_retries(5);

        assert_eq!(builder.retries, 5);
    }

    #[test]
    fn when_api_key_set_then_should_be_some() {
        let builder = KagiClientBuilder::new().with_api_key("test-key");

        assert_eq!(builder.api_key, Some("test-key".to_owned()));
    }
}
