use crate::Meta;
use serde::{Deserialize, Serialize};

/// An extract response from the Kagi Extract API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractResponse {
    /// This object is subject to change as it is provided for debugging purposes. Fields may be added or removed as required. Don't build any hard dependancies on any of this data.
    pub meta: Meta,
    /// Array of extracted page content
    pub data: Option<Vec<ExtractData>>,
    /// Optional array of errors that occurred during extraction
    pub errors: Option<Vec<ExtractError>>,
}

/// Extracted content for a single page.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractData {
    /// The URL of the extracted page
    pub url: String,
    /// Extracted markdown content of the page. Will be absent if extraction fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    /// Per-URL error message returned when extraction fails for a specific page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// An error that occurred during extraction for a specific page.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractError {
    /// URL to error documentation
    pub url: String,
    /// Namespaced error code
    pub code: String,
    /// Human-readable error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
