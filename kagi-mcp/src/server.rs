use crate::cache::CacheStore;
use crate::tools::{extract_handler, ExtractParams};
use crate::tools::{search_handler, SearchConfig, SearchParams};
use kagi_api::{KagiApi, KagiClient};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError};
use rmcp::{RoleServer, ServerHandler};
use std::sync::Arc;

const DEFAULT_SEARCH_TIMEOUT: f64 = 4.0;
const DEFAULT_EXTRACT_TIMEOUT: f64 = 10.0;
const DEFAULT_LIMIT: u32 = 10;
const DEFAULT_SAFE_SEARCH: bool = true;
const DEFAULT_SPLIT_EXTRACT_REQUESTS: bool = true;

#[derive(Clone)]
pub struct KagiMcpServer {
    pub client: Arc<dyn KagiApi>,
    pub search_timeout: f64,
    pub extract_timeout: f64,
    pub limit: u32,
    pub safe_search: bool,
    pub region: Option<String>,
    pub split_extract_requests: bool,
    pub cache_store: Option<Arc<CacheStore>>,
}

impl KagiMcpServer {
    /// Create a new server with the given client and default settings.
    pub fn new(client: KagiClient) -> Self {
        Self {
            client: Arc::new(client),
            search_timeout: DEFAULT_SEARCH_TIMEOUT,
            extract_timeout: DEFAULT_EXTRACT_TIMEOUT,
            limit: DEFAULT_LIMIT,
            safe_search: DEFAULT_SAFE_SEARCH,
            region: None,
            split_extract_requests: DEFAULT_SPLIT_EXTRACT_REQUESTS,
            cache_store: None,
        }
    }

    /// Set the search timeout in seconds.
    pub fn with_search_timeout(mut self, timeout: f64) -> Self {
        self.search_timeout = timeout;
        self
    }

    /// Set the extract timeout in seconds.
    pub fn with_extract_timeout(mut self, timeout: f64) -> Self {
        self.extract_timeout = timeout;
        self
    }

    /// Set the default result limit for search.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    /// Enable or disable safe search.
    pub fn with_safe_search(mut self, safe_search: bool) -> Self {
        self.safe_search = safe_search;
        self
    }

    /// Set the default region filter.
    pub fn with_region(mut self, region: Option<String>) -> Self {
        self.region = region;
        self
    }

    /// Enable or disable splitting extract requests per URL.
    pub fn with_split_extract_requests(mut self, split: bool) -> Self {
        self.split_extract_requests = split;
        self
    }

    /// Set the cache store.
    pub fn with_cache_store(mut self, cache_store: Option<Arc<CacheStore>>) -> Self {
        self.cache_store = cache_store;
        self
    }

    #[cfg(test)]
    pub fn with_client(client: Arc<dyn KagiApi>) -> Self {
        Self {
            client,
            search_timeout: DEFAULT_SEARCH_TIMEOUT,
            extract_timeout: 30.0,
            limit: DEFAULT_LIMIT,
            safe_search: DEFAULT_SAFE_SEARCH,
            region: None,
            split_extract_requests: DEFAULT_SPLIT_EXTRACT_REQUESTS,
            cache_store: None,
        }
    }
}

#[tool_router(vis = "pub")]
impl KagiMcpServer {
    #[tool(
        description = "Search the web via Kagi. Returns results in markdown by default. Use when you need current information from the web, news, images, videos, or podcasts."
    )]
    async fn search(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = SearchConfig {
            search_timeout: self.search_timeout,
            limit: self.limit,
            safe_search: self.safe_search,
            region: self.region.clone(),
        };
        search_handler(
            &*self.client,
            params,
            &ctx,
            &config,
            self.cache_store.as_deref(),
        )
        .await
    }

    #[tool(description = "Extract clean Markdown from URLs")]
    async fn extract(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<ExtractParams>,
    ) -> Result<CallToolResult, McpError> {
        extract_handler(
            self.client.clone(),
            params,
            &ctx,
            self.extract_timeout,
            self.split_extract_requests,
            self.cache_store.as_deref(),
        )
        .await
    }
}

#[tool_handler(name = "Kagi", router = Self::tool_router())]
impl ServerHandler for KagiMcpServer {}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::KagiClientBuilder;

    #[test]
    fn when_server_created_then_tools_should_be_registered() {
        let client = KagiClientBuilder::new()
            .with_api_key("test-key")
            .build()
            .unwrap();

        let server = KagiMcpServer::new(client)
            .with_search_timeout(4.0)
            .with_extract_timeout(30.0);

        let info = server.get_info();
        assert!(
            info.capabilities.tools.is_some(),
            "server should have tools capability"
        );
    }

    #[test]
    fn when_mock_client_provided_then_server_should_accept_it() {
        let mock = kagi_api::MockKagiApi::new();
        let server = KagiMcpServer::with_client(Arc::new(mock));

        let info = server.get_info();
        assert!(
            info.capabilities.tools.is_some(),
            "server should have tools capability"
        );
    }

    #[tokio::test]
    async fn when_server_created_with_cache_store_then_tools_and_cache_should_be_present() {
        let store = CacheStore::open_in_memory()
            .await
            .expect("failed to create in-memory cache store");

        let client = KagiClientBuilder::new()
            .with_api_key("test-key")
            .build()
            .unwrap();

        let server = KagiMcpServer::new(client)
            .with_search_timeout(4.0)
            .with_extract_timeout(30.0)
            .with_cache_store(Some(Arc::new(store)));

        let info = server.get_info();
        assert!(
            info.capabilities.tools.is_some(),
            "server with cache store should have tools capability"
        );
        assert!(
            server.cache_store.is_some(),
            "cache_store should be present"
        );
    }
}
