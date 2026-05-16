pub(crate) fn collapse_snippet_ellipses(s: &str) -> String {
    // First, normalize runs of 3+ consecutive dots to space-padded `...`,
    // which handles directly concatenated ellipsis like `......`.
    let normalized = {
        let mut buf = String::with_capacity(s.len());
        let mut dot_count = 0;
        for ch in s.chars() {
            if ch == '.' {
                dot_count += 1;
            } else {
                if dot_count >= 3 {
                    buf.push_str(" ... ");
                } else {
                    for _ in 0..dot_count {
                        buf.push('.');
                    }
                }
                dot_count = 0;
                buf.push(ch);
            }
        }
        // Handle trailing dots
        if dot_count >= 3 {
            buf.push_str(" ... ");
        } else {
            for _ in 0..dot_count {
                buf.push('.');
            }
        }
        buf
    };

    // Then collapse whitespace-separated consecutive `...`
    let mut result = Vec::new();
    let mut prev_was_ellipsis = false;
    for word in normalized.split_whitespace() {
        if word == "..." {
            if !prev_was_ellipsis {
                result.push(word);
                prev_was_ellipsis = true;
            }
        } else {
            result.push(word);
            prev_was_ellipsis = false;
        }
    }
    result.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_collapse_ellipses_with_multiple_runs_then_should_collapse_to_one() {
        assert_eq!(
            collapse_snippet_ellipses("foo ... ... ... bar"),
            "foo ... bar"
        );
    }

    #[test]
    fn when_collapse_ellipses_with_single_run_then_should_preserve() {
        assert_eq!(collapse_snippet_ellipses("foo ... bar"), "foo ... bar");
    }

    #[test]
    fn when_collapse_ellipses_with_leading_then_should_preserve() {
        assert_eq!(collapse_snippet_ellipses("... foo"), "... foo");
    }

    #[test]
    fn when_collapse_ellipses_with_trailing_then_should_preserve() {
        assert_eq!(collapse_snippet_ellipses("foo ..."), "foo ...");
    }

    #[test]
    fn when_collapse_ellipses_with_directly_concatenated_then_should_collapse_to_one() {
        assert_eq!(collapse_snippet_ellipses("foo......bar"), "foo ... bar");
    }

    #[test]
    fn when_collapse_ellipses_with_mixed_separators_then_should_collapse_to_one() {
        assert_eq!(
            collapse_snippet_ellipses("foo ... ...... ... bar"),
            "foo ... bar"
        );
    }
}
