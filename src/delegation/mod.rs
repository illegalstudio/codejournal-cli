pub mod args;
mod commands;
mod contention;
mod context;
mod create;
mod delivery;
mod events;
pub(crate) mod guide;
mod lifecycle;
mod maintenance;
mod mcp;
mod migrate;
mod models;
mod process;
mod providers;
mod publish;
mod reads;
mod schema;
mod store;
mod worker;
mod workspace;

use anyhow::Result;
use serde_json::Value;

pub fn now() -> f64 {
    chrono::Utc::now().timestamp_millis() as f64 / 1000.0
}

pub fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    value[field].as_str().unwrap_or("")
}

pub fn terminal(value: &Value) -> bool {
    matches!(text(value, "status"), "completed" | "failed" | "cancelled")
}

pub fn run(
    args: args::DelegationArgs,
    project: Option<String>,
    server: Option<String>,
    offline: bool,
) -> Result<()> {
    let offline = offline || std::env::var("CJ_DELEGATION_OFFLINE").as_deref() == Ok("1");
    let result = commands::run(args.action, project, server, offline);
    if !offline && std::env::var("CJ_DELEGATION_JOURNAL").as_deref() != Ok("off") {
        if let Err(error) = crate::outbox::spawn_flush() {
            eprintln!("Delegation events are queued; delivery could not start: {error}");
        }
    }
    result
}
