use crate::api::Api;
use crate::feedback_args::{FeedbackAction, FeedbackAddArgs, FeedbackListArgs};
use crate::{attribution, checkout_identity, input, output, project, project_bootstrap};
use anyhow::{Result, bail};
use serde_json::{Value, json};

fn path(tenant: &str) -> String {
    format!("/api/v1/tenants/{tenant}/feedback")
}
fn short(id: &str) -> String {
    id.replace('-', "").chars().take(8).collect()
}
fn string(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

pub fn run(
    api: &Api,
    tenant: &str,
    explicit_project: Option<&str>,
    action: FeedbackAction,
    json_mode: bool,
) -> Result<()> {
    match action {
        FeedbackAction::Add(args) => add(api, tenant, explicit_project, args, json_mode),
        FeedbackAction::List(args) => list(api, tenant, args, json_mode),
        FeedbackAction::Show { id } => crate::feedback_show::show(api, tenant, &id, json_mode),
        FeedbackAction::Close { id, note } => status(api, tenant, &id, "done", note, json_mode),
        FeedbackAction::Dismiss { id, note } => {
            status(api, tenant, &id, "dismissed", note, json_mode)
        }
        FeedbackAction::Reopen { id } => status(api, tenant, &id, "open", None, json_mode),
    }
}

fn add(
    api: &Api,
    tenant: &str,
    explicit_project: Option<&str>,
    args: FeedbackAddArgs,
    json_mode: bool,
) -> Result<()> {
    let body = input::body(args.body, args.body_file)?;
    let project = if explicit_project.is_some() {
        project::slug(explicit_project)?
    } else if checkout_identity::current().is_some() {
        project_bootstrap::ensure(api, tenant, false)?
    } else {
        bail!("{}", project::missing_here());
    };
    let response = api.post(
        &path(tenant),
        &json!({
            "project": project, "category": args.category, "title": args.title,
            "body": body, "force": args.force, "agent": attribution::agent(args.agent.as_deref()),
            "host": attribution::host(),
        }),
    )?;
    let item = &response["feedback"];
    output::emit(
        &json!({"feedback": item, "queued": false, "warnings": []}),
        &format!(
            "Recorded feedback {} ({}): {}\nTell the user about it in your final message.",
            short(string(&item["id"])),
            string(&item["category"]),
            string(&item["title"])
        ),
        json_mode,
    )
}

fn list(api: &Api, tenant: &str, args: FeedbackListArgs, json_mode: bool) -> Result<()> {
    let mut params = vec![
        (
            "status",
            if args.all {
                "all".to_owned()
            } else {
                args.status.clone()
            },
        ),
        ("summary", if args.verbose { "0" } else { "1" }.to_owned()),
    ];
    if let Some(category) = args.category {
        params.push(("category", category));
    }
    let query = reqwest::Url::parse_with_params("http://local/", &params)?;
    let mut result = api.get(&format!("{}?{}", path(tenant), query.query().unwrap_or("")))?;
    crate::output::discovery::list(&mut result, "feedback", &params[0].1, args.verbose);
    let mut lines = Vec::new();
    for item in result["feedback"].as_array().into_iter().flatten() {
        let date = string(&item["created_at"]);
        lines.push(format!(
            "  {}  {:<9} {:<10} {}  {}  [{}, {}]",
            short(string(&item["id"])),
            string(&item["status"]),
            string(&item["category"]),
            &date[..date.len().min(10)],
            string(&item["title"]),
            item["agent"].as_str().unwrap_or("unknown"),
            item["project_slug"].as_str().unwrap_or("no project")
        ));
        if args.verbose {
            lines.extend(
                string(&item["body"])
                    .lines()
                    .map(|line| format!("      {line}")),
            );
            if let Some(note) = item["resolution"].as_str() {
                lines.push(format!("      resolution: {note}"));
            }
        }
    }
    output::emit(
        &result,
        &if lines.is_empty() {
            "  (no feedback)".to_owned()
        } else {
            lines.join("\n")
        },
        json_mode,
    )
}

fn status(
    api: &Api,
    tenant: &str,
    id: &str,
    status: &str,
    note: Option<String>,
    json_mode: bool,
) -> Result<()> {
    let result = api.patch(
        &format!("{}/{}", path(tenant), id),
        &json!({"status": status, "note": note}),
    )?;
    let actual = string(&result["feedback"]["id"]);
    output::emit(
        &result,
        &format!("Feedback {} is now {status}.", short(actual)),
        json_mode,
    )
}
