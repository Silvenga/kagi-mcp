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
//!     .with_api_key("my-api-key")
//!     .build()?;
//!
//! let request = SearchRequest::new("rust programming");
//! let results = client.search(request).await?;
//! println!("{}", results.meta.trace);
//! # Ok(())
//! # }
//! ```

pub mod api_trait;
pub mod client;
pub mod client_builder;
pub mod error;
pub mod types;

pub use client::KagiClient;
pub use client_builder::KagiClientBuilder;
pub use error::{KagiError, KagiErrorResponse};
pub use types::{
    ExtractData, ExtractError, ExtractPage, ExtractRequest, ExtractResponse, Filters, Image, Meta,
    SearchData, SearchRequest, SearchResponse, SearchResult,
};

pub use api_trait::KagiApi;

#[cfg(any(test, feature = "mock"))]
pub use api_trait::MockKagiApi;


