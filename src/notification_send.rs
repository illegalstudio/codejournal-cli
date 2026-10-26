use crate::api::Api;
use crate::watch_args::NotifyArgs;
use crate::{attribution, git, input, notification_delivery, output, project, project_bootstrap};
use anyhow::{Result, bail};
use serde_json::{Value, json};

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
    let body = input::optional_body(args.body, args.body_file)?;
    let cwd = std::env::current_dir()?;
    let branch = git::output(&["branch", "--show-current"]);
    let slug = if name.is_some() {
        project::slug(name)?
    } else {
        project_bootstrap::ensure(api, tenant, false)?
    };
    let mut result = api.post(
        &format!("/api/v1/tenants/{tenant}/notifications"),
        &json!({
            "project": slug, "kind": args.kind, "title": title, "body": body,
            "agent": attribution::agent(args.agent.as_deref()), "host": attribution::host(),
            "checkout_path": cwd, "checkout_kind": "main", "branch": branch,
            "tmux": std::env::var("TMUX").ok(),
        }),
    )?;
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
