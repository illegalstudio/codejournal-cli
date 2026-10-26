use serde_json::{Value, json};

pub const NAMES: &[&str] = &[
    "CJ_DELEGATION_ID",
    "CJ_DELEGATION_ROLE",
    "CJ_DELEGATION_TOKEN",
    "CJ_DELEGATION_ATTEMPT",
    "CJ_DELEGATION_CLIENT",
    "CJ_DELEGATION_DEPTH",
    "CJ_DELEGATION_OFFLINE",
    "CJ_DELEGATION_JOURNAL",
    "XDG_STATE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_CACHE_HOME",
    "CODE_JOURNAL_AUTO_SYNC",
    "CODE_JOURNAL_HOOK_FLUSH",
];

pub fn values() -> Value {
    let mut values = json!({});
    for name in NAMES {
        if let Ok(value) = std::env::var(name) {
            values[*name] = json!(value);
        }
    }
    values
}
