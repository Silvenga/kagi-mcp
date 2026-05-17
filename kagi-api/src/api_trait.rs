use crate::{ExtractRequest, ExtractResponse, KagiError, SearchRequest, SearchResponse};

/// Async trait abstracting the Kagi Search and Extract APIs.
#[async_trait::async_trait]
pub trait KagiApi: Send + Sync {
    /// Execute a search request.
    async fn search(&self, request: SearchRequest) -> Result<SearchResponse, KagiError>;

    /// Execute an extract request.
    async fn extract(&self, request: ExtractRequest) -> Result<ExtractResponse, KagiError>;
}

#[cfg(any(test, feature = "mock"))]
mockall::mock! {
    pub KagiApi {}

    #[async_trait::async_trait]
    impl KagiApi for KagiApi {
        async fn search(
            &self,
            request: SearchRequest,
        ) -> Result<SearchResponse, KagiError>;

        async fn extract(
            &self,
            request: ExtractRequest,
        ) -> Result<ExtractResponse, KagiError>;
    }
}
