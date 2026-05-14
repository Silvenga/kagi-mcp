use kagi_api::types::{ExtractResponse, SearchResponse, SearchResult};

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    html_escape::decode_html_entities(s).into_owned()
}

fn normalize_title_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collapse_snippet_ellipses(s: &str) -> String {
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

fn trim_iso_date(s: &str) -> String {
    let bytes = s.as_bytes();
    if s.len() >= 11 && bytes[4] == b'-' && bytes[7] == b'-' && bytes[10] == b'T' {
        s[..10].to_string()
    } else {
        s.to_owned()
    }
}

pub fn format_search_markdown(response: &SearchResponse) -> String {
    let mut output = String::new();
    let data = &response.data;

    let mut has_results = false;

    let mut format_general = |title: &str, results: &Option<Vec<SearchResult>>| {
        if let Some(results) = results {
            if !results.is_empty() {
                has_results = true;
                output.push_str(&format!("## {}\n\n", title));
                for (i, result) in results.iter().enumerate() {
                    output.push_str(&format!(
                        "{}. **[{}]({})**\n",
                        i + 1,
                        decode_entities(&normalize_title_whitespace(&result.title)),
                        result.url
                    ));
                    if let Some(snippet) = &result.snippet {
                        output.push_str(&format!(
                            "   - Snippet: {}\n",
                            decode_entities(&collapse_snippet_ellipses(snippet))
                        ));
                    }
                    if let Some(time) = &result.time {
                        output.push_str(&format!("   - Published: {}\n", trim_iso_date(time)));
                    }
                }
                output.push('\n');
            }
        }
    };

    format_general("Web Results", &data.search);
    format_general("News", &data.news);
    format_general("Interesting News", &data.interesting_news);
    format_general("Interesting Finds", &data.interesting_finds);
    format_general("Code Results", &data.code);
    format_general("Public Records", &data.public_records);
    format_general("Listicles", &data.listicle);
    format_general("Web Archive", &data.web_archive);

    if let Some(results) = &data.image {
        if !results.is_empty() {
            has_results = true;
            output.push_str("## Images\n\n");
            for (i, result) in results.iter().enumerate() {
                output.push_str(&format!(
                    "{}. **[{}]({})**\n",
                    i + 1,
                    decode_entities(&normalize_title_whitespace(&result.title)),
                    result.url
                ));
                if let Some(image) = &result.image {
                    let width = image.width.map_or("?".to_owned(), |w| w.to_string());
                    let height = image.height.map_or("?".to_owned(), |h| h.to_string());
                    output.push_str(&format!(
                        "   - Image: {} ({}x{})\n",
                        image.url, width, height
                    ));
                }
            }
            output.push('\n');
        }
    }

    if let Some(results) = &data.video {
        if !results.is_empty() {
            has_results = true;
            output.push_str("## Videos\n\n");
            for (i, result) in results.iter().enumerate() {
                output.push_str(&format!(
                    "{}. **[{}]({})**\n",
                    i + 1,
                    decode_entities(&normalize_title_whitespace(&result.title)),
                    result.url
                ));
                if let Some(snippet) = &result.snippet {
                    output.push_str(&format!(
                        "   - Snippet: {}\n",
                        decode_entities(&collapse_snippet_ellipses(snippet))
                    ));
                }
                if let Some(time) = &result.time {
                    output.push_str(&format!("   - Published: {}\n", trim_iso_date(time)));
                }
            }
            output.push('\n');
        }
    }

    if let Some(results) = &data.podcast {
        if !results.is_empty() {
            has_results = true;
            output.push_str("## Podcasts\n\n");
            for (i, result) in results.iter().enumerate() {
                output.push_str(&format!(
                    "{}. **[{}]({})**\n",
                    i + 1,
                    decode_entities(&normalize_title_whitespace(&result.title)),
                    result.url
                ));
                if let Some(snippet) = &result.snippet {
                    output.push_str(&format!(
                        "   - Snippet: {}\n",
                        decode_entities(&collapse_snippet_ellipses(snippet))
                    ));
                }
                if let Some(time) = &result.time {
                    output.push_str(&format!("   - Published: {}\n", trim_iso_date(time)));
                }
            }
            output.push('\n');
        }
    }

    if let Some(results) = &data.podcast_creator {
        if !results.is_empty() {
            has_results = true;
            output.push_str("## Podcast Creators\n\n");
            for (i, result) in results.iter().enumerate() {
                output.push_str(&format!(
                    "{}. **[{}]({})**\n",
                    i + 1,
                    decode_entities(&normalize_title_whitespace(&result.title)),
                    result.url
                ));
                if let Some(snippet) = &result.snippet {
                    output.push_str(&format!(
                        "   - Snippet: {}\n",
                        decode_entities(&collapse_snippet_ellipses(snippet))
                    ));
                }
                if let Some(time) = &result.time {
                    output.push_str(&format!("   - Published: {}\n", trim_iso_date(time)));
                }
            }
            output.push('\n');
        }
    }

    if let Some(results) = &data.adjacent_question {
        if !results.is_empty() {
            has_results = true;
            output.push_str("## Related Questions\n\n");
            for (i, result) in results.iter().enumerate() {
                let question = result
                    .props
                    .as_ref()
                    .and_then(|p| p.get("question"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Question");
                let question = decode_entities(&normalize_title_whitespace(question));
                output.push_str(&format!("{}. **{}**\n", i + 1, question));
                if let Some(snippet) = &result.snippet {
                    output.push_str(&format!(
                        "    - [Answer]({}): {}\n",
                        result.url,
                        decode_entities(&collapse_snippet_ellipses(snippet))
                    ));
                }
            }
            output.push('\n');
        }
    }

    if let Some(results) = &data.direct_answer {
        if !results.is_empty() {
            has_results = true;
            output.push_str("## Direct Answer\n\n");
            for result in results {
                if let Some(snippet) = &result.snippet {
                    output.push_str(&format!(
                        "{}\n\n",
                        decode_entities(&collapse_snippet_ellipses(snippet))
                    ));
                }
            }
        }
    }

    if let Some(results) = &data.infobox {
        if !results.is_empty() {
            has_results = true;
            output.push_str("## Infobox\n\n");
            for result in results {
                output.push_str(&format!(
                    "**[{}]({})**\n\n",
                    decode_entities(&normalize_title_whitespace(&result.title)),
                    result.url
                ));
                if let Some(snippet) = &result.snippet {
                    output.push_str(&format!(
                        "{}\n\n",
                        decode_entities(&collapse_snippet_ellipses(snippet))
                    ));
                }
                if let Some(props) = &result.props {
                    if let Some(infobox) = props.get("infobox") {
                        if let Some(obj) = infobox.as_object() {
                            for (key, value) in obj {
                                let val_str = if let Some(s) = value.as_str() {
                                    s.to_owned()
                                } else {
                                    value.to_string()
                                };
                                output.push_str(&format!("{}: {}\n", key, val_str));
                            }
                        }
                    }
                }
                output.push('\n');
            }
        }
    }

    if let Some(results) = &data.related_search {
        if !results.is_empty() {
            has_results = true;
            output.push_str("## Related Searches\n\n");
            for result in results {
                output.push_str(&format!(
                    "- {}\n",
                    decode_entities(&normalize_title_whitespace(&result.title))
                ));
            }
            output.push('\n');
        }
    }

    if let Some(results) = &data.weather {
        if !results.is_empty() {
            has_results = true;
            output.push_str("## Weather\n\n");
            for result in results {
                if let Some(snippet) = &result.snippet {
                    output.push_str(&format!(
                        "{}\n",
                        decode_entities(&collapse_snippet_ellipses(snippet))
                    ));
                }
            }
            output.push('\n');
        }
    }

    if let Some(results) = &data.package_tracking {
        if !results.is_empty() {
            has_results = true;
            output.push_str("## Package Tracking\n\n");
            for result in results {
                output.push_str(&format!("- [Tracking Link]({})\n", result.url));
            }
            output.push('\n');
        }
    }

    if !has_results {
        return "No results found.".to_owned();
    }

    output.trim_end().to_owned()
}

pub fn format_extract_markdown(response: &ExtractResponse) -> String {
    let mut output = String::new();
    output.push_str("## Extracted Content\n\n");

    let mut has_content = false;

    if let Some(data) = &response.data {
        for item in data {
            has_content = true;
            output.push_str(&format!("### {}\n\n", item.url));
            if let Some(markdown) = &item.markdown {
                output.push_str(&format!("{}\n\n", markdown));
            }
            output.push_str("---\n\n");
        }
    }

    if let Some(errors) = &response.errors {
        for error in errors {
            has_content = true;
            output.push_str(&format!("### {}\n\n", error.url));
            let message = error.message.as_deref().unwrap_or("Unknown error");
            output.push_str(&format!("**Extraction failed:** {}\n\n", message));
        }
    }

    if !has_content {
        return "No content extracted.".to_owned();
    }

    output.trim_end().to_owned()
}

pub fn format_json<T: serde::Serialize>(response: &T) -> String {
    serde_json::to_string_pretty(response)
        .unwrap_or_else(|e| format!("JSON serialization error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::types::{ExtractData, ExtractError, Image, Meta, SearchData};

    #[test]
    fn format_search_markdown_empty() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        assert_eq!(format_search_markdown(&response), "No results found.");
    }

    #[test]
    fn format_search_markdown_web_results() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: Some(vec![SearchResult {
                    url: "https://example.com".to_owned(),
                    title: "Example".to_owned(),
                    snippet: Some("This is an example.".to_owned()),
                    time: Some("2023-01-01".to_owned()),
                    image: None,
                    props: None,
                }]),
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Web Results\n\n1. **[Example](https://example.com)**\n   - Snippet: This is an example.\n   - Published: 2023-01-01";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn format_search_markdown_images() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: Some(vec![SearchResult {
                    url: "https://example.com/page".to_owned(),
                    title: "Example Image".to_owned(),
                    snippet: None,
                    time: None,
                    image: Some(Image {
                        url: "https://example.com/image.jpg".to_owned(),
                        width: Some(800),
                        height: Some(600),
                    }),
                    props: None,
                }]),
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Images\n\n1. **[Example Image](https://example.com/page)**\n   - Image: https://example.com/image.jpg (800x600)";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn format_search_markdown_missing_snippet_time() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: Some(vec![SearchResult {
                    url: "https://example.com".to_owned(),
                    title: "Example".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                }]),
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Web Results\n\n1. **[Example](https://example.com)**";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_data_has_podcast_creator_then_markdown_should_include_podcast_creator_section() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: None,
                podcast_creator: Some(vec![SearchResult {
                    url: "https://example.com/creator1".to_owned(),
                    title: "Creator One".to_owned(),
                    snippet: Some("A great podcast creator.".to_owned()),
                    time: Some("2024-01-15".to_owned()),
                    image: None,
                    props: None,
                }]),
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Podcast Creators\n\n1. **[Creator One](https://example.com/creator1)**\n   - Snippet: A great podcast creator.\n   - Published: 2024-01-15";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_data_has_podcast_then_markdown_should_include_podcast_section() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: Some(vec![SearchResult {
                    url: "https://example.com/podcast1".to_owned(),
                    title: "Podcast One".to_owned(),
                    snippet: Some("A great podcast episode.".to_owned()),
                    time: Some("2024-01-15".to_owned()),
                    image: None,
                    props: None,
                }]),
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Podcasts\n\n1. **[Podcast One](https://example.com/podcast1)**\n   - Snippet: A great podcast episode.\n   - Published: 2024-01-15";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_data_has_video_then_markdown_should_include_video_section() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: Some(vec![SearchResult {
                    url: "https://example.com/video1".to_owned(),
                    title: "Video One".to_owned(),
                    snippet: Some("A great video.".to_owned()),
                    time: Some("2024-02-01".to_owned()),
                    image: None,
                    props: None,
                }]),
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Videos\n\n1. **[Video One](https://example.com/video1)**\n   - Snippet: A great video.\n   - Published: 2024-02-01";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_data_has_adjacent_question_then_markdown_should_include_related_questions() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: Some(vec![SearchResult {
                    url: "https://example.com/answer".to_owned(),
                    title: "Answer Page".to_owned(),
                    snippet: Some("The answer is 42.".to_owned()),
                    time: None,
                    image: None,
                    props: Some(serde_json::json!({"question": "What is the meaning of life?"})),
                }]),
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Related Questions\n\n1. **What is the meaning of life?**\n    - [Answer](https://example.com/answer): The answer is 42.";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_data_has_direct_answer_then_markdown_should_include_direct_answer() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: Some(vec![SearchResult {
                    url: "https://example.com".to_owned(),
                    title: "Answer".to_owned(),
                    snippet: Some("The direct answer is 42.".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                }]),
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Direct Answer\n\nThe direct answer is 42.";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_data_has_infobox_then_markdown_should_include_infobox_section() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: Some(vec![SearchResult {
                    url: "https://example.com/info".to_owned(),
                    title: "Info Title".to_owned(),
                    snippet: Some("Key information.".to_owned()),
                    time: None,
                    image: None,
                    props: Some(
                        serde_json::json!({"infobox": {"Population": "1.4B", "Capital": "Beijing"}}),
                    ),
                }]),
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let markdown = format_search_markdown(&response);
        assert!(markdown.contains("## Infobox"));
        assert!(markdown.contains("**[Info Title](https://example.com/info)**"));
        assert!(markdown.contains("Population: 1.4B"));
        assert!(markdown.contains("Capital: Beijing"));
    }

    #[test]
    fn when_search_data_has_related_search_then_markdown_should_include_related_searches() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: Some(vec![SearchResult {
                    url: "https://example.com".to_owned(),
                    title: "Related Topic".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                }]),
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Related Searches\n\n- Related Topic";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_data_has_weather_then_markdown_should_include_weather_section() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: Some(vec![SearchResult {
                    url: "".to_owned(),
                    title: "".to_owned(),
                    snippet: Some("Sunny, 25°C".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                }]),
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Weather\n\nSunny, 25°C";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_data_has_package_tracking_then_markdown_should_include_package_tracking() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: Some(vec![SearchResult {
                    url: "https://track.example.com/1".to_owned(),
                    title: "Package".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                }]),
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Package Tracking\n\n- [Tracking Link](https://track.example.com/1)";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn format_search_markdown_mixed() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: Some(vec![SearchResult {
                    url: "https://example.com".to_owned(),
                    title: "Example".to_owned(),
                    snippet: Some("This is an example.".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                }]),
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: Some(vec![SearchResult {
                    url: "".to_owned(),
                    title: "".to_owned(),
                    snippet: Some("Sunny, 25C".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                }]),
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Web Results\n\n1. **[Example](https://example.com)**\n   - Snippet: This is an example.\n\n## Weather\n\nSunny, 25C";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn format_extract_markdown_success() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: Some(vec![ExtractData {
                url: "https://example.com".to_owned(),
                markdown: Some("# Hello\nWorld".to_owned()),
            }]),
            errors: None,
        };

        let expected = "## Extracted Content\n\n### https://example.com\n\n# Hello\nWorld\n\n---";
        assert_eq!(format_extract_markdown(&response), expected);
    }

    #[test]
    fn format_extract_markdown_failure() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: None,
            errors: Some(vec![ExtractError {
                url: "https://example.com".to_owned(),
                code: "404".to_owned(),
                message: Some("Not Found".to_owned()),
            }]),
        };

        let expected =
            "## Extracted Content\n\n### https://example.com\n\n**Extraction failed:** Not Found";
        assert_eq!(format_extract_markdown(&response), expected);
    }

    #[test]
    fn format_json_works() {
        let data = serde_json::json!({
            "key": "value"
        });
        let expected = "{\n  \"key\": \"value\"\n}";
        assert_eq!(super::format_json(&data), expected);
    }

    #[test]
    fn when_decode_entities_with_known_entities_then_should_decode_them() {
        let result = decode_entities("foo &amp; bar &quot;baz&quot; &lt;qux&gt;");

        assert_eq!(result, "foo & bar \"baz\" <qux>");
    }

    #[test]
    fn when_decode_entities_with_no_entities_then_should_return_unchanged() {
        let result = decode_entities("hello world");

        assert_eq!(result, "hello world");
    }

    #[test]
    fn when_decode_entities_with_numeric_entity_then_should_decode_it() {
        let result = decode_entities("it&#39;s");

        assert_eq!(result, "it's");
    }

    #[test]
    fn when_normalize_title_with_double_space_then_should_collapse_to_single() {
        let result = normalize_title_whitespace("hello   world");

        assert_eq!(result, "hello world");
    }

    #[test]
    fn when_normalize_title_with_leading_trailing_whitespace_then_should_trim() {
        let result = normalize_title_whitespace("  hello world  ");

        assert_eq!(result, "hello world");
    }

    #[test]
    fn when_normalize_title_with_tabs_and_newlines_then_should_collapse() {
        let result = normalize_title_whitespace("hello\t\tworld\nfoo\r\nbar");

        assert_eq!(result, "hello world foo bar");
    }

    #[test]
    fn when_collapse_ellipses_with_multiple_runs_then_should_collapse_to_one() {
        let result = collapse_snippet_ellipses("foo ... ... ... bar");

        assert_eq!(result, "foo ... bar");
    }

    #[test]
    fn when_collapse_ellipses_with_single_run_then_should_preserve() {
        let result = collapse_snippet_ellipses("foo ... bar");

        assert_eq!(result, "foo ... bar");
    }

    #[test]
    fn when_collapse_ellipses_with_leading_trailing_then_should_preserve() {
        let result = collapse_snippet_ellipses("... foo");

        assert_eq!(result, "... foo");
    }

    #[test]
    fn when_collapse_ellipses_with_trailing_then_should_preserve() {
        let result = collapse_snippet_ellipses("foo ...");

        assert_eq!(result, "foo ...");
    }

    #[test]
    fn when_collapse_ellipses_with_directly_concatenated_then_should_collapse_to_one() {
        let result = collapse_snippet_ellipses("foo......bar");

        assert_eq!(result, "foo ... bar");
    }

    #[test]
    fn when_collapse_ellipses_with_mixed_separators_then_should_collapse_to_one() {
        let result = collapse_snippet_ellipses("foo ... ...... ... bar");

        assert_eq!(result, "foo ... bar");
    }

    #[test]
    fn when_trim_iso_date_with_full_timestamp_then_should_return_date_only() {
        let result = trim_iso_date("2011-06-06T10:52:26Z");

        assert_eq!(result, "2011-06-06");
    }

    #[test]
    fn when_trim_iso_date_with_already_date_only_then_should_return_unchanged() {
        let result = trim_iso_date("2023-01-01");

        assert_eq!(result, "2023-01-01");
    }

    #[test]
    fn when_trim_iso_date_with_non_iso_string_then_should_return_unchanged() {
        let result = trim_iso_date("not-a-date");

        assert_eq!(result, "not-a-date");
    }

    #[test]
    fn when_format_search_with_html_entities_in_title_then_should_decode() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: Some(vec![SearchResult {
                    url: "https://example.com".to_owned(),
                    title: "Foo &amp; Bar &quot;baz&quot;".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                }]),
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Web Results\n\n1. **[Foo & Bar \"baz\"](https://example.com)**";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_format_search_with_html_entities_in_snippet_then_should_decode() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: Some(vec![SearchResult {
                    url: "https://example.com".to_owned(),
                    title: "Example".to_owned(),
                    snippet: Some("It&#39;s great &amp; amazing.".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                }]),
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Web Results\n\n1. **[Example](https://example.com)**\n   - Snippet: It's great & amazing.";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_format_search_with_iso_timestamp_in_time_then_should_render_date_only() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: Some(vec![SearchResult {
                    url: "https://example.com".to_owned(),
                    title: "Example".to_owned(),
                    snippet: None,
                    time: Some("2024-03-15T10:30:00Z".to_owned()),
                    image: None,
                    props: None,
                }]),
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected =
            "## Web Results\n\n1. **[Example](https://example.com)**\n   - Published: 2024-03-15";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_format_search_with_ellipsis_run_in_snippet_then_should_collapse() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: Some(vec![SearchResult {
                    url: "https://example.com".to_owned(),
                    title: "Example".to_owned(),
                    snippet: Some("foo ... ... ... bar".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                }]),
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected =
            "## Web Results\n\n1. **[Example](https://example.com)**\n   - Snippet: foo ... bar";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_format_search_has_news_then_section_header_should_be_news_not_web_results() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: Some(vec![SearchResult {
                    url: "https://example.com/news".to_owned(),
                    title: "News Item".to_owned(),
                    snippet: Some("Breaking news.".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                }]),
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected =
            "## News\n\n1. **[News Item](https://example.com/news)**\n   - Snippet: Breaking news.";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_format_search_has_code_then_section_header_should_be_code_results() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: Some(vec![SearchResult {
                    url: "https://example.com/code".to_owned(),
                    title: "Code Snippet".to_owned(),
                    snippet: Some("fn main() {}".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                }]),
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: None,
                web_archive: None,
            },
        };

        let expected = "## Code Results\n\n1. **[Code Snippet](https://example.com/code)**\n   - Snippet: fn main() {}";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_format_search_has_listicle_then_section_header_should_be_listicles() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: None,
                adjacent_question: None,
                direct_answer: None,
                interesting_news: None,
                interesting_finds: None,
                infobox: None,
                code: None,
                package_tracking: None,
                public_records: None,
                weather: None,
                related_search: None,
                listicle: Some(vec![SearchResult {
                    url: "https://example.com/list".to_owned(),
                    title: "Top 10 Things".to_owned(),
                    snippet: Some("A listicle.".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                }]),
                web_archive: None,
            },
        };

        let expected = "## Listicles\n\n1. **[Top 10 Things](https://example.com/list)**\n   - Snippet: A listicle.";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_format_search_has_all_8_web_categories_then_each_should_have_distinct_header() {
        let mk = |title: &str| SearchResult {
            url: "https://example.com".to_owned(),
            title: title.to_owned(),
            snippet: None,
            time: None,
            image: None,
            props: None,
        };

        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: Some(vec![mk("Web")]),
                image: None,
                video: None,
                podcast: None,
                podcast_creator: None,
                news: Some(vec![mk("News")]),
                adjacent_question: None,
                direct_answer: None,
                interesting_news: Some(vec![mk("Interesting News")]),
                interesting_finds: Some(vec![mk("Interesting Finds")]),
                infobox: None,
                code: Some(vec![mk("Code")]),
                package_tracking: None,
                public_records: Some(vec![mk("Public Records")]),
                weather: None,
                related_search: None,
                listicle: Some(vec![mk("Listicle")]),
                web_archive: Some(vec![mk("Web Archive")]),
            },
        };

        let markdown = format_search_markdown(&response);
        assert!(markdown.contains("## Web Results"));
        assert!(markdown.contains("## News"));
        assert!(markdown.contains("## Interesting News"));
        assert!(markdown.contains("## Interesting Finds"));
        assert!(markdown.contains("## Code Results"));
        assert!(markdown.contains("## Public Records"));
        assert!(markdown.contains("## Listicles"));
        assert!(markdown.contains("## Web Archive"));
    }
}
