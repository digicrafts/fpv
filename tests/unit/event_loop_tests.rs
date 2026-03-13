use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use fpv::app::preview_controller::refresh_preview;
use fpv::app::state::{NodeType, SessionState, TreeNode};
use fpv::config::keymap::default_keymap;
use fpv::highlight::syntax::HighlightContext;
use fpv::tui::event_loop::process_once;
use fpv::tui::preview_pane::preview_total_lines;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn diff_mode_toggle_refreshes_preview_in_same_loop_cycle() {
    let dir = tempdir().expect("create tempdir");
    let repo = dir.path();
    let file_path = repo.join("README.md");
    let run_git = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(repo)
            .status()
            .expect("run git");
        assert!(status.success(), "git command failed: {:?}", args);
    };

    run_git(&["init"]);
    run_git(&["config", "user.name", "fpv-tests"]);
    run_git(&["config", "user.email", "fpv-tests@example.com"]);
    fs::write(&file_path, "line 1\nline 2\n").expect("write base");
    run_git(&["add", "README.md"]);
    run_git(&["commit", "-m", "base"]);
    fs::write(&file_path, "line 1\nline 2 changed\n").expect("write modified");

    let mut nodes = vec![TreeNode {
        path: file_path,
        name: "README.md".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];
    let mut state = SessionState::new(repo.to_path_buf());
    state.selected_index = 0;
    state.update_selected_path(&nodes);

    let mut preview = refresh_preview(&mut state, &nodes, &HighlightContext::new(), 1024 * 1024);
    let bindings = default_keymap();
    assert!(!preview
        .line_changes
        .iter()
        .any(|change| change.is_some()));
    assert!(!state.preview_diff_mode);

    state.deferred_input_event = Some(Event::Key(KeyEvent::new(
        KeyCode::Char('d'),
        KeyModifiers::NONE,
    )));
    let (should_quit, should_refresh_preview, should_refresh_tree) = process_once(
        &mut state,
        &mut nodes,
        &bindings,
        preview_total_lines(&preview),
        40,
        &preview,
    )
    .expect("process key input");
    assert!(!should_quit);
    assert!(!should_refresh_tree);
    assert!(should_refresh_preview);
    assert!(state.preview_diff_mode);

    preview = refresh_preview(&mut state, &nodes, &HighlightContext::new(), 1024 * 1024);
    assert!(preview
        .line_changes
        .iter()
        .any(|change| change.is_some()));
}
