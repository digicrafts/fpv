pub const METADATA_FALLBACK: &str = "-";

pub fn truncate_for_status(input: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let chars: Vec<char> = input.chars().collect();
    if chars.len() <= max_chars {
        return input.to_string();
    }
    if max_chars == 1 {
        return "…".to_string();
    }
    let keep = max_chars - 1;
    let head: String = chars.into_iter().take(keep).collect();
    format!("{head}…")
}
