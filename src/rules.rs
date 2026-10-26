use crate::api::Api;
use crate::{input, output, project};
use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use serde_json::json;

#[derive(Subcommand)]
pub enum RulesAction {
    Show,
    Clear,
    Set {
        #[arg(allow_hyphen_values = true)]
        text: Option<String>,
    },
    Append {
        #[arg(allow_hyphen_values = true)]
        text: Option<String>,
    },
}

#[derive(Args)]
pub struct RulesArgs {
    #[command(subcommand)]
    pub action: Option<RulesAction>,
    #[arg(
        long,
        global = true,
        conflicts_with = "body_file",
        allow_hyphen_values = true
    )]
    pub body: Option<String>,
    #[arg(long, global = true)]
    pub body_file: Option<String>,
}

pub fn run(
    api: &Api,
    tenant: &str,
    project_name: Option<&str>,
    action: Option<RulesAction>,
    body: Option<String>,
    body_file: Option<String>,
    json_mode: bool,
) -> Result<()> {
    let endpoint = format!(
        "/api/v1/tenants/{tenant}/projects/{}/rules",
        project::slug(project_name)?
    );
    match action {
        None | Some(RulesAction::Show) => {
            let result = api.get(&endpoint)?;
            let rules = result["rules"].as_str().unwrap_or("");
            if rules.is_empty() && !json_mode {
                eprintln!(
                    "No project rules set. Use cj rules set to add them; --json preserves the empty value for scripts."
                );
            }
            output::emit(&result, rules, json_mode)
        }
        Some(RulesAction::Clear) => {
            api.put(&endpoint, &json!({"rules": ""}))?;
            output::emit(
                &json!({"rules": null, "queued": false}),
                "Project rules cleared.",
                json_mode,
            )
        }
        Some(RulesAction::Set { text }) => {
            let rules = input::body(body.or(text), body_file)?;
            write(api, &endpoint, &rules, false, json_mode)
        }
        Some(RulesAction::Append { text }) => {
            let rules = input::body(body.or(text), body_file)?;
            write(api, &endpoint, &rules, true, json_mode)
        }
    }
}

fn write(api: &Api, endpoint: &str, rules: &str, append: bool, json_mode: bool) -> Result<()> {
    if rules.trim().is_empty() {
        bail!("rules must not be empty; use `cj rules clear` to remove them");
    }
    let result = api.put(endpoint, &json!({"rules": rules, "append": append}))?;
    let lines = result["rules"].as_str().unwrap_or("").lines().count();
    output::emit(
        &json!({"rules": result["rules"], "queued": false}),
        &format!("Project rules updated ({lines} line(s))."),
        json_mode,
    )
}
