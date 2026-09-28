use crate::api::Api;
use crate::{input, output, project};
use anyhow::Result;
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
    #[arg(long, global = true, conflicts_with = "body_file")]
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
) -> Result<()> {
    let endpoint = format!(
        "/api/v1/tenants/{tenant}/projects/{}/rules",
        project::slug(project_name)?
    );
    match action {
        None | Some(RulesAction::Show) => {
            let result = api.get(&endpoint)?;
            println!("{}", result["rules"].as_str().unwrap_or(""));
            Ok(())
        }
        Some(RulesAction::Clear) => output::json(&api.put(&endpoint, &json!({"rules": ""}))?),
        Some(RulesAction::Set { text }) => {
            let rules = input::body(body.or(text), body_file)?;
            output::json(&api.put(&endpoint, &json!({"rules": rules}))?)
        }
        Some(RulesAction::Append { text }) => {
            let rules = input::body(body.or(text), body_file)?;
            output::json(&api.put(&endpoint, &json!({"rules": rules, "append": true}))?)
        }
    }
}
