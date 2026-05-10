use kagi_api::types::{ExtractResponse, SearchResponse, SearchResult};

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
                        result.title,
                        result.url
                    ));
                    if let Some(snippet) = &result.snippet {
                        output.push_str(&format!("   - Snippet: {}\n", snippet));
                    }
                    if let Some(time) = &result.time {
                        output.push_str(&format!("   - Published: {}\n", time));
                    }
                }
                output.push('\n');
            }
        }
    };

    format_general("Web Results", &data.search);
    format_general("Web Results", &data.news);
    format_general("Web Results", &data.interesting_news);
    format_general("Web Results", &data.interesting_finds);
    format_general("Web Results", &data.code);
    format_general("Web Results", &data.public_records);
    format_general("Web Results", &data.listicle);
    format_general("Web Results", &data.web_archive);

    if let Some(results) = &data.image {
        if !results.is_empty() {
            has_results = true;
            output.push_str("## Images\n\n");
            for (i, result) in results.iter().enumerate() {
                output.push_str(&format!(
                    "{}. **[{}]({})**\n",
                    i + 1,
                    result.title,
                    result.url
                ));
                if let Some(image) = &result.image {
                    let width = image.width.map_or("?".to_string(), |w| w.to_string());
                    let height = image.height.map_or("?".to_string(), |h| h.to_string());
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
                    result.title,
                    result.url
                ));
                if let Some(snippet) = &result.snippet {
                    output.push_str(&format!("   - Snippet: {}\n", snippet));
                }
                if let Some(time) = &result.time {
                    output.push_str(&format!("   - Published: {}\n", time));
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
                    result.title,
                    result.url
                ));
                if let Some(snippet) = &result.snippet {
                    output.push_str(&format!("   - Snippet: {}\n", snippet));
                }
                if let Some(time) = &result.time {
                    output.push_str(&format!("   - Published: {}\n", time));
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
                    result.title,
                    result.url
                ));
                if let Some(snippet) = &result.snippet {
                    output.push_str(&format!("   - Snippet: {}\n", snippet));
                }
                if let Some(time) = &result.time {
                    output.push_str(&format!("   - Published: {}\n", time));
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
                output.push_str(&format!("{}. **{}**\n", i + 1, question));
                if let Some(snippet) = &result.snippet {
                    output.push_str(&format!("    - [Answer]({}): {}\n", result.url, snippet));
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
                    output.push_str(&format!("{}\n\n", snippet));
                }
            }
        }
    }

    if let Some(results) = &data.infobox {
        if !results.is_empty() {
            has_results = true;
            output.push_str("## Infobox\n\n");
            for result in results {
                output.push_str(&format!("**[{}]({})**\n\n", result.title, result.url));
                if let Some(snippet) = &result.snippet {
                    output.push_str(&format!("{}\n\n", snippet));
                }
                if let Some(props) = &result.props {
                    if let Some(infobox) = props.get("infobox") {
                        if let Some(obj) = infobox.as_object() {
                            for (key, value) in obj {
                                let val_str = if let Some(s) = value.as_str() {
                                    s.to_string()
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
                output.push_str(&format!("- {}\n", result.title));
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
                    output.push_str(&format!("{}\n", snippet));
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
        return "No results found.".to_string();
    }

    output.trim_end().to_string()
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
        return "No content extracted.".to_string();
    }

    output.trim_end().to_string()
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
    fn test_format_search_markdown_empty() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_string(),
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
    fn test_format_search_markdown_web_results() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_string(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: Some(vec![SearchResult {
                    url: "https://example.com".to_string(),
                    title: "Example".to_string(),
                    snippet: Some("This is an example.".to_string()),
                    time: Some("2023-01-01".to_string()),
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
    fn test_format_search_markdown_images() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_string(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: Some(vec![SearchResult {
                    url: "https://example.com/page".to_string(),
                    title: "Example Image".to_string(),
                    snippet: None,
                    time: None,
                    image: Some(Image {
                        url: "https://example.com/image.jpg".to_string(),
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
    fn test_format_search_markdown_missing_snippet_time() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_string(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: Some(vec![SearchResult {
                    url: "https://example.com".to_string(),
                    title: "Example".to_string(),
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
                trace: "test".to_string(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: None,
                podcast_creator: Some(vec![SearchResult {
                    url: "https://example.com/creator1".to_string(),
                    title: "Creator One".to_string(),
                    snippet: Some("A great podcast creator.".to_string()),
                    time: Some("2024-01-15".to_string()),
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
                trace: "test".to_string(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: None,
                podcast: Some(vec![SearchResult {
                    url: "https://example.com/podcast1".to_string(),
                    title: "Podcast One".to_string(),
                    snippet: Some("A great podcast episode.".to_string()),
                    time: Some("2024-01-15".to_string()),
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
                trace: "test".to_string(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: None,
                image: None,
                video: Some(vec![SearchResult {
                    url: "https://example.com/video1".to_string(),
                    title: "Video One".to_string(),
                    snippet: Some("A great video.".to_string()),
                    time: Some("2024-02-01".to_string()),
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
                trace: "test".to_string(),
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
                    url: "https://example.com/answer".to_string(),
                    title: "Answer Page".to_string(),
                    snippet: Some("The answer is 42.".to_string()),
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
                trace: "test".to_string(),
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
                    url: "https://example.com".to_string(),
                    title: "Answer".to_string(),
                    snippet: Some("The direct answer is 42.".to_string()),
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
                trace: "test".to_string(),
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
                    url: "https://example.com/info".to_string(),
                    title: "Info Title".to_string(),
                    snippet: Some("Key information.".to_string()),
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
                trace: "test".to_string(),
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
                    url: "https://example.com".to_string(),
                    title: "Related Topic".to_string(),
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
                trace: "test".to_string(),
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
                    url: "".to_string(),
                    title: "".to_string(),
                    snippet: Some("Sunny, 25°C".to_string()),
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
                trace: "test".to_string(),
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
                    url: "https://track.example.com/1".to_string(),
                    title: "Package".to_string(),
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
    fn test_format_search_markdown_mixed() {
        let response = SearchResponse {
            meta: Meta {
                trace: "test".to_string(),
                node: None,
                ms: None,
            },
            data: SearchData {
                search: Some(vec![SearchResult {
                    url: "https://example.com".to_string(),
                    title: "Example".to_string(),
                    snippet: Some("This is an example.".to_string()),
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
                    url: "".to_string(),
                    title: "".to_string(),
                    snippet: Some("Sunny, 25C".to_string()),
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
    fn test_format_extract_markdown_success() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_string(),
                node: None,
                ms: None,
            },
            data: Some(vec![ExtractData {
                url: "https://example.com".to_string(),
                markdown: Some("# Hello\nWorld".to_string()),
            }]),
            errors: None,
        };

        let expected = "## Extracted Content\n\n### https://example.com\n\n# Hello\nWorld\n\n---";
        assert_eq!(format_extract_markdown(&response), expected);
    }

    #[test]
    fn test_format_extract_markdown_failure() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_string(),
                node: None,
                ms: None,
            },
            data: None,
            errors: Some(vec![ExtractError {
                url: "https://example.com".to_string(),
                code: "404".to_string(),
                message: Some("Not Found".to_string()),
            }]),
        };

        let expected =
            "## Extracted Content\n\n### https://example.com\n\n**Extraction failed:** Not Found";
        assert_eq!(format_extract_markdown(&response), expected);
    }

    #[test]
    fn test_format_json() {
        let data = serde_json::json!({
            "key": "value"
        });
        let expected = "{\n  \"key\": \"value\"\n}";
        assert_eq!(format_json(&data), expected);
    }
}
