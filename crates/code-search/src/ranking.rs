use crate::tantivy_index::SearchResult;

/// Re-rank search results using a simple heuristic scoring model.
///
/// Boosting factors (in priority order):
/// 1. Symbol match in path (e.g., query word appears in file name)
/// 2. Recent modification (requires last_modified metadata — applied externally)
/// 3. Score from the underlying full-text index (already encoded in `score`)
///
/// The re-ranker mutates the scores in-place and sorts the results.
pub fn rerank(results: &mut Vec<SearchResult>, query: &str) {
    let query_terms: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();

    for result in results.iter_mut() {
        let path_lower = result.path.to_lowercase();
        let mut boost = 1.0f32;

        // Boost if a query term appears in the file path.
        for term in &query_terms {
            if path_lower.contains(term.as_str()) {
                boost += 0.5;
            }
        }

        // Boost test files slightly lower (they are usually secondary context).
        if path_lower.contains("test") || path_lower.contains("spec") {
            boost *= 0.9;
        }

        result.score *= boost;
    }

    // Sort by score descending.
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(path: &str, score: f32) -> SearchResult {
        SearchResult {
            path: path.to_string(),
            start_line: 1,
            end_line: 10,
            snippet: String::new(),
            score,
        }
    }

    #[test]
    fn test_path_boost() {
        let mut results = vec![
            make_result("src/auth/session.rs", 1.0),
            make_result("src/util.rs", 1.0),
        ];
        rerank(&mut results, "session");
        assert_eq!(results[0].path, "src/auth/session.rs");
    }
}
