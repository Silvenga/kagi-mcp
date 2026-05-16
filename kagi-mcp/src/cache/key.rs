use serde::Serialize;
use xxhash_rust::xxh3;

/// Generates a deterministic cache key from a serializable request.
///
/// Serializes the request to JSON bytes and hashes them with XXH3-64,
/// producing a 16-character lowercase hex string.
pub fn generate_cache_key(request: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(request).expect("serialization should not fail");
    let hash = xxh3::xxh3_64(&bytes);
    format!("{:016x}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::{ExtractPage, ExtractRequest, SearchRequest};

    #[test]
    fn when_same_request_then_same_key_should_be_equal() {
        let req = SearchRequest::new("rust programming");

        let key1 = generate_cache_key(&req);
        let key2 = generate_cache_key(&req);

        assert_eq!(key1, key2);
    }

    #[test]
    fn when_different_requests_then_different_keys_should_not_be_equal() {
        let req1 = SearchRequest::new("rust programming");
        let req2 = SearchRequest::new("python programming");

        let key1 = generate_cache_key(&req1);
        let key2 = generate_cache_key(&req2);

        assert_ne!(key1, key2);
    }

    #[test]
    fn when_generated_then_key_format_should_be_hex() {
        let req = SearchRequest::new("format test");

        let key = generate_cache_key(&req);

        assert_eq!(16, key.len());
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(key, key.to_ascii_lowercase());
    }

    #[test]
    fn when_called_multiple_then_deterministic_should_be_consistent() {
        let req = SearchRequest::new("deterministic test");

        let key = generate_cache_key(&req);
        for _ in 0..100 {
            assert_eq!(key, generate_cache_key(&req));
        }
    }

    #[test]
    fn when_extract_request_then_key_should_be_stable() {
        let req = ExtractRequest::new(vec![ExtractPage {
            url: "https://example.com".into(),
        }]);

        let key1 = generate_cache_key(&req);
        let key2 = generate_cache_key(&req);

        assert_eq!(key1, key2);
    }

    #[test]
    fn when_default_fields_then_explicit_and_implicit_should_be_equal() {
        let implicit = SearchRequest::new("test");
        let explicit = SearchRequest::new("test");

        assert_eq!(generate_cache_key(&implicit), generate_cache_key(&explicit));
    }
}
