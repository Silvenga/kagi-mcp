mod error_response;
mod extract_request;
mod extract_response;
mod search_extract_config;
mod search_lens;
mod search_personalizations;
mod search_request;
mod search_response;

#[cfg(test)]
pub(crate) use error_response::ErrorDetail;
pub(crate) use error_response::KagiErrorResponse;
pub use extract_request::{ExtractPage, ExtractRequest};
pub use extract_response::{ExtractData, ExtractError, ExtractResponse};
pub use search_extract_config::SearchExtractConfig;
pub use search_lens::{Lens, TimeRelative};
pub use search_personalizations::{
    DomainKind, PersonalizationDomain, PersonalizationRegex, Personalizations,
};
pub use search_request::{Filters, SearchRequest};
pub use search_response::{Image, Meta, SearchData, SearchResponse, SearchResult};
