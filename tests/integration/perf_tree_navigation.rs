use fpv::fs::tree::build_tree;
use std::time::Instant;

#[test]
fn tree_build_is_fast_for_fixture() {
    let root = std::path::Path::new("tests/fixtures");
    let start = Instant::now();
    let nodes = build_tree(root, 5000).expect("build tree");
    let elapsed = start.elapsed().as_secs_f32();
    assert!(!nodes.is_empty());
    assert!(elapsed < 2.0, "tree build took {elapsed}s");
}
