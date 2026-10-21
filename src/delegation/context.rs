use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub fn role(requested: Option<&str>) -> Result<String> {
    let inherited = std::env::var("CJ_DELEGATION_ROLE").unwrap_or_default();
    ensure!(
        inherited.is_empty() || requested.is_none_or(|r| r == inherited),
        "a delegated guest cannot impersonate the host"
    );
    let role = requested.unwrap_or(if inherited.is_empty() {
        "host"
    } else {
        &inherited
    });
    ensure!(
        matches!(role, "host" | "guest"),
        "role must be host or guest"
    );
    Ok(role.to_owned())
}

pub fn host_only() -> Result<()> {
    ensure!(
        role(None)? != "guest",
        "guest processes cannot perform host actions"
    );
    Ok(())
}

pub fn authorize(session: &Value, role: &str) -> Result<()> {
    if role == "guest" {
        let token = std::env::var("CJ_DELEGATION_TOKEN").unwrap_or_default();
        let id = std::env::var("CJ_DELEGATION_ID").unwrap_or_default();
        ensure!(
            id == super::text(session, "id")
                && !token.is_empty()
                && equal(&token, super::text(session, "attempt_token")),
            "guest attempt is no longer authorized"
        );
    }
    Ok(())
}

fn equal(left: &str, right: &str) -> bool {
    let mut difference = left.len() ^ right.len();
    for (a, b) in left.bytes().zip(right.bytes()) {
        difference |= usize::from(a ^ b);
    }
    difference == 0
}

pub fn capture(project: Option<String>, server: Option<String>) -> Value {
    let mut origin = json!({});
    crate::outbox::origin::capture(&mut origin);
    if let Some(server) = &server {
        let binding = crate::config::Config::load().ok().and_then(|config| {
            config
                .token_for(server)
                .ok()
                .map(|token| crate::outbox::fingerprint(server, &config.tenant, &token))
        });
        origin["origin"] = json!(binding);
    }
    let project = project.or_else(|| crate::project::slug(None).ok());
    json!({"binding": origin, "project": project,
        "host": {"agent": crate::attribution::agent(None), "session": crate::session_state::current_id()},
        "machine": crate::attribution::host()})
}
