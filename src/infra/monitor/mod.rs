pub fn service_name_from_command(raw: &str) -> String {
    raw.split_whitespace()
        .next()
        .unwrap_or("service")
        .to_string()
}

