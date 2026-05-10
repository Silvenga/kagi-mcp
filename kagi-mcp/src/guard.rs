/// Default maximum response size in bytes (256KB).
#[expect(dead_code)]
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 262_144;

/// Truncates content if it exceeds the maximum byte limit.
///
/// If content is within the limit, returns it unchanged.
/// If content exceeds the limit, truncates at the last valid UTF-8 boundary
/// before the limit and appends a truncation notice.
#[expect(dead_code)]
pub fn truncate_response(content: &str, max_bytes: usize) -> String {
    let content_bytes = content.len();

    if content_bytes <= max_bytes {
        return content.to_string();
    }

    // Find the last valid UTF-8 boundary before max_bytes
    let mut truncate_at = max_bytes;
    while truncate_at > 0 && !content.is_char_boundary(truncate_at) {
        truncate_at -= 1;
    }

    let truncated = &content[..truncate_at];
    format!("{truncated}\n\n_(Content truncated. Total size: {content_bytes} bytes)_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_content_under_limit_then_returned_unchanged() {
        let content = "a".repeat(100);
        let result = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
        assert_eq!(result, content);
    }

    #[test]
    fn when_content_at_limit_then_returned_unchanged() {
        let content = "a".repeat(DEFAULT_MAX_RESPONSE_BYTES);
        let result = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
        assert_eq!(result, content);
    }

    #[test]
    fn when_content_over_limit_then_truncated_with_notice() {
        let content = "a".repeat(300 * 1024); // 300KB
        let result = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
        assert!(result.len() < content.len());
        assert!(result.ends_with("_(Content truncated. Total size: 307200 bytes)_"));
    }

    #[test]
    fn when_truncated_then_notice_includes_correct_byte_count() {
        let content = "a".repeat(300 * 1024); // 300KB = 307200 bytes
        let result = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);
        assert!(result.contains("Total size: 307200 bytes"));
    }

    #[test]
    fn when_multi_byte_char_at_boundary_then_truncated_correctly() {
        // Each emoji is 4 bytes. 256KB = 65536 emojis.
        // We'll create 66000 emojis (264000 bytes) which exceeds 256KB (262144 bytes).
        let emoji_count = 66_000;
        let content: String = std::iter::repeat('😀').take(emoji_count).collect();
        // Total: 66_000 * 4 = 264_000 bytes > 262_144
        let result = truncate_response(&content, DEFAULT_MAX_RESPONSE_BYTES);

        // Verify truncation happened
        assert!(result.len() < content.len(), "result should be shorter than original");

        // Verify the truncated content ends with a complete character (no broken UTF-8)
        assert!(result.is_char_boundary(result.len() - 1) || result.is_char_boundary(result.len()));
        // Actually, let's verify the whole string is valid UTF-8 and ends with the notice
        assert!(result.ends_with(")_"), "result should end with truncation notice");

        // Verify the truncated content portion is valid UTF-8 (no mid-char cut)
        let truncated_part = result.strip_suffix("\n\n_(Content truncated. Total size: 264000 bytes)_").unwrap();
        assert_eq!(truncated_part.len() % 4, 0, "emoji string should be truncated at char boundary (multiple of 4 bytes)");
    }
}
