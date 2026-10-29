use crate::{api::Api, output, project, reminder_args::ReminderList, reminder_format};
use anyhow::Result;
use serde_json::{Value, json};

pub fn list(
    api: &Api,
    root: &str,
    project: Option<&str>,
    args: ReminderList,
    json_mode: bool,
) -> Result<()> {
    let mut query = reqwest::Url::parse("http://local/")?;
    let mut params = query.query_pairs_mut();
    params.append_pair("state", if args.all { "all" } else { &args.state });
    params.append_pair("summary", if args.verbose { "0" } else { "1" });
    params
        .append_pair("page", &args.page.to_string())
        .append_pair("limit", &args.limit.to_string());
    if args.project_only {
        params.append_pair("project", &project::slug(project)?);
    }
    drop(params);
    let mut result = api.get(&format!("{root}?{}", query.query().unwrap_or("")))?;
    let mut lines = vec![format!(
        "{} reminders (page {})",
        result["total"], result["page"]
    )];
    if let Some(rows) = result["items"].as_array_mut() {
        for row in rows {
            lines.push(item_text(row, args.verbose));
            if !args.verbose
                && let Some(fields) = row.as_object_mut()
            {
                fields.remove("body");
                fields.remove("reference_url");
            }
        }
    }
    output::emit(&result, &lines.join("\n"), json_mode)
}

fn item_text(row: &Value, verbose: bool) -> String {
    if verbose {
        reminder_format::detail(&json!({"reminder": row}))
    } else {
        reminder_format::item(row)
    }
}
