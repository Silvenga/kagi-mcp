use crate::tools::search::group::extract_group_key;
use kagi_api::{SearchData, SearchResult};
use std::collections::HashMap;

pub fn dedup_by_domain(data: &mut SearchData, limit_per_domain: u32, final_limit: u32) {
    let dedup_vec = |vec: &mut Option<Vec<SearchResult>>| {
        if let Some(results) = vec {
            let mut counts: HashMap<String, u32> = HashMap::new();
            let mut kept: Vec<SearchResult> = Vec::new();
            for result in results.drain(..) {
                match extract_group_key(&result) {
                    Some(key) => {
                        let count = counts.entry(key).or_insert(0);
                        if *count < limit_per_domain {
                            *count += 1;
                            kept.push(result);
                        }
                    }
                    None => kept.push(result),
                }
            }
            kept.truncate(final_limit as usize);
            *results = kept;
        }
    };

    dedup_vec(&mut data.search);
    dedup_vec(&mut data.news);
    dedup_vec(&mut data.interesting_news);
    dedup_vec(&mut data.interesting_finds);
    dedup_vec(&mut data.code);
    dedup_vec(&mut data.public_records);
    dedup_vec(&mut data.listicle);
    dedup_vec(&mut data.web_archive);
    dedup_vec(&mut data.image);
    dedup_vec(&mut data.video);
    dedup_vec(&mut data.podcast);
    dedup_vec(&mut data.podcast_creator);
    // The following categories are intentionally skipped because they do not
    // represent ranked web results that benefit from per-domain deduplication:
    // adjacent_question, direct_answer, infobox, related_search, weather,
    // package_tracking.
}
