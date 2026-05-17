use serde::Deserialize;

/// The Kagi API error response envelope.
///
/// Returned when a request fails. The `error` array contains one or more
/// error detail objects describing what went wrong.
#[derive(Debug, Deserialize)]
pub struct KagiErrorResponse {
    /// This object is subject to change as it is provided for debugging purposes.
    /// Fields may be added or removed as required.
    /// Don't build any hard dependencies on any of this data.
    pub meta: serde_json::Value,
    /// Empty data array when error occurs.
    pub data: Option<serde_json::Value>,
    /// Array of error detail objects describing what went wrong.
    pub error: Vec<ErrorDetail>,
}

impl KagiErrorResponse {
    /// Formats the error details into a human-readable message.
    pub fn format_message(&self) -> String {
        self.error
            .iter()
            .filter_map(|e| e.message.as_deref())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Error detail as returned by the Kagi API.
#[derive(Debug, Deserialize)]
pub struct ErrorDetail {
    /// Namespaced error code.
    /// Example: `extract.invalid_url`
    pub code: String,
    /// URL to error documentation.
    /// NOTE: These URLs are still a work in progress and will currently return a 404 error.
    /// Example: `https://help.kagi.com/api/errors#extract.invalid_url`
    pub url: String,
    /// Human-readable error message.
    ///  Example: `URL must be a valid HTTPS URL`
    pub message: Option<String>,
    /// Field location where the error occurred.
    /// Example: `pages[0].url`
    pub location: Option<String>,
}
