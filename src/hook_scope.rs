use crate::{
    api::Api,
    config::Config,
    project_folder::{self, Scope},
    session_state::SessionState,
};
use std::time::Duration;

pub fn prepare(state: &mut SessionState, event: &str) {
    if event != "SessionStart" && state.journal_active.is_some() {
        return;
    }
    if let Ok(slug) = std::env::var("CJ_PROJECT") {
        state.project = Some(slug);
        state.journal_active = Some(true);
        return;
    }
    if state.root.is_some() {
        state.journal_active = Some(true);
        return;
    }
    let scope = (|| {
        let config = Config::load().ok()?;
        let api = Api::with_timeout(
            &config.server,
            &config.token().ok()?,
            Duration::from_secs(3),
        )
        .ok()?;
        Some(project_folder::registered(&api, &config.tenant))
    })()
    .unwrap_or(Scope::Unknown);
    match scope {
        Scope::Named(slug) => {
            state.project = Some(slug);
            state.journal_active = Some(true);
        }
        Scope::Outside => {
            state.project = None;
            state.journal_active = Some(false);
        }
        _ => {
            state.journal_active = None;
        }
    }
}
