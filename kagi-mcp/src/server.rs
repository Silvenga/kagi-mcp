use crate::cache::store::CacheStore;
use crate::tools::extract::{extract_handler, ExtractParams};
use crate::tools::search::{search_handler, SearchConfig, SearchParams};
use kagi_api::KagiApi;
use kagi_api::KagiClient;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError};
use std::sync::Arc;

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
    #[expect(
        clippy::too_many_arguments,
        reason = "constructor naturally needs many config values"
    )]
    pub fn new(
        client: KagiClient,
        search_timeout: f64,
        extract_timeout: f64,
        limit: u32,
        safe_search: bool,
        region: Option<String>,
        split_extract_requests: bool,
        cache_store: Option<Arc<CacheStore>>,
    ) -> Self {
        Self {
            client: Arc::new(client),
            search_timeout,
            extract_timeout,
            limit,
            safe_search,
            region,
            split_extract_requests,
            cache_store,
        }
    }

    #[cfg(test)]
    pub fn with_client(client: Arc<dyn KagiApi>, cache_store: Option<Arc<CacheStore>>) -> Self {
        Self {
            client,
            search_timeout: 4.0,
            extract_timeout: 30.0,
            limit: 10,
            safe_search: true,
            region: None,
            split_extract_requests: true,
            cache_store,
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
impl rmcp::ServerHandler for KagiMcpServer {}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::KagiClientBuilder;
    use rmcp::ServerHandler;

    #[test]
    fn when_server_created_then_tools_should_be_registered() {
        let client = KagiClientBuilder::new()
            .with_api_key("test-key")
            .build()
            .unwrap();

        let server = KagiMcpServer::new(client, 4.0, 30.0, 10, true, None, true, None);

        let info = server.get_info();
        assert!(
            info.capabilities.tools.is_some(),
            "server should have tools capability"
        );
    }

    #[test]
    fn when_mock_client_provided_then_server_should_accept_it() {
        let mock = kagi_api::MockKagiApi::new();
        let server = KagiMcpServer::with_client(Arc::new(mock), None);

        let info = server.get_info();
        assert!(
            info.capabilities.tools.is_some(),
            "server should have tools capability"
        );
    }

    #[tokio::test]
    async fn when_server_created_with_cache_store_then_it_should_compile() {
        let store = CacheStore::open_in_memory()
            .await
            .expect("failed to create in-memory cache store");

        let client = KagiClientBuilder::new()
            .with_api_key("test-key")
            .build()
            .unwrap();

        let server = KagiMcpServer::new(
            client,
            4.0,
            30.0,
            10,
            true,
            None,
            true,
            Some(Arc::new(store)),
        );

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
