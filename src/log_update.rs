use crate::{api::Api, input, log_args::LogUpdateArgs, output, refs};
use anyhow::Result;
use serde_json::json;

pub fn run(api: &Api, tenant: &str, args: LogUpdateArgs, json_mode: bool) -> Result<()> {
    let references = refs::parse_all(&args.refs)?;
    let has_body = args.body.is_some() || args.body_file.is_some();
    let mut payload = json!({});
    if args.clear_refs || !args.refs.is_empty() {
        payload["refs"] = json!(references);
    }
    if has_body {
        let body = input::optional_body(args.body, args.body_file)?;
        anyhow::ensure!(
            body.chars().count() <= 100_000,
            "work log body exceeds 100000 characters; nothing was sent or queued"
        );
        payload["body"] = json!(body);
    }
    if let Some(status) = args.status {
        payload["status"] = json!(status);
    }
    let id = &args.id;
    let path = format!("/api/v1/tenants/{tenant}/logs/{id}");
    let endpoint = if let Some(base) = args.base {
        payload["based_on"] = json!(base);
        format!("{path}/amend")
    } else {
        path
    };
    let result = api.patch(&endpoint, &payload)?;
    if let Some(base) = args.base {
        anyhow::ensure!(
            result["log"]["revision"].as_u64() == Some(u64::from(base) + 1),
            "the server did not confirm the protected log amendment; inspect the current log before retrying"
        );
    }
    let text = if args.base.is_some() {
        format!(
            "Updated work log {id} to revision {}.",
            result["log"]["revision"]
        )
    } else {
        format!("Updated refs of work log {id}.")
    };
    output::emit(&result, &text, json_mode)
}
