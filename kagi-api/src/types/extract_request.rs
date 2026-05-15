use serde::Serialize;

/// An extract request to the Kagi Extract API.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractRequest {
    /// Array of pages to extract content from. Must contain 1-10 URLs.
    /// Each URL must be a valid HTTPS URL.
    pages: Vec<ExtractPage>,
    /// **(EXPERIMENTAL)** Format to serialize the API response as. The
    /// exact contents and structure of markdown output is still being
    /// worked on - please send your feedback!
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    /// Optional timeout in seconds for the extraction operation
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "timeout")]
    timeout_seconds: Option<f64>,
}

impl ExtractRequest {
    pub fn new(pages: Vec<ExtractPage>) -> Self {
        Self {
            pages,
            format: None,
            timeout_seconds: None,
        }
    }

    pub fn pages(&self) -> &[ExtractPage] {
        &self.pages
    }

    pub fn format(&self) -> Option<&str> {
        self.format.as_deref()
    }

    pub fn timeout_seconds(&self) -> Option<f64> {
        self.timeout_seconds
    }

    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    pub fn with_timeout_seconds(mut self, timeout: f64) -> Self {
        self.timeout_seconds = Some(timeout);
        self
    }
}

/// A single page to extract content from.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractPage {
    /// The HTTPS URL of the page to extract content from. Must use HTTPS scheme with a valid host.
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_minimal_extract_request_then_should_serialize_to_pages_only() {
        let request = ExtractRequest::new(vec![ExtractPage {
            url: "https://example.com".to_owned(),
        }]);
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"pages":[{"url":"https://example.com"}]}"#);
    }

    #[test]
    fn when_with_timeout_seconds_then_should_use_timeout_key_in_json() {
        let request = ExtractRequest::new(vec![ExtractPage {
            url: "https://example.com".to_owned(),
        }]).with_timeout_seconds(30.0);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"timeout\":30.0"));
        assert!(!json.contains("\"timeout_seconds\""));
    }
}
