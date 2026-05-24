use serde::Serialize;
use xxhash_rust::xxh3;

const CACHE_KEY_VERSION: u8 = 1;

/// Generates a deterministic 16-byte content ID from a serializable request.
///
/// Serializes the request to JSON bytes, prepends a version salt byte,
/// and hashes with XXH3-128 to produce a 16-byte content ID.
pub fn generate_cid(request: &impl Serialize) -> [u8; 16] {
    let bytes = serde_json::to_vec(request).expect("serialization should not fail");
    let mut salted = Vec::with_capacity(1 + bytes.len());
    salted.push(CACHE_KEY_VERSION);
    salted.extend_from_slice(&bytes);
    let hash = xxh3::xxh3_128(&salted);
    hash.to_le_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::{ExtractPage, ExtractRequest, SearchRequest};

    #[test]
    fn when_same_request_then_same_cid_should_be_equal() {
        let req = SearchRequest::new("rust programming");

        let cid1 = generate_cid(&req);
        let cid2 = generate_cid(&req);

        assert_eq!(cid1, cid2);
    }

    #[test]
    fn when_different_requests_then_different_cids_should_not_be_equal() {
        let req1 = SearchRequest::new("rust programming");
        let req2 = SearchRequest::new("python programming");

        let cid1 = generate_cid(&req1);
        let cid2 = generate_cid(&req2);

        assert_ne!(cid1, cid2);
    }

    #[test]
    fn when_generated_then_cid_length_should_be_16_bytes() {
        let req = SearchRequest::new("length test");

        let cid = generate_cid(&req);

        assert_eq!(16, cid.len());
    }

    #[test]
    fn when_salted_then_cid_should_differ_from_unsalted_hash() {
        let req = SearchRequest::new("salt test");
        let bytes = serde_json::to_vec(&req).expect("serialization should not fail");

        let cid = generate_cid(&req);
        let unsalted_hash = xxh3::xxh3_128(&bytes).to_le_bytes();

        assert_ne!(cid, unsalted_hash);
    }

    #[test]
    fn when_called_multiple_then_deterministic_should_be_consistent() {
        let req = SearchRequest::new("deterministic test");

        let cid = generate_cid(&req);
        for _ in 0..100 {
            assert_eq!(cid, generate_cid(&req));
        }
    }

    #[test]
    fn when_extract_request_then_cid_should_be_stable() {
        let req = ExtractRequest::new(vec![ExtractPage {
            url: "https://example.com".into(),
        }]);

        let cid1 = generate_cid(&req);
        let cid2 = generate_cid(&req);

        assert_eq!(cid1, cid2);
    }

    #[test]
    fn when_default_fields_then_explicit_and_implicit_should_be_equal() {
        let implicit = SearchRequest::new("test");
        let explicit = SearchRequest::new("test");

        assert_eq!(generate_cid(&implicit), generate_cid(&explicit));
    }
}
