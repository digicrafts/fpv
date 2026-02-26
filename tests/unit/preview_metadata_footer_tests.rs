use fpv::app::state::SelectedEntryMetadata;
use fpv::tui::status_bar::compose_preview_metadata_line;

#[test]
fn metadata_footer_contains_required_fields() {
    let metadata = SelectedEntryMetadata {
        filename: "sample.rs".to_string(),
        size_text: "2.0 KiB".to_string(),
        permission_text: "rw-r--r--".to_string(),
        modified_text: "1700000000".to_string(),
        hidden_text: "off".to_string(),
    };

    let line = compose_preview_metadata_line(&metadata, 200);
    assert!(line.contains("Rust(.rs) | 2.0 KiB | rw-r--r-- | 1700000000"));
}

#[test]
fn metadata_footer_uses_deterministic_fallback_values() {
    let metadata = SelectedEntryMetadata::default();
    let line = compose_preview_metadata_line(&metadata, 200);

    assert!(line.contains("Unknown(none) | - | - | -"));
}

#[test]
fn metadata_footer_uses_none_extension_for_dotless_name() {
    let metadata = SelectedEntryMetadata {
        filename: "Makefile".to_string(),
        size_text: "128 B".to_string(),
        permission_text: "rw-r--r--".to_string(),
        modified_text: "12:12 25-Feb-2026".to_string(),
        hidden_text: "off".to_string(),
    };
    let line = compose_preview_metadata_line(&metadata, 200);
    assert!(line.starts_with("Unknown(none) | 128 B"));
}
