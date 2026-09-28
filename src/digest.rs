use crate::api::Api;
use crate::{notification_delivery, output, project};
use anyhow::Result;
use clap::Args;
use serde_json::json;

#[derive(Args)]
pub struct DigestArgs {
    #[arg(long, default_value = "yesterday")]
    pub since: String,
    #[arg(long)]
    pub project_only: bool,
    #[arg(long)]
    pub send: bool,
}

pub fn run(
    api: &Api,
    tenant: &str,
    name: Option<&str>,
    since: &str,
    project_only: bool,
    send: bool,
    json_mode: bool,
) -> Result<()> {
    let mut params = vec![("since", since.to_owned())];
    if project_only {
        params.push(("project", project::slug(name)?));
    }
    let url = reqwest::Url::parse_with_params("http://local/", &params)?;
    let mut result = api.get(&format!(
        "/api/v1/tenants/{tenant}/digest?{}",
        url.query().unwrap_or("")
    ))?;
    let label = if ["today", "yesterday"].contains(&since) {
        since.to_owned()
    } else {
        format!("since {since}")
    };
    let markdown = crate::digest_format::render(&result, &label);
    let mut notes = Vec::new();
    if send {
        match notification_delivery::send_digest(&markdown, &label) {
            Ok(()) => notes.push("sent to ntfy".to_owned()),
            Err(error) => notes.push(format!("ntfy delivery failed: {error}")),
        }
    }
    result["markdown"] = json!(markdown);
    result["notes"] = json!(notes);
    let suffix = notes
        .iter()
        .map(|note| format!("\nnote: {note}"))
        .collect::<String>();
    output::emit(&result, &format!("{markdown}{suffix}"), json_mode)
}
