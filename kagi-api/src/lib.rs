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

#![warn(missing_docs)]

mod api_trait;
mod builder;
mod client;
mod error;
mod types;

pub use builder::*;
pub use client::*;
pub use error::*;
pub use types::*;

pub use api_trait::KagiApi;
#[cfg(any(test, feature = "mock"))]
pub use api_trait::MockKagiApi;
