use crate::error;
use crate::types;

/// Async trait abstracting the Kagi Search and Extract APIs.
#[async_trait::async_trait]
pub trait KagiApi: Send + Sync {
    async fn search(
        &self,
        request: types::SearchRequest,
    ) -> Result<types::SearchResponse, error::KagiError>;

    async fn extract(
        &self,
        request: types::ExtractRequest,
    ) -> Result<types::ExtractResponse, error::KagiError>;
}

#[cfg(any(test, feature = "mock"))]
mockall::mock! {
    pub KagiApi {}

    #[async_trait::async_trait]
    impl KagiApi for KagiApi {
        async fn search(
            &self,
            request: types::SearchRequest,
        ) -> Result<types::SearchResponse, error::KagiError>;

        async fn extract(
            &self,
            request: types::ExtractRequest,
        ) -> Result<types::ExtractResponse, error::KagiError>;
    }
}
