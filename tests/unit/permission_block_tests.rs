use fpv::app::navigation::enter_selected_directory;
use fpv::app::navigation_result::ActionOutcome;
use fpv::app::state::{NodeType, SessionState, TreeNode};
use std::path::PathBuf;

#[test]
fn unreadable_directory_is_blocked() {
    let mut state = SessionState::new(PathBuf::from("."));
    let mut nodes = vec![TreeNode {
        path: PathBuf::from("private"),
        name: "private".into(),
        node_type: NodeType::Directory,
        depth: 0,
        expanded: false,
        readable: false,
        children_loaded: false,
    }];
    let result = enter_selected_directory(&mut state, &mut nodes).expect("enter");
    assert_eq!(result.outcome, ActionOutcome::Blocked);
}
