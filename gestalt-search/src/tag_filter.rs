use gestalt_core::ports::outbound::search::SearchResult;

/// Checks if a result contains a specific tag formatted as [tag] in its content, snippet, or path.
pub fn has_tag(result: &SearchResult, tag: &str) -> bool {
    let formatted_tag = format!("[{}]", tag);
    result.content.contains(&formatted_tag)
        || result.snippet.contains(&formatted_tag)
        || result.path.contains(&formatted_tag)
}

/// Filters a vector of search results, keeping only those that contain all required tags.
pub fn filter_by_tags(results: Vec<SearchResult>, required_tags: &[&str]) -> Vec<SearchResult> {
    if required_tags.is_empty() {
        return results;
    }

    results
        .into_iter()
        .filter(|result| required_tags.iter().all(|tag| has_tag(result, tag)))
        .collect()
}

/// Boosts the score of search results that contain specific tags.
/// Results are multiplied by the boost_factor for each matching tag, then sorted descending by score.
pub fn boost_by_tags(
    mut results: Vec<SearchResult>,
    boost_tags: &[&str],
    boost_factor: f64,
) -> Vec<SearchResult> {
    if boost_tags.is_empty() || boost_factor == 1.0 {
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        return results;
    }

    for result in &mut results {
        for tag in boost_tags {
            if has_tag(result, tag) {
                result.score *= boost_factor;
            }
        }
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

/// Extracts tags in the format [tag] from a string (e.g., query).
pub fn extract_tags(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut current_tag = String::new();
    let mut in_tag = false;

    for c in text.chars() {
        if c == '[' {
            in_tag = true;
            current_tag.clear();
        } else if c == ']' && in_tag {
            in_tag = false;
            if !current_tag.is_empty() {
                tags.push(current_tag.clone());
            }
        } else if in_tag {
            current_tag.push(c);
        }
    }

    tags
}
