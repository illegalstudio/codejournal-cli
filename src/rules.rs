use crate::api::Api;
use crate::{input, output, project};
use anyhow::Result;
use clap::Subcommand;
use serde_json::json;

#[derive(Subcommand)]
pub enum RulesAction {
    Set {
        #[arg(allow_hyphen_values = true)]
        text: Option<String>,
    },
    Append {
        #[arg(allow_hyphen_values = true)]
        text: Option<String>,
    },
}

pub fn run(
    api: &Api,
    tenant: &str,
    project_name: Option<&str>,
    action: Option<RulesAction>,
) -> Result<()> {
    let endpoint = format!(
        "/api/v1/tenants/{tenant}/projects/{}/rules",
        project::slug(project_name)?
    );
    match action {
        None => output::json(&api.get(&endpoint)?),
        Some(RulesAction::Set { text }) => {
            let rules = input::body(text, None)?;
            output::json(&api.put(&endpoint, &json!({"rules": rules}))?)
        }
        Some(RulesAction::Append { text }) => {
            let rules = input::body(text, None)?;
            output::json(&api.put(&endpoint, &json!({"rules": rules, "append": true}))?)
        }
    }
}
