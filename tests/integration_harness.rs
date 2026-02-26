#[path = "integration/config_override_flow.rs"]
mod config_override_flow;
#[path = "integration/empty_directory_state_tests.rs"]
mod empty_directory_state_tests;
#[path = "integration/keyboard_navigation_flow.rs"]
mod keyboard_navigation_flow;
#[path = "integration/path_context_consistency_tests.rs"]
mod path_context_consistency_tests;
#[path = "integration/perf_directory_transition_tests.rs"]
mod perf_directory_transition_tests;
#[path = "integration/perf_preview_latency.rs"]
mod perf_preview_latency;
#[path = "integration/perf_tree_navigation.rs"]
mod perf_tree_navigation;
#[path = "integration/permission_display_regression.rs"]
mod permission_display_regression;
#[path = "integration/preview_footer_layout_flow.rs"]
mod preview_footer_layout_flow;
#[path = "integration/preview_navigation_flow.rs"]
mod preview_navigation_flow;
#[path = "integration/preview_panel_title_flow.rs"]
mod preview_panel_title_flow;
#[path = "integration/quit_restore_terminal.rs"]
mod quit_restore_terminal;
#[path = "integration/rapid_navigation_regression_tests.rs"]
mod rapid_navigation_regression_tests;
#[path = "integration/regression_suite.rs"]
mod regression_suite;
#[path = "integration/single_layer_navigation_flow.rs"]
mod single_layer_navigation_flow;

// Navigation display refinement coverage:
// - directory/file ordering and prefix rendering flow
// - hidden toggle behavior and selection validity
// - directory preview neutrality and file-error preservation
// - top header/two-pane metadata/status layout contract
// - home-path (~) display behavior and theme visual cue regression checks
