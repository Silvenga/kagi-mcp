use std::sync::LazyLock;

use regex::Regex;

static HTML_TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());

/// Strip HTML tags (`<...>`) from a string, preserving text content.
pub fn strip_html_tags(s: &str) -> String {
    if !s.contains('<') {
        return s.to_owned();
    }
    HTML_TAG_RE.replace_all(s, "").into_owned()
}

pub fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    html_escape::decode_html_entities(s).into_owned()
}

pub fn normalize_title_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn trim_iso_date(s: &str) -> String {
    let bytes = s.as_bytes();
    if s.len() >= 11 && bytes[4] == b'-' && bytes[7] == b'-' && bytes[10] == b'T' {
        s[..10].to_string()
    } else {
        s.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_strip_tags_with_strong_then_should_remove() {
        assert_eq!(strip_html_tags("<strong>2026</strong>"), "2026");
    }

    #[test]
    fn when_strip_tags_with_anchor_and_attributes_then_should_remove_tag_keep_text() {
        assert_eq!(
            strip_html_tags(
                r#"<a href="https://en.wikipedia.org/wiki/Paris" data-wiki-article="/wiki/Paris">Paris</a>"#
            ),
            "Paris",
        );
    }

    #[test]
    fn when_strip_tags_with_nested_then_should_remove_all_tags_keep_inner_text() {
        assert_eq!(strip_html_tags("<a><strong>text</strong></a>"), "text",);
    }

    #[test]
    fn when_strip_tags_with_self_closing_br_then_should_remove() {
        assert_eq!(strip_html_tags("foo<br/>bar"), "foobar");
    }

    #[test]
    fn when_strip_tags_with_no_angle_bracket_then_should_return_unchanged() {
        assert_eq!(strip_html_tags("no tags here"), "no tags here");
    }

    #[test]
    fn when_strip_tags_with_empty_then_should_return_empty() {
        assert_eq!(strip_html_tags(""), "");
    }

    #[test]
    fn when_strip_tags_with_text_only_inside_tags_then_should_preserve_text() {
        assert_eq!(
            strip_html_tags("<strong>Paris</strong>, city and capital of <strong>France</strong>"),
            "Paris, city and capital of France",
        );
    }

    #[test]
    fn when_strip_tags_with_entities_in_text_then_should_preserve_undecoded() {
        assert_eq!(
            strip_html_tags("It&#39;s <strong>great</strong> &amp; amazing"),
            "It&#39;s great &amp; amazing",
        );
    }

    #[test]
    fn when_strip_tags_with_entities_in_attribute_then_should_discard_with_tag() {
        assert_eq!(
            strip_html_tags(r#"<a href="/wiki/Beneficiary">heiress</a>"#),
            "heiress",
        );
    }

    #[test]
    fn when_strip_tags_with_multiple_tags_in_sequence_then_should_remove_all() {
        assert_eq!(
            strip_html_tags("<b>foo</b> <i>bar</i> <u>baz</u>"),
            "foo bar baz",
        );
    }

    #[test]
    fn when_strip_tags_with_unclosed_tag_at_end_then_should_preserve_unchanged() {
        assert_eq!(strip_html_tags("text <unterminated"), "text <unterminated");
    }

    #[test]
    fn when_strip_tags_with_trailing_angle_bracket_only_then_should_preserve() {
        assert_eq!(strip_html_tags("3 < 5"), "3 < 5");
    }

    #[test]
    fn when_strip_tags_with_only_tags_then_should_return_empty() {
        assert_eq!(strip_html_tags("<strong></strong>"), "");
    }

    #[test]
    fn when_strip_tags_with_data_attributes_then_should_remove_all_attributes() {
        assert_eq!(
            strip_html_tags(
                r#"<span data-wiki-article="/wiki/X" data-wiki-locale="en">word</span>"#
            ),
            "word",
        );
    }

    #[test]
    fn when_decode_entities_with_known_entities_then_should_decode_them() {
        assert_eq!(
            decode_entities("foo &amp; bar &quot;baz&quot; &lt;qux&gt;"),
            "foo & bar \"baz\" <qux>"
        );
    }

    #[test]
    fn when_decode_entities_with_no_entities_then_should_return_unchanged() {
        assert_eq!(decode_entities("hello world"), "hello world");
    }

    #[test]
    fn when_decode_entities_with_numeric_entity_then_should_decode_it() {
        assert_eq!(decode_entities("it&#39;s"), "it's");
    }

    #[test]
    fn when_normalize_title_with_double_space_then_should_collapse_to_single() {
        assert_eq!(normalize_title_whitespace("hello   world"), "hello world");
    }

    #[test]
    fn when_normalize_title_with_leading_trailing_whitespace_then_should_trim() {
        assert_eq!(normalize_title_whitespace("  hello world  "), "hello world");
    }

    #[test]
    fn when_normalize_title_with_tabs_and_newlines_then_should_collapse() {
        assert_eq!(
            normalize_title_whitespace("hello\t\tworld\nfoo\r\nbar"),
            "hello world foo bar"
        );
    }

    #[test]
    fn when_trim_iso_date_with_full_timestamp_then_should_return_date_only() {
        assert_eq!(trim_iso_date("2011-06-06T10:52:26Z"), "2011-06-06");
    }

    #[test]
    fn when_trim_iso_date_with_already_date_only_then_should_return_unchanged() {
        assert_eq!(trim_iso_date("2023-01-01"), "2023-01-01");
    }

    #[test]
    fn when_trim_iso_date_with_non_iso_string_then_should_return_unchanged() {
        assert_eq!(trim_iso_date("not-a-date"), "not-a-date");
    }
}
