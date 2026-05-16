use askama::Template;
use kagi_api::{ExtractResponse, SearchResponse, SearchResult};

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

#[derive(Template)]
#[template(path = "search_results.md", escape = "none")]
struct SearchResultsTemplate {
    general_sections: Vec<GeneralSection>,
    image_results: Vec<ImageItem>,
    related_questions: Vec<RelatedQuestionItem>,
    direct_answers: Vec<DirectAnswerItem>,
    infoboxes: Vec<InfoboxItem>,
    related_searches: Vec<RelatedSearchItem>,
    weather: Vec<WeatherItem>,
    package_tracking: Vec<PackageTrackingItem>,
    has_results: bool,
}

struct GeneralSection {
    title: String,
    items: Vec<GeneralItem>,
}

struct GeneralItem {
    index: usize,
    title: String,
    url: String,
    snippet: Option<String>,
    time: Option<String>,
}

struct ImageItem {
    index: usize,
    title: String,
    url: String,
    image_url: String,
    width: String,
    height: String,
}

struct RelatedQuestionItem {
    index: usize,
    question: String,
    url: String,
    snippet: Option<String>,
}

struct DirectAnswerItem {
    snippet: String,
}

struct InfoboxItem {
    title: String,
    url: String,
    snippet: Option<String>,
    properties: Vec<(String, String)>,
}

struct RelatedSearchItem {
    title: String,
}

struct WeatherItem {
    snippet: String,
}

struct PackageTrackingItem {
    url: String,
}

pub fn format_search_markdown(response: &SearchResponse) -> String {
    let data = &response.data;
    let mut has_results = false;

    let mut general_sections = Vec::new();

    let mut add_general = |title: &str, results: &Option<Vec<SearchResult>>| {
        if let Some(results) = results.as_ref() {
            if !results.is_empty() {
                has_results = true;
                general_sections.push(GeneralSection {
                    title: title.to_owned(),
                    items: results
                        .iter()
                        .enumerate()
                        .map(|(i, r)| GeneralItem {
                            index: i + 1,
                            title: decode_entities(&normalize_title_whitespace(&r.title)),
                            url: r.url.clone(),
                            snippet: r
                                .snippet
                                .as_ref()
                                .map(|s| decode_entities(&collapse_snippet_ellipses(s))),
                            time: r.time.as_ref().map(|t| trim_iso_date(t)),
                        })
                        .collect(),
                });
            }
        }
    };

    add_general("Web Results", &data.search);
    add_general("News", &data.news);
    add_general("Interesting News", &data.interesting_news);
    add_general("Interesting Finds", &data.interesting_finds);
    add_general("Code Results", &data.code);
    add_general("Public Records", &data.public_records);
    add_general("Listicles", &data.listicle);
    add_general("Web Archive", &data.web_archive);
    add_general("Videos", &data.video);
    add_general("Podcasts", &data.podcast);
    add_general("Podcast Creators", &data.podcast_creator);

    let image_results = match &data.image {
        Some(results) if !results.is_empty() => {
            has_results = true;
            results
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let width = r
                        .image
                        .as_ref()
                        .and_then(|img| img.width)
                        .map_or_else(|| "?".to_owned(), |w| w.to_string());
                    let height = r
                        .image
                        .as_ref()
                        .and_then(|img| img.height)
                        .map_or_else(|| "?".to_owned(), |h| h.to_string());
                    ImageItem {
                        index: i + 1,
                        title: decode_entities(&normalize_title_whitespace(&r.title)),
                        url: r.url.clone(),
                        image_url: r
                            .image
                            .as_ref()
                            .map_or(String::new(), |img| img.url.clone()),
                        width,
                        height,
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    };

    let related_questions = match &data.adjacent_question {
        Some(results) if !results.is_empty() => {
            has_results = true;
            results
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let question = r
                        .props
                        .as_ref()
                        .and_then(|p| p.get("question"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown Question");
                    RelatedQuestionItem {
                        index: i + 1,
                        question: decode_entities(&normalize_title_whitespace(question)),
                        url: r.url.clone(),
                        snippet: r
                            .snippet
                            .as_ref()
                            .map(|s| decode_entities(&collapse_snippet_ellipses(s))),
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    };

    let direct_answers = match &data.direct_answer {
        Some(results) if !results.is_empty() => {
            has_results = true;
            results
                .iter()
                .filter_map(|r| {
                    r.snippet.as_ref().map(|s| DirectAnswerItem {
                        snippet: decode_entities(&collapse_snippet_ellipses(s)),
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    };

    let infoboxes = match &data.infobox {
        Some(results) if !results.is_empty() => {
            has_results = true;
            results
                .iter()
                .map(|r| {
                    let mut properties = Vec::new();
                    if let Some(props) = &r.props {
                        if let Some(infobox) = props.get("infobox") {
                            if let Some(obj) = infobox.as_object() {
                                for (key, value) in obj {
                                    let val_str = value
                                        .as_str()
                                        .map(|s| s.to_owned())
                                        .unwrap_or_else(|| value.to_string());
                                    properties.push((key.clone(), val_str));
                                }
                            }
                        }
                    }
                    InfoboxItem {
                        title: decode_entities(&normalize_title_whitespace(&r.title)),
                        url: r.url.clone(),
                        snippet: r
                            .snippet
                            .as_ref()
                            .map(|s| decode_entities(&collapse_snippet_ellipses(s))),
                        properties,
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    };

    let related_searches = match &data.related_search {
        Some(results) if !results.is_empty() => {
            has_results = true;
            results
                .iter()
                .map(|r| RelatedSearchItem {
                    title: decode_entities(&normalize_title_whitespace(&r.title)),
                })
                .collect()
        }
        _ => Vec::new(),
    };

    let weather = match &data.weather {
        Some(results) if !results.is_empty() => {
            has_results = true;
            results
                .iter()
                .filter_map(|r| {
                    r.snippet.as_ref().map(|s| WeatherItem {
                        snippet: decode_entities(&collapse_snippet_ellipses(s)),
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    };

    let package_tracking = match &data.package_tracking {
        Some(results) if !results.is_empty() => {
            has_results = true;
            results
                .iter()
                .map(|r| PackageTrackingItem { url: r.url.clone() })
                .collect()
        }
        _ => Vec::new(),
    };

    let template = SearchResultsTemplate {
        general_sections,
        image_results,
        related_questions,
        direct_answers,
        infoboxes,
        related_searches,
        weather,
        package_tracking,
        has_results,
    };

    template.render().unwrap().trim_end().to_owned()
}

#[derive(Template)]
#[template(path = "extract_results.md", escape = "none")]
struct ExtractResultsTemplate {
    has_content: bool,
    data_items: Vec<ExtractDataItem>,
    error_items: Vec<ExtractErrorItem>,
}

struct ExtractDataItem {
    url: String,
    has_markdown: bool,
    markdown: String,
}

struct ExtractErrorItem {
    url: String,
    message: String,
}

pub fn format_extract_markdown(response: &ExtractResponse) -> String {
    let mut data_items = Vec::new();
    let mut error_items = Vec::new();
    let mut has_content = false;

    if let Some(data) = &response.data {
        for item in data {
            has_content = true;
            let markdown = item.markdown.clone();
            let has_markdown = markdown.is_some();
            data_items.push(ExtractDataItem {
                url: item.url.clone(),
                has_markdown,
                markdown: markdown.unwrap_or_default(),
            });
        }
    }

    if let Some(errors) = &response.errors {
        for error in errors {
            has_content = true;
            error_items.push(ExtractErrorItem {
                url: error.url.clone(),
                message: error
                    .message
                    .clone()
                    .unwrap_or_else(|| "Unknown error".to_owned()),
            });
        }
    }

    let template = ExtractResultsTemplate {
        has_content,
        data_items,
        error_items,
    };

    template.render().unwrap().trim_end().to_owned()
}

pub fn format_json<T: serde::Serialize>(response: &T) -> String {
    serde_json::to_string_pretty(response)
        .unwrap_or_else(|e| format!("JSON serialization error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::{ExtractData, ExtractError, Image, Meta, SearchData};

    #[test]
    fn when_search_data_is_empty_then_should_return_no_results() {
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
    fn when_search_has_web_results_then_should_format_web_section() {
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
    fn when_search_has_images_then_should_format_images_section() {
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
    fn when_search_result_missing_snippet_and_time_then_should_handle_gracefully() {
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
    fn when_search_has_mixed_results_then_should_format_all_sections() {
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
    fn when_extract_succeeds_then_should_format_content() {
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
    fn when_extract_fails_then_should_format_error() {
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
    fn when_format_is_json_then_should_serialize_correctly() {
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

    // ---------------------------------------------------------------------------
    // Helpers for golden tests
    // ---------------------------------------------------------------------------

    fn empty_search_data() -> SearchData {
        SearchData {
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
        }
    }

    fn make_response(data: SearchData) -> SearchResponse {
        SearchResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data,
        }
    }

    // ---------------------------------------------------------------------------
    // Fixture-based golden tests
    // ---------------------------------------------------------------------------

    #[test]
    fn when_using_search_response_fixture_then_markdown_should_match_golden() {
        let response: SearchResponse = serde_json::from_str(include_str!(
            "../../docs/test-fixtures/search-response.json"
        ))
        .expect("valid fixture JSON");
        let expected = r##"## Web Results

1. **[Rust Programming Language](https://rust-lang.org/)**
   - Snippet: Rust is blazingly fast and memory-efficient: with no runtime or garbage collector, it can power performance-critical services, run on embedded devices, and ... Rust is a fast, reliable, and productive programming language that can run on embedded devices, web services, and more. Learn how to get started, why Rust is different, and what companies are using it in production. ... A language empowering everyone to build reliable and efficient software.In 2018, the Rust community decided to improve the programming experience for a few distinct domains (see the 2018 roadmap). For these, you can find many high-quality crates and some awesome guides on how to get started.
   - Published: 2011-06-06
2. **[Learn](https://www.rust-lang.org/learn)**
   - Snippet: Learn Rust Get started with Rust Affectionately nicknamed “the book,” The Rust Programming Language will give you an overview of the language from first principles. You’ll build a few projects along the way, and by the end, you’ll have a solid grasp of the language. Read the Book! ... Affectionately nicknamed “the book,” The Rust Programming Language will give you an overview of the language from first principles. You’ll build a few projects along the way, and by the end, you’ll have a solid grasp of the language.If reading multiple hundreds of pages about a language isn’t your style, then Rust By Example has you covered. ... Find out how to get started with Rust, a language for reliable and efficient software. Explore the book, the documentation, the courses, and the application domains for Rust.
   - Published: 2014-05-22"##;
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_using_extract_response_fixture_then_markdown_should_match_golden() {
        let response: ExtractResponse = serde_json::from_str(include_str!(
            "../../docs/test-fixtures/extract-response.json"
        ))
        .expect("valid fixture JSON");
        let expected = r##"## Extracted Content

### https://www.rust-lang.org

### Performance

 Rust is blazingly fast and memory-efficient: with no runtime or garbage collector, it can power performance-critical services, run on embedded devices, and easily integrate with other languages.

### Reliability

 Rust’s rich type system and ownership model guarantee memory-safety and thread-safety — enabling you to eliminate many classes of bugs at compile-time.

---"##;
        assert_eq!(format_extract_markdown(&response), expected);
    }

    #[test]
    fn when_using_search_with_group_id_fixture_then_markdown_should_match_golden() {
        let response: SearchResponse = serde_json::from_str(include_str!(
            "../tests/fixtures/search_response_with_group_id.json"
        ))
        .expect("valid fixture JSON");
        let expected = r##"## Web Results

1. **[Learn Rust - Rust Programming Language](https://www.rust-lang.org/learn)**
   - Snippet: A comprehensive guide to learning Rust, from beginner to advanced. Includes the official book, rustlings exercises, and 'many' community resources.
   - Published: 2025-12-01
2. **[Install Rust - Rust Programming Language](https://www.rust-lang.org/install)**
   - Snippet: Install rustup, the official Rust toolchain installer. Works on Linux, macOS, and Windows.
   - Published: 2025-11-15
3. **[The Rust Programming Language Book](https://doc.rust-lang.org/book/)**
   - Snippet: The official guide to Rust, covering ownership, types, generics, concurrency, and more. Often called 'the book' by the community.
   - Published: 2025-10-20
4. **[Rust (programming language) - Wikipedia](https://en.wikipedia.org/wiki/Rust_(programming_language))**
   - Snippet: Rust is a multi-paradigm, general-purpose programming language emphasizing performance, type safety, and concurrency.
   - Published: 2025-12-10

## Videos

1. **[Rust Crash Course | Rust Programming Tutorial for Beginners](https://www.youtube.com/watch?v=ygL_xcavzX4)**
   - Snippet: A complete Rust tutorial covering variables, ownership, structs, enums, error handling, and more in a single video.
   - Published: 2025-09-14
2. **[Rust for Beginners - Learn Rust in 2 Hours](https://www.youtube.com/watch?v=MsocPEZBd-M)**
   - Snippet: A fast-paced introduction to Rust for programmers coming from Python, JavaScript, or Go.
   - Published: 2025-08-22

## Related Questions

1. **What are the downsides of using Rust?**
    - [Answer](https://www.rust-lang.org/faq): Answers to common questions about Rust, including its design decisions, use cases, and 'community guidelines'."##;
        assert_eq!(format_search_markdown(&response), expected);
    }

    // ---------------------------------------------------------------------------
    // Section golden tests
    // ---------------------------------------------------------------------------

    #[test]
    fn when_search_has_interesting_news_then_markdown_should_match_golden() {
        let response = make_response(SearchData {
            interesting_news: Some(vec![SearchResult {
                url: "https://example.com/inews".to_owned(),
                title: "Interesting News Item".to_owned(),
                snippet: Some("A noteworthy event.".to_owned()),
                time: Some("2024-07-01".to_owned()),
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        let expected =
            "## Interesting News\n\n1. **[Interesting News Item](https://example.com/inews)**\n   - Snippet: A noteworthy event.\n   - Published: 2024-07-01";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_interesting_finds_then_markdown_should_match_golden() {
        let response = make_response(SearchData {
            interesting_finds: Some(vec![SearchResult {
                url: "https://example.com/find".to_owned(),
                title: "Interesting Find".to_owned(),
                snippet: Some("A curious discovery.".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        let expected =
            "## Interesting Finds\n\n1. **[Interesting Find](https://example.com/find)**\n   - Snippet: A curious discovery.";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_public_records_then_markdown_should_match_golden() {
        let response = make_response(SearchData {
            public_records: Some(vec![SearchResult {
                url: "https://example.com/record".to_owned(),
                title: "Public Record".to_owned(),
                snippet: Some("A public record entry.".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        let expected =
            "## Public Records\n\n1. **[Public Record](https://example.com/record)**\n   - Snippet: A public record entry.";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_web_archive_then_markdown_should_match_golden() {
        let response = make_response(SearchData {
            web_archive: Some(vec![SearchResult {
                url: "https://web.archive.org/web/2024/example".to_owned(),
                title: "Archived Page".to_owned(),
                snippet: Some("A snapshot of a web page.".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        let expected =
            "## Web Archive\n\n1. **[Archived Page](https://web.archive.org/web/2024/example)**\n   - Snippet: A snapshot of a web page.";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_proper_infobox_golden_then_markdown_should_match_exact() {
        let response = make_response(SearchData {
            infobox: Some(vec![SearchResult {
                url: "https://example.com/entity".to_owned(),
                title: "Entity Name".to_owned(),
                snippet: Some("A well-known entity.".to_owned()),
                time: None,
                image: None,
                props: Some(serde_json::json!({
                    "infobox": {
                        "Population": "1.4B",
                        "Capital": "Beijing",
                        "Founded": "1949"
                    }
                })),
            }]),
            ..empty_search_data()
        });
        let expected = "## Infobox\n\n**[Entity Name](https://example.com/entity)**\n\nA well-known entity.\n\nCapital: Beijing\nFounded: 1949\nPopulation: 1.4B";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_infobox_without_snippet_then_markdown_should_skip_snippet() {
        let response = make_response(SearchData {
            infobox: Some(vec![SearchResult {
                url: "https://example.com/place".to_owned(),
                title: "Place Name".to_owned(),
                snippet: None,
                time: None,
                image: None,
                props: Some(serde_json::json!({
                    "infobox": {
                        "Population": "8M",
                        "Country": "USA"
                    }
                })),
            }]),
            ..empty_search_data()
        });
        let expected = "## Infobox\n\n**[Place Name](https://example.com/place)**\n\nCountry: USA\nPopulation: 8M";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_infobox_without_infobox_props_then_markdown_should_skip_props() {
        let response = make_response(SearchData {
            infobox: Some(vec![SearchResult {
                url: "https://example.com/plain".to_owned(),
                title: "Plain Info".to_owned(),
                snippet: Some("Just text.".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        let expected = "## Infobox\n\n**[Plain Info](https://example.com/plain)**\n\nJust text.";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_image_with_missing_dimensions_then_markdown_should_use_question_mark() {
        let response = make_response(SearchData {
            image: Some(vec![SearchResult {
                url: "https://example.com/page".to_owned(),
                title: "Image No Size".to_owned(),
                snippet: None,
                time: None,
                image: Some(Image {
                    url: "https://example.com/img.jpg".to_owned(),
                    width: None,
                    height: None,
                }),
                props: None,
            }]),
            ..empty_search_data()
        });
        let expected =
            "## Images\n\n1. **[Image No Size](https://example.com/page)**\n   - Image: https://example.com/img.jpg (?x?)";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_direct_answer_with_multiple_results_then_markdown_should_show_all() {
        let response = make_response(SearchData {
            direct_answer: Some(vec![
                SearchResult {
                    url: "https://example.com/a1".to_owned(),
                    title: "Answer 1".to_owned(),
                    snippet: Some("First direct answer.".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                },
                SearchResult {
                    url: "https://example.com/a2".to_owned(),
                    title: "Answer 2".to_owned(),
                    snippet: Some("Second direct answer.".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                },
            ]),
            ..empty_search_data()
        });
        let expected = "## Direct Answer\n\nFirst direct answer.\n\nSecond direct answer.";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_adjacent_question_without_question_prop_then_markdown_should_fallback() {
        let response = make_response(SearchData {
            adjacent_question: Some(vec![SearchResult {
                url: "https://example.com/faq".to_owned(),
                title: "FAQ Page".to_owned(),
                snippet: Some("Some answer text.".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        let expected =
            "## Related Questions\n\n1. **Unknown Question**\n    - [Answer](https://example.com/faq): Some answer text.";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_podcast_creator_with_time_then_markdown_should_show_published() {
        let response = make_response(SearchData {
            podcast_creator: Some(vec![SearchResult {
                url: "https://example.com/creator2".to_owned(),
                title: "Creator Two".to_owned(),
                snippet: Some("Another creator.".to_owned()),
                time: Some("2024-03-01T08:00:00Z".to_owned()),
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        let expected =
            "## Podcast Creators\n\n1. **[Creator Two](https://example.com/creator2)**\n   - Snippet: Another creator.\n   - Published: 2024-03-01";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_code_without_snippet_then_markdown_should_skip_snippet_line() {
        let response = make_response(SearchData {
            code: Some(vec![SearchResult {
                url: "https://github.com/user/repo".to_owned(),
                title: "A Code Repository".to_owned(),
                snippet: None,
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        let expected =
            "## Code Results\n\n1. **[A Code Repository](https://github.com/user/repo)**";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_related_searches_with_multiple_entries_then_markdown_should_show_all() {
        let response = make_response(SearchData {
            related_search: Some(vec![
                SearchResult {
                    url: "".to_owned(),
                    title: "Search Term One".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                },
                SearchResult {
                    url: "".to_owned(),
                    title: "Search Term Two".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                },
            ]),
            ..empty_search_data()
        });
        let expected = "## Related Searches\n\n- Search Term One\n- Search Term Two";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_package_tracking_with_multiple_links_then_markdown_should_show_all() {
        let response = make_response(SearchData {
            package_tracking: Some(vec![
                SearchResult {
                    url: "https://track.example.com/pkg1".to_owned(),
                    title: "Package 1".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                },
                SearchResult {
                    url: "https://track.example.com/pkg2".to_owned(),
                    title: "Package 2".to_owned(),
                    snippet: None,
                    time: None,
                    image: None,
                    props: None,
                },
            ]),
            ..empty_search_data()
        });
        let expected =
            "## Package Tracking\n\n- [Tracking Link](https://track.example.com/pkg1)\n- [Tracking Link](https://track.example.com/pkg2)";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_news_without_time_then_markdown_should_omit_published() {
        let response = make_response(SearchData {
            news: Some(vec![SearchResult {
                url: "https://example.com/story".to_owned(),
                title: "Breaking Story".to_owned(),
                snippet: Some("Latest updates.".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        let expected =
            "## News\n\n1. **[Breaking Story](https://example.com/story)**\n   - Snippet: Latest updates.";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_listicle_without_snippet_then_markdown_should_omit_snippet() {
        let response = make_response(SearchData {
            listicle: Some(vec![SearchResult {
                url: "https://example.com/top10".to_owned(),
                title: "Top 10 List".to_owned(),
                snippet: None,
                time: Some("2024-05-01".to_owned()),
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        let expected =
            "## Listicles\n\n1. **[Top 10 List](https://example.com/top10)**\n   - Published: 2024-05-01";
        assert_eq!(format_search_markdown(&response), expected);
    }

    #[test]
    fn when_search_has_weather_with_multiple_entries_then_markdown_should_show_all() {
        let response = make_response(SearchData {
            weather: Some(vec![
                SearchResult {
                    url: "".to_owned(),
                    title: "".to_owned(),
                    snippet: Some("Monday: Sunny, 25°C".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                },
                SearchResult {
                    url: "".to_owned(),
                    title: "".to_owned(),
                    snippet: Some("Tuesday: Cloudy, 20°C".to_owned()),
                    time: None,
                    image: None,
                    props: None,
                },
            ]),
            ..empty_search_data()
        });
        let expected = "## Weather\n\nMonday: Sunny, 25°C\nTuesday: Cloudy, 20°C";
        assert_eq!(format_search_markdown(&response), expected);
    }

    // ---------------------------------------------------------------------------
    // Extract golden tests
    // ---------------------------------------------------------------------------

    #[test]
    fn when_extract_is_empty_then_markdown_should_be_no_content() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: None,
            errors: None,
        };
        assert_eq!(format_extract_markdown(&response), "No content extracted.");
    }

    #[test]
    fn when_extract_has_empty_arrays_then_markdown_should_be_no_content() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: Some(vec![]),
            errors: Some(vec![]),
        };
        assert_eq!(format_extract_markdown(&response), "No content extracted.");
    }

    #[test]
    fn when_extract_has_data_and_errors_then_markdown_should_include_both() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: Some(vec![ExtractData {
                url: "https://example.com/ok".to_owned(),
                markdown: Some("Successful content.".to_owned()),
            }]),
            errors: Some(vec![ExtractError {
                url: "https://example.com/bad".to_owned(),
                code: "500".to_owned(),
                message: Some("Server Error".to_owned()),
            }]),
        };
        let expected = "## Extracted Content\n\n### https://example.com/ok\n\nSuccessful content.\n\n---\n\n### https://example.com/bad\n\n**Extraction failed:** Server Error";
        assert_eq!(format_extract_markdown(&response), expected);
    }

    #[test]
    fn when_extract_error_without_message_then_markdown_should_use_fallback() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: None,
            errors: Some(vec![ExtractError {
                url: "https://example.com/err".to_owned(),
                code: "403".to_owned(),
                message: None,
            }]),
        };
        let expected =
            "## Extracted Content\n\n### https://example.com/err\n\n**Extraction failed:** Unknown error";
        assert_eq!(format_extract_markdown(&response), expected);
    }

    #[test]
    fn when_extract_has_multiple_data_items_then_markdown_should_show_all() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: Some(vec![
                ExtractData {
                    url: "https://example.com/a".to_owned(),
                    markdown: Some("Content A.".to_owned()),
                },
                ExtractData {
                    url: "https://example.com/b".to_owned(),
                    markdown: Some("Content B.".to_owned()),
                },
            ]),
            errors: None,
        };
        let expected = "## Extracted Content\n\n### https://example.com/a\n\nContent A.\n\n---\n\n### https://example.com/b\n\nContent B.\n\n---";
        assert_eq!(format_extract_markdown(&response), expected);
    }

    #[test]
    fn when_extract_data_has_no_markdown_then_markdown_should_skip_content() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: Some(vec![ExtractData {
                url: "https://example.com/empty".to_owned(),
                markdown: None,
            }]),
            errors: None,
        };
        let expected = "## Extracted Content\n\n### https://example.com/empty\n\n---";
        assert_eq!(format_extract_markdown(&response), expected);
    }
}
