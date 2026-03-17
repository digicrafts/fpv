use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use fpv::app::preview_controller::refresh_preview;
use fpv::app::state::{NodeType, SessionState, TreeNode};
use fpv::config::keymap::default_keymap;
use fpv::highlight::syntax::HighlightContext;
use fpv::tui::event_loop::process_once;
use fpv::tui::preview_pane::preview_total_lines;
use std::fs;
use tempfile::tempdir;

#[test]
fn search_activates_and_finds_matches() {
    let dir = tempdir().expect("create tempdir");
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "hello world\nhello there\nworld hello").expect("write file");

    let mut nodes = vec![TreeNode {
        path: file_path.clone(),
        name: "test.txt".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];

    let mut state = SessionState::new(dir.path().to_path_buf());
    state.selected_index = 0;
    state.update_selected_path(&nodes);

    let preview = refresh_preview(&mut state, &nodes, &HighlightContext::new(), 1024 * 1024);
    let bindings = default_keymap();

    state.deferred_input_event = Some(Event::Key(KeyEvent::new(
        KeyCode::Char('f'),
        KeyModifiers::NONE,
    )));

    let (_, _, _) = process_once(
        &mut state,
        &mut nodes,
        &bindings,
        preview_total_lines(&preview),
        20,
        &preview,
    )
    .expect("process search activation");

    assert!(state.preview_search_input_active);
    assert!(state.preview_search.is_some());
}

#[test]
fn search_typing_and_matching() {
    let dir = tempdir().expect("create tempdir");
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "hello\nworld\nhello again").expect("write file");

    let mut nodes = vec![TreeNode {
        path: file_path,
        name: "test.txt".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];

    let mut state = SessionState::new(dir.path().to_path_buf());
    state.selected_index = 0;
    state.update_selected_path(&nodes);

    let preview = refresh_preview(&mut state, &nodes, &HighlightContext::new(), 1024 * 1024);
    let bindings = default_keymap();

    state.preview_search_input_active = true;
    state.preview_search = Some(Default::default());

    state.deferred_input_event = Some(Event::Key(KeyEvent::new(
        KeyCode::Char('h'),
        KeyModifiers::NONE,
    )));

    process_once(
        &mut state,
        &mut nodes,
        &bindings,
        preview_total_lines(&preview),
        20,
        &preview,
    )
    .expect("process search typing");

    if let Some(search) = &state.preview_search {
        assert_eq!(search.query, "h");
    }

    state.deferred_input_event = Some(Event::Key(KeyEvent::new(
        KeyCode::Char('e'),
        KeyModifiers::NONE,
    )));

    process_once(
        &mut state,
        &mut nodes.clone(),
        &bindings,
        preview_total_lines(&preview),
        20,
        &preview,
    )
    .expect("process search typing");

    if let Some(search) = &state.preview_search {
        assert_eq!(search.query, "he");
    }
}

#[test]
fn search_dismiss_with_esc() {
    let dir = tempdir().expect("create tempdir");
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "hello").expect("write file");

    let mut nodes = vec![TreeNode {
        path: file_path,
        name: "test.txt".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];

    let mut state = SessionState::new(dir.path().to_path_buf());
    state.selected_index = 0;
    state.update_selected_path(&nodes);

    let preview = refresh_preview(&mut state, &nodes, &HighlightContext::new(), 1024 * 1024);
    let bindings = default_keymap();

    state.preview_search_input_active = true;
    state.preview_search = Some(Default::default());

    state.deferred_input_event = Some(Event::Key(KeyEvent::new(
        KeyCode::Esc,
        KeyModifiers::NONE,
    )));

    process_once(
        &mut state,
        &mut nodes,
        &bindings,
        preview_total_lines(&preview),
        20,
        &preview,
    )
    .expect("process escape");

    assert!(!state.preview_search_input_active);
    assert!(state.preview_search.is_none());
}
