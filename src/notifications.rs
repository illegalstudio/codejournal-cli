use crate::api::Api;
use crate::watch_args::NotificationAction;
use crate::{notification_delivery, output, project};
use anyhow::{Result, bail};
use serde_json::{Value, json};

fn endpoint(tenant: &str) -> String {
    format!("/api/v1/tenants/{tenant}/notifications")
}

pub fn run(
    api: &Api,
    tenant: &str,
    name: Option<&str>,
    action: NotificationAction,
    json_mode: bool,
) -> Result<()> {
    match action {
        NotificationAction::List {
            unread,
            read,
            kind,
            project_only,
            limit,
            verbose,
        } => {
            let mut params = vec![("limit", limit.to_string())];
            if unread {
                params.push(("state", "unread".into()));
            }
            if read {
                params.push(("state", "read".into()));
            }
            if let Some(kind) = kind {
                params.push(("kind", kind));
            }
            if project_only {
                params.push(("project", project::slug(name)?));
            }
            let url = reqwest::Url::parse_with_params("http://local/", &params)?;
            let result = api.get(&format!(
                "{}?{}",
                endpoint(tenant),
                url.query().unwrap_or("")
            ))?;
            let mut lines = vec![format!("{} unread (* marks unread)", result["unread"])];
            for row in result["notifications"].as_array().into_iter().flatten() {
                let id = value(&row["id"]).replace('-', "");
                let date = value(&row["created_at"]);
                let where_text = row["project_slug"]
                    .as_str()
                    .or_else(|| row["checkout_path"].as_str())
                    .unwrap_or("?");
                lines.push(format!(
                    "{} {}  {}  {:<11} {:<8} {}  [{}]",
                    if row["read_at"].is_null() { '*' } else { ' ' },
                    &id[..id.len().min(8)],
                    &date[..date.len().min(16)],
                    value(&row["kind"]),
                    row["agent"].as_str().unwrap_or("unknown"),
                    value(&row["title"]),
                    where_text
                ));
                if verbose {
                    lines.extend(
                        value(&row["body"])
                            .lines()
                            .map(|line| format!("      {line}")),
                    );
                }
            }
            output::emit(&result, &lines.join("\n"), json_mode)
        }
        NotificationAction::Read { ids, all } => mark(api, tenant, ids, all, true, json_mode),
        NotificationAction::Unread { ids, all } => {
            if all {
                bail!("--all only applies to `read`");
            }
            mark(api, tenant, ids, false, false, json_mode)
        }
        NotificationAction::Config { desktop, ntfy } => {
            notification_delivery::configure(desktop, ntfy, json_mode)
        }
    }
}

fn mark(
    api: &Api,
    tenant: &str,
    ids: Vec<String>,
    all: bool,
    read: bool,
    json_mode: bool,
) -> Result<()> {
    if !all && ids.is_empty() {
        bail!("pass notification ids or --all");
    }
    let result = api.patch(
        &endpoint(tenant),
        &json!({"ids": ids, "all": all, "read": read}),
    )?;
    output::emit(
        &result,
        &format!(
            "Marked {} notification(s) as {}.",
            result["changed"],
            if read { "read" } else { "unread" }
        ),
        json_mode,
    )
}

fn value(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
