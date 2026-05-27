use super::ellipsis::collapse_snippet_ellipses;
use super::text_helpers::{decode_entities, normalize_title_whitespace, trim_iso_date};
use crate::format::FormatError;
use askama::Template;
use kagi_api::{SearchResponse, SearchResult};

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
    paywalled: bool,
    ai_content: Option<String>,
    language: Option<String>,
    duration: Option<String>,
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

fn extract_bool_prop(props: &Option<serde_json::Value>, key: &str) -> Option<bool> {
    props
        .as_ref()
        .and_then(|p| p.get(key))
        .and_then(|v| v.as_bool())
}

fn extract_string_prop(props: &Option<serde_json::Value>, key: &str) -> Option<String> {
    props
        .as_ref()
        .and_then(|p| p.get(key))
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned())
}

pub fn format_search_markdown(response: &SearchResponse) -> Result<String, FormatError> {
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
                        .map(|(i, r)| {
                            let paywalled =
                                extract_bool_prop(&r.props, "paywalled").unwrap_or(false);

                            let ai_content =
                                if extract_bool_prop(&r.props, "ai_generated") == Some(true) {
                                    Some("generated".to_owned())
                                } else if extract_bool_prop(&r.props, "ai_possible") == Some(true) {
                                    Some("possibly AI-generated".to_owned())
                                } else {
                                    None
                                };

                            let language = extract_string_prop(&r.props, "language")
                                .filter(|lang| lang != "en");

                            let is_media = title == "Videos"
                                || title == "Podcasts"
                                || title == "Podcast Creators";
                            let duration = if is_media {
                                extract_string_prop(&r.props, "duration")
                            } else {
                                None
                            };

                            GeneralItem {
                                index: i + 1,
                                title: decode_entities(&normalize_title_whitespace(&r.title)),
                                url: r.url.clone(),
                                snippet: r
                                    .snippet
                                    .as_ref()
                                    .map(|s| decode_entities(&collapse_snippet_ellipses(s))),
                                time: r.time.as_ref().map(|t| trim_iso_date(t)),
                                paywalled,
                                ai_content,
                                language,
                                duration,
                            }
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

    template
        .render()
        .map_err(FormatError::TemplateError)
        .map(|s| s.trim_end().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::{Image, Meta, SearchData, SearchResult};

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

    macro_rules! assert_snapshot {
        ($value:expr) => {
            insta::assert_snapshot!($value.replace("\r\n", "\n"));
        };
    }

    #[test]
    fn when_search_data_is_empty_then_should_return_no_results() {
        let response = make_response(empty_search_data());
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_web_results_then_should_format_web_section() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("This is an example.".to_owned()),
                time: Some("2023-01-01".to_owned()),
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_result_missing_snippet_and_time_then_should_handle_gracefully() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: None,
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_images_then_should_format_images_section() {
        let response = make_response(SearchData {
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
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_image_with_missing_dimensions_then_should_use_question_mark() {
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
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_mixed_results_then_should_format_all_sections() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("This is an example.".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            weather: Some(vec![SearchResult {
                url: "".to_owned(),
                title: "".to_owned(),
                snippet: Some("Sunny, 25C".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_format_search_with_html_entities_in_title_then_should_decode() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Foo &amp; Bar &quot;baz&quot;".to_owned(),
                snippet: None,
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_format_search_with_html_entities_in_snippet_then_should_decode() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("It&#39;s great &amp; amazing.".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_format_search_with_iso_timestamp_in_time_then_should_render_date_only() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: None,
                time: Some("2024-03-15T10:30:00Z".to_owned()),
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_format_search_with_ellipsis_run_in_snippet_then_should_collapse() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("foo ... ... ... bar".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_all_8_web_categories_then_each_should_have_distinct_header() {
        let mk = |title: &str| SearchResult {
            url: "https://example.com".to_owned(),
            title: title.to_owned(),
            snippet: None,
            time: None,
            image: None,
            props: None,
        };

        let response = make_response(SearchData {
            search: Some(vec![mk("Web")]),
            news: Some(vec![mk("News")]),
            interesting_news: Some(vec![mk("Interesting News")]),
            interesting_finds: Some(vec![mk("Interesting Finds")]),
            code: Some(vec![mk("Code")]),
            public_records: Some(vec![mk("Public Records")]),
            listicle: Some(vec![mk("Listicle")]),
            web_archive: Some(vec![mk("Web Archive")]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_infobox_with_props_then_should_format_all() {
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
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_infobox_without_snippet_then_should_skip_snippet() {
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
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_infobox_without_props_then_should_skip_props() {
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
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_adjacent_question_then_should_format_related_questions() {
        let response = make_response(SearchData {
            adjacent_question: Some(vec![SearchResult {
                url: "https://example.com/answer".to_owned(),
                title: "Answer Page".to_owned(),
                snippet: Some("The answer is 42.".to_owned()),
                time: None,
                image: None,
                props: Some(serde_json::json!({"question": "What is the meaning of life?"})),
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_adjacent_question_without_question_prop_then_should_fallback() {
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
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_direct_answer_then_should_format() {
        let response = make_response(SearchData {
            direct_answer: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Answer".to_owned(),
                snippet: Some("The direct answer is 42.".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_direct_answer_with_multiple_results_then_should_show_all() {
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
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_weather_then_should_format() {
        let response = make_response(SearchData {
            weather: Some(vec![SearchResult {
                url: "".to_owned(),
                title: "".to_owned(),
                snippet: Some("Sunny, 25°C".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_weather_with_multiple_entries_then_should_show_all() {
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
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_package_tracking_then_should_format() {
        let response = make_response(SearchData {
            package_tracking: Some(vec![SearchResult {
                url: "https://track.example.com/1".to_owned(),
                title: "Package".to_owned(),
                snippet: None,
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_related_searches_then_should_format() {
        let response = make_response(SearchData {
            related_search: Some(vec![SearchResult {
                url: "".to_owned(),
                title: "Related Topic".to_owned(),
                snippet: None,
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_news_without_time_then_should_omit_published() {
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
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_has_listicle_without_snippet_then_should_omit_snippet() {
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
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_extract_bool_prop_with_non_bool_value_then_should_return_none() {
        let props = Some(serde_json::json!({"paywalled": "true"}));
        assert_eq!(extract_bool_prop(&props, "paywalled"), None);

        let props = Some(serde_json::json!({"paywalled": 1}));
        assert_eq!(extract_bool_prop(&props, "paywalled"), None);

        let props = Some(serde_json::json!({"paywalled": null}));
        assert_eq!(extract_bool_prop(&props, "paywalled"), None);

        let props = Some(serde_json::json!({"paywalled": [true]}));
        assert_eq!(extract_bool_prop(&props, "paywalled"), None);

        let props = Some(serde_json::json!({"paywalled": {"val": true}}));
        assert_eq!(extract_bool_prop(&props, "paywalled"), None);
    }

    #[test]
    fn when_extract_bool_prop_with_bool_value_then_should_return_some() {
        let props = Some(serde_json::json!({"paywalled": true}));
        assert_eq!(extract_bool_prop(&props, "paywalled"), Some(true));

        let props = Some(serde_json::json!({"paywalled": false}));
        assert_eq!(extract_bool_prop(&props, "paywalled"), Some(false));
    }

    #[test]
    fn when_extract_bool_prop_with_none_or_missing_key_then_should_return_none() {
        assert_eq!(extract_bool_prop(&None, "paywalled"), None);

        let props = Some(serde_json::json!({"other": true}));
        assert_eq!(extract_bool_prop(&props, "paywalled"), None);
    }

    #[test]
    fn when_extract_string_prop_with_non_string_value_then_should_return_none() {
        let props = Some(serde_json::json!({"lang": true}));
        assert_eq!(extract_string_prop(&props, "lang"), None);

        let props = Some(serde_json::json!({"lang": 123}));
        assert_eq!(extract_string_prop(&props, "lang"), None);

        let props = Some(serde_json::json!({"lang": null}));
        assert_eq!(extract_string_prop(&props, "lang"), None);

        let props = Some(serde_json::json!({"lang": ["en"]}));
        assert_eq!(extract_string_prop(&props, "lang"), None);

        let props = Some(serde_json::json!({"lang": {"code": "en"}}));
        assert_eq!(extract_string_prop(&props, "lang"), None);
    }

    #[test]
    fn when_extract_string_prop_with_string_value_then_should_return_some() {
        let props = Some(serde_json::json!({"lang": "fr"}));
        assert_eq!(extract_string_prop(&props, "lang"), Some("fr".to_owned()));
    }

    #[test]
    fn when_extract_string_prop_with_none_or_missing_key_then_should_return_none() {
        assert_eq!(extract_string_prop(&None, "lang"), None);

        let props = Some(serde_json::json!({"other": "fr"}));
        assert_eq!(extract_string_prop(&props, "lang"), None);
    }

    #[test]
    fn when_search_has_podcast_creator_then_should_format() {
        let response = make_response(SearchData {
            podcast_creator: Some(vec![SearchResult {
                url: "https://example.com/creator1".to_owned(),
                title: "Creator One".to_owned(),
                snippet: Some("A great podcast creator.".to_owned()),
                time: Some("2024-01-15".to_owned()),
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_result_has_paywalled_true_then_should_render_paywalled() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("This is an example.".to_owned()),
                time: None,
                image: None,
                props: Some(serde_json::json!({"paywalled": true})),
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_result_has_paywalled_false_then_should_not_render_paywalled() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("This is an example.".to_owned()),
                time: None,
                image: None,
                props: Some(serde_json::json!({"paywalled": false})),
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_result_has_paywalled_missing_then_should_not_render_paywalled() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("This is an example.".to_owned()),
                time: None,
                image: None,
                props: None,
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_result_has_ai_generated_true_then_should_render_generated() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("This is an example.".to_owned()),
                time: None,
                image: None,
                props: Some(serde_json::json!({"ai_generated": true})),
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_result_has_ai_possible_true_then_should_render_possibly_ai() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("This is an example.".to_owned()),
                time: None,
                image: None,
                props: Some(serde_json::json!({"ai_possible": true})),
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_result_has_both_ai_flags_then_generated_takes_precedence() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("This is an example.".to_owned()),
                time: None,
                image: None,
                props: Some(serde_json::json!({"ai_generated": true, "ai_possible": true})),
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_podcast_has_duration_then_should_render_duration() {
        let response = make_response(SearchData {
            podcast: Some(vec![SearchResult {
                url: "https://example.com/podcast".to_owned(),
                title: "Great Podcast".to_owned(),
                snippet: Some("An amazing episode.".to_owned()),
                time: None,
                image: None,
                props: Some(serde_json::json!({"duration": "1:10:14"})),
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_video_has_duration_then_should_render_duration() {
        let response = make_response(SearchData {
            video: Some(vec![SearchResult {
                url: "https://example.com/video".to_owned(),
                title: "Great Video".to_owned(),
                snippet: Some("An amazing video.".to_owned()),
                time: None,
                image: None,
                props: Some(serde_json::json!({"duration": "45:30"})),
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_result_has_non_english_language_then_should_render_language() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("This is an example.".to_owned()),
                time: None,
                image: None,
                props: Some(serde_json::json!({"language": "ja"})),
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_result_has_english_language_then_should_not_render_language() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("This is an example.".to_owned()),
                time: None,
                image: None,
                props: Some(serde_json::json!({"language": "en"})),
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_search_result_has_multiple_props_then_should_render_all() {
        let response = make_response(SearchData {
            search: Some(vec![SearchResult {
                url: "https://example.com".to_owned(),
                title: "Example".to_owned(),
                snippet: Some("This is an example.".to_owned()),
                time: None,
                image: None,
                props: Some(serde_json::json!({
                    "paywalled": true,
                    "ai_generated": true,
                    "language": "fr"
                })),
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }

    #[test]
    fn when_news_result_has_paywalled_and_language_then_should_render_both() {
        let response = make_response(SearchData {
            news: Some(vec![SearchResult {
                url: "https://example.com/story".to_owned(),
                title: "Breaking Story".to_owned(),
                snippet: Some("Latest updates.".to_owned()),
                time: Some("2024-06-01".to_owned()),
                image: None,
                props: Some(serde_json::json!({
                    "paywalled": true,
                    "language": "de"
                })),
            }]),
            ..empty_search_data()
        });
        assert_snapshot!(format_search_markdown(&response).unwrap());
    }
}
