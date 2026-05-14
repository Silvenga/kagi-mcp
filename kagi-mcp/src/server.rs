use crate::tools::extract::{extract_handler, ExtractParams};
use crate::tools::search::{search_handler, SearchConfig, SearchParams};
use kagi_api::client::KagiClient;
use kagi_api::KagiApi;
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
    pub overfetch_multiplier: u32,
    pub overfetch_max: u32,
}

impl KagiMcpServer {
    // Allow >7 args because each timeout is a distinct API parameter that must be
    // configurable independently; bundling them would hurt readability.
    #[expect(
        clippy::too_many_arguments,
        reason = "each timeout is a distinct API parameter"
    )]
    pub fn new(
        client: KagiClient,
        search_timeout: f64,
        extract_timeout: f64,
        limit: u32,
        safe_search: bool,
        region: Option<String>,
        overfetch_multiplier: u32,
        overfetch_max: u32,
    ) -> Self {
        Self {
            client: Arc::new(client),
            search_timeout,
            extract_timeout,
            limit,
            safe_search,
            region,
            overfetch_multiplier,
            overfetch_max,
        }
    }

    #[cfg(test)]
    pub fn with_client(client: Arc<dyn KagiApi>) -> Self {
        Self {
            client,
            search_timeout: 4.0,
            extract_timeout: 30.0,
            limit: 10,
            safe_search: true,
            region: None,
            overfetch_multiplier: 5,
            overfetch_max: 50,
        }
    }
}

#[tool_router(vis = "pub")]
impl KagiMcpServer {
    #[tool(
        description = "Search the web via Kagi's premium search engine. Returns markdown-formatted results with sections for Web Results, News, Videos, Podcasts, Images, and related entities. Supports Kagi query operators: `site:domain`, `\"exact phrase\"`, `-negation`, `inurl:keyword`. Use `workflow` to scope results (search, images, videos, news, podcasts). Use `after` / `before` (YYYY-MM-DD) to filter by date. Set `limit_per_domain` to deduplicate same-domain results (e.g. 1 = one result per domain). Set `output_format=\"json\"` for raw structured response."
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
            overfetch_multiplier: self.overfetch_multiplier,
            overfetch_max: self.overfetch_max,
        };
        search_handler(&*self.client, params, &ctx, &config).await
    }

    #[tool(description = "Extract clean Markdown from URLs")]
    async fn extract(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<ExtractParams>,
    ) -> Result<CallToolResult, McpError> {
        extract_handler(&*self.client, params, &ctx, self.extract_timeout).await
    }
}

#[tool_handler(name = "Kagi", router = Self::tool_router())]
impl rmcp::ServerHandler for KagiMcpServer {}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::client::KagiClientBuilder;
    use rmcp::ServerHandler;

    #[test]
    fn when_server_created_then_tools_should_be_registered() {
        let client = KagiClientBuilder::new()
            .api_key("test-key")
            .build()
            .unwrap();

        let server = KagiMcpServer::new(client, 4.0, 30.0, 10, true, None, 5, 50);

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
}
