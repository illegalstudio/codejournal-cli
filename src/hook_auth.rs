use crate::config::Config;

pub fn notice() -> Option<String> {
    let config = Config::load().ok()?;
    config.token().err().map(|error| format!(
        "Code Journal authentication is unavailable in this session: {error}. Journal commands cannot save writes until authentication is available."
    ))
}
