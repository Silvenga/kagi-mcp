//! MCP server crate for the [Kagi Search API](https://kagi.com/api).
//!
//! This is primarily a binary crate — the entrypoint is `main.rs`.
//! The `kagi-mcp` binary embeds the `kagi-api` client crate and exposes
//! search and extract tools via the Model Context Protocol (MCP).

pub mod cache;
pub mod subscriber;
pub mod tools;

mod config;
mod format;
mod server;

pub use config::*;
pub use format::*;
pub use server::*;
