use serde::{Deserialize, Serialize};

/// A search request to the Kagi Search API.
#[derive(Debug, Clone, Serialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe_search: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Filters>,
}

/// Filters applied to search results.
#[derive(Debug, Clone, Serialize)]
pub struct Filters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

/// A Kagi search response.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    pub meta: Meta,
    pub data: SearchData,
}

/// The data payload of a search response.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchData {
    pub search: Option<Vec<SearchResult>>,
    pub image: Option<Vec<SearchResult>>,
    pub video: Option<Vec<SearchResult>>,
    pub podcast: Option<Vec<SearchResult>>,
    pub podcast_creator: Option<Vec<SearchResult>>,
    pub news: Option<Vec<SearchResult>>,
    pub adjacent_question: Option<Vec<SearchResult>>,
    pub direct_answer: Option<Vec<SearchResult>>,
    pub interesting_news: Option<Vec<SearchResult>>,
    pub interesting_finds: Option<Vec<SearchResult>>,
    pub infobox: Option<Vec<SearchResult>>,
    pub code: Option<Vec<SearchResult>>,
    pub package_tracking: Option<Vec<SearchResult>>,
    pub public_records: Option<Vec<SearchResult>>,
    pub weather: Option<Vec<SearchResult>>,
    pub related_search: Option<Vec<SearchResult>>,
    pub listicle: Option<Vec<SearchResult>>,
    pub web_archive: Option<Vec<SearchResult>>,
}

/// A single search result.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<serde_json::Value>,
}

/// An image associated with a search result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Image {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
}

/// Metadata returned with every API response.
#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    pub trace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ms: Option<u64>,
}

/// An extract request to the Kagi Extract API.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractRequest {
    pub pages: Vec<ExtractPage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// A single page to extract content from.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractPage {
    pub url: String,
}

/// An extract response from the Kagi Extract API.
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractResponse {
    pub meta: Meta,
    pub data: Option<Vec<ExtractData>>,
    pub errors: Option<Vec<ExtractError>>,
}

/// Extracted content for a single page.
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractData {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

/// An error that occurred during extraction for a specific page.
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractError {
    pub url: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
