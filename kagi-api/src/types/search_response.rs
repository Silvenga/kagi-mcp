use serde::{Deserialize, Serialize};

/// A Kagi search response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResponse {
    /// This object is subject to change as it is provided for debugging purposes. Fields may be added or removed as required. Don't build any hard dependancies on any of this data.
    pub meta: Meta,
    /// Holds all the search results for the query specified. As results have different types and sources, they are separated into different fields within this object. For example image results are stored under `data.image` while news results are stored under `data.news`. All results will have the same main structure, but additional data might be stored under the `data.{name}[].props` path. Please look at each different type of result for more information about this.
    pub data: SearchData,
}

/// The data payload of a search response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchData {
    /// Contains all search results for html pages or websites.
    pub search: Option<Vec<SearchResult>>,
    /// Contains all search results for images.
    pub image: Option<Vec<SearchResult>>,
    /// Contains all search results for videos.
    pub video: Option<Vec<SearchResult>>,
    /// Contains all search results for podcasts.
    pub podcast: Option<Vec<SearchResult>>,
    /// Contains all search results for creators of podcasts.
    pub podcast_creator: Option<Vec<SearchResult>>,
    /// Contains all search results for news articles.
    pub news: Option<Vec<SearchResult>>,
    /// Contains results that are obtained by searching for slightly different queries. These questions are stored under the `props.question` path.
    pub adjacent_question: Option<Vec<SearchResult>>,
    /// If the search query was a math equation, or unit conversions, things that can be answered quickly, the result will be in here.
    pub direct_answer: Option<Vec<SearchResult>>,
    /// Contains news results from publishers collected and stored in Kagi's news index.
    pub interesting_news: Option<Vec<SearchResult>>,
    /// Contains small web results from publishers collected and stored in Kagi's small web index.
    pub interesting_finds: Option<Vec<SearchResult>>,
    /// Contains detailed summary and tabulated information about a person, place, or thing.
    pub infobox: Option<Vec<SearchResult>>,
    /// Contains all search results that link to code resources or repositories.
    pub code: Option<Vec<SearchResult>>,
    /// If the search query was a package tracking number, the correct package tracking website should be present in this collection.
    pub package_tracking: Option<Vec<SearchResult>>,
    /// Contains all search results for public records, such as government documents or public court records.
    pub public_records: Option<Vec<SearchResult>>,
    /// Contains all search results for the current weather.
    pub weather: Option<Vec<SearchResult>>,
    /// Contains a list of searches that are related to the current search, and may help narrow down the results.
    pub related_search: Option<Vec<SearchResult>>,
    /// Contains all search results that are lists of things. Results with titles like "5 things you didn't know about...", or "10 of the best headphones".
    pub listicle: Option<Vec<SearchResult>>,
    /// Contains all search results for archived websites that may not be available anymore
    pub web_archive: Option<Vec<SearchResult>>,
}

/// A single search result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResult {
    /// The location of the result. This is the direct URL to the resource that matches the query
    pub url: String,
    /// This is the title of the resource. For HTML documents, it reflects `<title>`. For videos, it is the name that would be displayed on the video site. Can be `None` for image results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A short summary of the contents of the resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    /// The date when the resource was created or last updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// An image that goes with the main resource. It could be a video's thumbnail or an HTML document's main image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    /// Holds arbitrary result metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<serde_json::Value>,
}

/// An image associated with a search result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Image {
    /// This is a URL that links directly to the image that goes with the resources.
    pub url: String,
    /// The height of the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// The width of the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
}

/// Metadata returned with every API response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Meta {
    /// Trace ID that can be used to debug individual API requests. Provide this, if needed, when contacting Kagi support.
    pub trace: String,
    /// The hostname of the node that fulfilled the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    /// how long the request took to fulfill, excluding round trip to the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ms: Option<u64>,
}
