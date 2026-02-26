use fpv::app::state::NodeType;
use fpv::fs::current_dir::{list_current_directory, list_current_directory_with_visibility};
use std::fs;
use tempfile::tempdir;

#[test]
fn lists_only_direct_children() {
    let d = tempdir().expect("tempdir");
    fs::create_dir_all(d.path().join("a/b")).expect("mkdir");
    fs::write(d.path().join("a/b/file.txt"), "x").expect("write");
    fs::write(d.path().join("root.txt"), "y").expect("write");

    let nodes = list_current_directory(d.path(), 2000).expect("list");
    let names: Vec<String> = nodes.into_iter().map(|n| n.name).collect();
    assert!(names.contains(&"a".to_string()));
    assert!(names.contains(&"root.txt".to_string()));
    assert!(!names.contains(&"file.txt".to_string()));
}

#[test]
fn sorts_directory_first_then_files() {
    let d = tempdir().expect("tempdir");
    fs::create_dir_all(d.path().join("beta_dir")).expect("mkdir");
    fs::create_dir_all(d.path().join("alpha_dir")).expect("mkdir");
    fs::write(d.path().join("zeta.txt"), "z").expect("write");
    fs::write(d.path().join("alpha.txt"), "a").expect("write");

    let nodes = list_current_directory(d.path(), 2000).expect("list");
    let kinds: Vec<NodeType> = nodes.iter().map(|n| n.node_type.clone()).collect();
    let names: Vec<String> = nodes.iter().map(|n| n.name.clone()).collect();

    assert_eq!(kinds[0], NodeType::Directory);
    assert_eq!(kinds[1], NodeType::Directory);
    assert_eq!(kinds[2], NodeType::File);
    assert_eq!(kinds[3], NodeType::File);
    assert_eq!(
        names,
        vec!["alpha_dir", "beta_dir", "alpha.txt", "zeta.txt"]
    );
}

#[test]
fn hides_dot_entries_when_show_hidden_is_false() {
    let d = tempdir().expect("tempdir");
    fs::create_dir_all(d.path().join(".hidden_dir")).expect("mkdir");
    fs::create_dir_all(d.path().join("visible_dir")).expect("mkdir");
    fs::write(d.path().join(".hidden.txt"), "h").expect("write");
    fs::write(d.path().join("visible.txt"), "v").expect("write");

    let hidden = list_current_directory_with_visibility(d.path(), 2000, true).expect("list all");
    let filtered =
        list_current_directory_with_visibility(d.path(), 2000, false).expect("list filtered");

    let hidden_names: Vec<String> = hidden.iter().map(|n| n.name.clone()).collect();
    let filtered_names: Vec<String> = filtered.iter().map(|n| n.name.clone()).collect();

    assert!(hidden_names.contains(&".hidden_dir".to_string()));
    assert!(hidden_names.contains(&".hidden.txt".to_string()));
    assert!(!filtered_names.iter().any(|n| n.starts_with('.')));
}
