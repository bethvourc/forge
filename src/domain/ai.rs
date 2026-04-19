#[derive(Debug, Clone, Default)]
pub struct AiSessionState {
    pub enabled: bool,
    pub provider: Option<String>,
    pub last_response: Option<String>,
    pub last_error: Option<String>,
}

