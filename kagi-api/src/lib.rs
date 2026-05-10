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
//! use kagi_api::{KagiClient, SearchRequest};
//!
//! let client = KagiClient::builder()
//!     .api_key("my-api-key")?
//!     .build()?;
//!
//! let results = client.search(SearchRequest::new("rust programming")).await?;
//! println!("{}", results.meta);
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
