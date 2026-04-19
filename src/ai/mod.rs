pub fn unavailable_message(prompt: &str) -> String {
    format!(
        "AI provider is not configured yet. Request captured but not executed: {}",
        prompt.trim()
    )
}
