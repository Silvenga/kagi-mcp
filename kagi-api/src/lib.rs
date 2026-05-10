//! Rust client for the [Kagi Search API](https://kagi.com/api).
//!
//! Provides [`KagiClient`] and [`KagiClientBuilder`] for making
//! search and extract requests, and the [`KagiApi`] async trait
//! for testability / mocking.
//!
//! # Quick start
//!
//! ```no_run
//! # async fn example() -> Result<(), kagi_api::KagiError> {
//! use kagi_api::{KagiClientBuilder, SearchRequest};
//!
//! let client = KagiClientBuilder::new()
//!     .api_key("my-api-key")
//!     .build()?;
//!
//! let request = SearchRequest {
//!     query: "rust programming".to_string(),
//!     workflow: None,
//!     format: None,
//!     timeout: None,
//!     page: None,
//!     limit: None,
//!     safe_search: None,
//!     region: None,
//!     filters: None,
//! };
//! let results = client.search(request).await?;
//! println!("{}", results.meta.trace);
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod error;
pub mod types;

pub use client::{KagiClient, KagiClientBuilder};
pub use error::{KagiError, KagiErrorResponse};
pub use types::{
    ExtractData, ExtractError, ExtractPage, ExtractRequest, ExtractResponse, Filters, Image, Meta,
    SearchData, SearchRequest, SearchResponse, SearchResult,
};

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
