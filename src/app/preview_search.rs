use crate::app::state::{PreviewDocument, PreviewSearch};
use regex::Regex;

pub fn find_all_matches(
    doc: &PreviewDocument,
    query: &str,
    case_sensitive: bool,
) -> Vec<(usize, usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }

    let pattern = if case_sensitive {
        Regex::new(&regex::escape(query)).ok()
    } else {
        Regex::new(&format!("(?i){}", regex::escape(query))).ok()
    };

    let Some(regex) = pattern else {
        return Vec::new();
    };

    let mut matches = Vec::new();
    let lines: Vec<&str> = doc.content_excerpt.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        for mat in regex.find_iter(line) {
            matches.push((line_idx, mat.start(), mat.end()));
        }
    }

    matches
}

pub fn update_search(
    doc: &PreviewDocument,
    query: &str,
    case_sensitive: bool,
) -> Option<PreviewSearch> {
    if query.is_empty() {
        return None;
    }

    let match_positions = find_all_matches(doc, query, case_sensitive);
    if match_positions.is_empty() {
        return None;
    }

    Some(PreviewSearch {
        query: query.to_string(),
        case_sensitive,
        current_match_index: 0,
        match_positions,
    })
}

pub fn next_match(search: &mut PreviewSearch) {
    if !search.match_positions.is_empty() {
        search.current_match_index = (search.current_match_index + 1) % search.match_positions.len();
    }
}

pub fn prev_match(search: &mut PreviewSearch) {
    if !search.match_positions.is_empty() {
        search.current_match_index = if search.current_match_index == 0 {
            search.match_positions.len() - 1
        } else {
            search.current_match_index - 1
        };
    }
}

pub fn match_count(search: &PreviewSearch) -> (usize, usize) {
    (search.current_match_index + 1, search.match_positions.len())
}
