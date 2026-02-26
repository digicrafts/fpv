use fpv::app::state::{NodeType, SessionState, TreeNode};
use std::path::PathBuf;

#[test]
fn selection_revalidates_on_smaller_list() {
    let mut state = SessionState::new(PathBuf::from("."));
    state.selected_index = 5;
    let nodes = vec![TreeNode {
        path: PathBuf::from("a"),
        name: "a".into(),
        node_type: NodeType::Directory,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];
    state.revalidate_selection(&nodes);
    assert_eq!(state.selected_index, 0);
}
