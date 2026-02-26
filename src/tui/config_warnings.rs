pub fn render_warning_text(warnings: &[String]) -> String {
    if warnings.is_empty() {
        String::new()
    } else {
        format!("Keymap warnings: {}", warnings.join("; "))
    }
}
