use crate::api::Api;
use crate::watch_args::NotifyArgs;
use crate::{attribution, git, input, notification_delivery, output, project, project_bootstrap};
use anyhow::{Result, bail, ensure};
use serde_json::{Value, json};

pub fn retired(body: &Value) -> bool {
    body["kind"] == "needs_input" || body["source"] == "hook_approval"
}

pub fn skip_queued(request: &crate::request_outbox::PendingRequest) -> bool {
    let parts = request
        .path
        .trim_matches('/')
        .split('/')
        .collect::<Vec<_>>();
    request.method == "POST"
        && matches!(
            parts.as_slice(),
            ["api", "v1", "tenants", _, "notifications"]
        )
        && request.body.as_ref().is_some_and(|body| {
            retired(body)
                || body["important"] != true
                || !matches!(body["kind"].as_str(), Some("info" | "done" | "error"))
        })
}

pub fn run(
    api: &Api,
    tenant: &str,
    name: Option<&str>,
    args: NotifyArgs,
    json_mode: bool,
) -> Result<()> {
    let title = args.title.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() {
        bail!("a notification needs a title");
    }
    ensure!(args.important, "use --important for exceptional updates");
    let body = input::optional_body(args.body, args.body_file)?;
    let cwd = std::env::current_dir()?;
    let branch = git::output(&["branch", "--show-current"]);
    let mut payload = json!({
        "kind": args.kind, "important": true, "title": title, "body": body,
        "agent": attribution::agent(args.agent.as_deref()), "host": attribution::host(),
        "checkout_path": cwd, "checkout_kind": "main", "branch": branch,
        "tmux": std::env::var("TMUX").ok(),
    });
    validate(&payload)?;
    let slug = if name.is_some() {
        project::slug(name)?
    } else {
        project_bootstrap::ensure(api, tenant, false)?
    };
    payload["project"] = json!(slug);
    validate_field(&payload, "project", 255)?;
    let mut result = api.post(&format!("/api/v1/tenants/{tenant}/notifications"), &payload)?;
    ensure!(
        result["notification"].is_object(),
        "server returned no confirmed notification"
    );
    result["notification"]["project_slug"] = json!(slug);
    let (delivery, notes) = notification_delivery::deliver(&result["notification"], &slug)?;
    let channels = delivery["channels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    result["delivery"] = delivery;
    result["delivery_notes"] = json!(notes);
    output::emit(
        &result,
        &format!(
            "Notified the user ({}): {title} [{slug}] via {channels}",
            args.kind
        ),
        json_mode,
    )
}

fn validate(payload: &Value) -> Result<()> {
    for (field, max) in [
        ("title", 255),
        ("body", 50000),
        ("agent", 100),
        ("host", 255),
        ("checkout_path", 2000),
        ("checkout_kind", 24),
        ("branch", 255),
        ("tmux", 255),
    ] {
        validate_field(payload, field, max)?;
    }
    Ok(())
}

fn validate_field(payload: &Value, field: &str, max: usize) -> Result<()> {
    ensure!(
        payload[field]
            .as_str()
            .is_none_or(|text| text.chars().count() <= max),
        "notification {field} exceeds {max} characters"
    );
    Ok(())
}
