use fpv::app::state::{NodeType, TreeNode};
use fpv::fs::current_dir::selected_entry_metadata;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn permission_display_is_compact_symbolic_form() {
    let d = tempdir().expect("tempdir");
    let file = d.path().join("perm.txt");
    fs::write(&file, "hello").expect("write");

    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&file).expect("metadata").permissions();
        perms.set_mode(0o754);
        fs::set_permissions(&file, perms).expect("chmod");
    }

    let node = TreeNode {
        path: file,
        name: "perm.txt".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    };

    let metadata = selected_entry_metadata(&node);
    assert_eq!(metadata.permission_text.chars().count(), 9);

    #[cfg(unix)]
    assert_eq!(metadata.permission_text, "rwxr-xr--");
}

#[test]
fn hidden_status_is_set_for_dot_entries() {
    let node = TreeNode {
        path: PathBuf::from(".secret"),
        name: ".secret".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    };

    let metadata = selected_entry_metadata(&node);
    assert_eq!(metadata.hidden_text, "on");
}
