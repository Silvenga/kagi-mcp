pub(crate) fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    html_escape::decode_html_entities(s).into_owned()
}

pub(crate) fn normalize_title_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn trim_iso_date(s: &str) -> String {
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
