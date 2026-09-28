use crate::{api::Api, project_bootstrap};
use anyhow::{Context, Result};
use clap::Args;
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct ExportArgs {
    #[arg(long)]
    pub all_projects: bool,
    #[arg(long)]
    pub output: Option<PathBuf>,
}

pub fn run(api: &Api, tenant: &str, current: Option<&str>, args: ExportArgs) -> Result<()> {
    let project = if args.all_projects {
        String::new()
    } else {
        format!(
            "?project={}",
            project_bootstrap::resolved_slug(api, tenant, current)?
        )
    };
    let result = api.get(&format!("/api/v1/tenants/{tenant}/export{project}"))?;
    let records = result["records"]
        .as_array()
        .context("invalid export response")?;
    let mut body = String::new();
    for record in records {
        body.push_str(&serde_json::to_string(record)?);
        body.push('\n');
    }
    if let Some(path) = args.output {
        fs::write(&path, body)?;
        println!("Exported {} record(s) to {}", records.len(), path.display());
    } else {
        print!("{body}");
    }
    Ok(())
}
