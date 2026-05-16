use kagi_api::{ExtractPage, ExtractRequest, SearchRequest};
use serde::Serialize;
use xxhash_rust::xxh3;

fn generate_cache_key(request: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(request).expect("serialization should not fail");
    let hash = xxh3::xxh3_64(&bytes);
    format!("{:016x}", hash)
}

#[test]
fn when_search_request_serialized_then_cache_key_should_be_stable() {
    let request = SearchRequest::new("rust programming");
    let key = generate_cache_key(&request);
    assert_eq!(key, "19b3e9497f295d8a");
}

#[test]
fn when_extract_request_serialized_then_cache_key_should_be_stable() {
    let request = ExtractRequest::new(vec![ExtractPage {
        url: "https://example.com".to_owned(),
    }]);
    let key = generate_cache_key(&request);
    assert_eq!(key, "60ebcbeac45ec224");
}
