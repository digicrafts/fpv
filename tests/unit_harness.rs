#[path = "unit/config_conflict_tests.rs"]
mod config_conflict_tests;
#[path = "unit/config_parse_tests.rs"]
mod config_parse_tests;
#[path = "unit/current_dir_listing_tests.rs"]
mod current_dir_listing_tests;
#[path = "unit/enter_directory_tests.rs"]
mod enter_directory_tests;
#[path = "unit/input_mapping_tests.rs"]
mod input_mapping_tests;
#[path = "unit/permission_block_tests.rs"]
mod permission_block_tests;
#[path = "unit/permission_display_value_tests.rs"]
mod permission_display_value_tests;
#[path = "unit/preview_metadata_footer_tests.rs"]
mod preview_metadata_footer_tests;
#[path = "unit/preview_mode_tests.rs"]
mod preview_mode_tests;
#[path = "unit/preview_panel_title_tests.rs"]
mod preview_panel_title_tests;
#[path = "unit/root_boundary_tests.rs"]
mod root_boundary_tests;
#[path = "unit/selection_bounds_tests.rs"]
mod selection_bounds_tests;
#[path = "unit/selection_revalidation_tests.rs"]
mod selection_revalidation_tests;
#[path = "unit/tree_state_tests.rs"]
mod tree_state_tests;

// Navigation display refinement coverage:
// - directory-first ordering and hidden filtering
// - prefix mapping
// - hidden-toggle key mapping
// - neutral directory preview mode
// - top header and preview metadata line formatting
// - home-path (~) formatting and theme-style resolution
