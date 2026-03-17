use fpv::app::preview_search::{find_all_matches, match_count, next_match, prev_match, update_search};
use fpv::app::state::{LoadState, PreviewDocument, PreviewSearch};

#[test]
fn find_all_matches_case_insensitive() {
    let mut doc = PreviewDocument::default();
    doc.content_excerpt = "Hello hello HELLO\nworld World".to_string();
    doc.load_state = LoadState::Ready;

    let matches = find_all_matches(&doc, "hello", false);
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0], (0, 0, 5));
    assert_eq!(matches[1], (0, 6, 11));
    assert_eq!(matches[2], (0, 12, 17));
}

#[test]
fn find_all_matches_case_sensitive() {
    let mut doc = PreviewDocument::default();
    doc.content_excerpt = "Hello hello HELLO".to_string();
    doc.load_state = LoadState::Ready;

    let matches = find_all_matches(&doc, "hello", true);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0], (0, 6, 11));
}

#[test]
fn find_all_matches_multiline() {
    let mut doc = PreviewDocument::default();
    doc.content_excerpt = "foo bar\nfoo baz\nbar foo".to_string();
    doc.load_state = LoadState::Ready;

    let matches = find_all_matches(&doc, "foo", false);
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0], (0, 0, 3));
    assert_eq!(matches[1], (1, 0, 3));
    assert_eq!(matches[2], (2, 4, 7));
}

#[test]
fn update_search_returns_none_for_empty_query() {
    let doc = PreviewDocument::default();
    let result = update_search(&doc, "", false);
    assert!(result.is_none());
}

#[test]
fn update_search_returns_none_for_no_matches() {
    let mut doc = PreviewDocument::default();
    doc.content_excerpt = "hello world".to_string();
    let result = update_search(&doc, "xyz", false);
    assert!(result.is_none());
}

#[test]
fn update_search_initializes_correctly() {
    let mut doc = PreviewDocument::default();
    doc.content_excerpt = "foo foo foo".to_string();
    doc.load_state = LoadState::Ready;

    let search = update_search(&doc, "foo", false).unwrap();
    assert_eq!(search.query, "foo");
    assert_eq!(search.match_positions.len(), 3);
    assert_eq!(search.current_match_index, 0);
    assert!(!search.case_sensitive);
}

#[test]
fn next_match_cycles_through_matches() {
    let mut search = PreviewSearch {
        query: "test".to_string(),
        case_sensitive: false,
        current_match_index: 0,
        match_positions: vec![(0, 0, 4), (0, 5, 9), (1, 0, 4)],
    };

    next_match(&mut search);
    assert_eq!(search.current_match_index, 1);

    next_match(&mut search);
    assert_eq!(search.current_match_index, 2);

    next_match(&mut search);
    assert_eq!(search.current_match_index, 0);
}

#[test]
fn prev_match_cycles_backward() {
    let mut search = PreviewSearch {
        query: "test".to_string(),
        case_sensitive: false,
        current_match_index: 0,
        match_positions: vec![(0, 0, 4), (0, 5, 9), (1, 0, 4)],
    };

    prev_match(&mut search);
    assert_eq!(search.current_match_index, 2);

    prev_match(&mut search);
    assert_eq!(search.current_match_index, 1);
}

#[test]
fn match_count_returns_current_and_total() {
    let search = PreviewSearch {
        query: "test".to_string(),
        case_sensitive: false,
        current_match_index: 1,
        match_positions: vec![(0, 0, 4), (0, 5, 9), (1, 0, 4)],
    };

    let (current, total) = match_count(&search);
    assert_eq!(current, 2);
    assert_eq!(total, 3);
}
