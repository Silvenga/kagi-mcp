use serde::Deserialize;

/// Error detail as returned by the Kagi API.
#[derive(Debug, Deserialize)]
pub(crate) struct ErrorDetail {
    #[expect(dead_code, reason = "field present in API response but not consumed")]
    pub(crate) code: String,
    #[expect(dead_code, reason = "field present in API response but not consumed")]
    pub(crate) url: String,
    pub(crate) message: Option<String>,
    #[expect(dead_code, reason = "field present in API response but not consumed")]
    pub(crate) location: Option<String>,
}

/// The Kagi API error response envelope.
///
/// Returned when a request fails. The `error` array contains one or more
/// error detail objects describing what went wrong.
#[derive(Debug, Deserialize)]
pub(crate) struct KagiErrorResponse {
    #[expect(dead_code, reason = "field present in API response but not consumed")]
    pub(crate) meta: serde_json::Value,
    #[expect(dead_code, reason = "field present in API response but not consumed")]
    pub(crate) data: Option<serde_json::Value>,
    pub(crate) error: Vec<ErrorDetail>,
}

impl KagiErrorResponse {
    /// Formats the error details into a human-readable message.
    pub(crate) fn format_message(&self) -> String {
        self.error
            .iter()
            .filter_map(|e| e.message.as_deref())
            .collect::<Vec<_>>()
            .join("; ")
    }
}
