mod error_response;
mod extract_request;
mod extract_response;
mod search_request;
mod search_response;

pub use error_response::{ErrorDetail, KagiErrorResponse};
pub use extract_request::{ExtractPage, ExtractRequest};
pub use extract_response::{ExtractData, ExtractError, ExtractResponse};
pub use search_request::{
    DomainKind, Filters, Lens, PersonalizationDomain, PersonalizationRegex, Personalizations,
    SearchExtractConfig, SearchRequest, TimeRelative,
};
pub use search_response::{Image, Meta, SearchData, SearchResponse, SearchResult};
