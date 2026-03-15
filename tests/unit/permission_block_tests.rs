use fpv::app::navigation::enter_selected_directory;
use fpv::app::navigation_result::ActionOutcome;
use fpv::app::state::{NodeType, SessionState, TreeNode};
use tempfile::tempdir;

#[test]
fn unreadable_directory_is_blocked() {
    let d = tempdir().expect("tempdir");
    let private = d.path().join("private");
    std::fs::create_dir_all(&private).expect("mkdir");

    // Remove read/execute permissions to make it unreadable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    }

    let mut state = SessionState::new(d.path().to_path_buf());
    let mut nodes = vec![TreeNode {
        path: private.clone(),
        name: "private".into(),
        node_type: NodeType::Directory,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];
    let result = enter_selected_directory(&mut state, &mut nodes).expect("enter");
    assert_eq!(result.outcome, ActionOutcome::Blocked);

    // Restore permissions so tempdir cleanup succeeds.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o755));
    }
}
