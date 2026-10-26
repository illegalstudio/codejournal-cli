mod archive;
mod manager;
mod releases;
mod version;

use anyhow::{Context, Result, bail};
use clap::Args;
use semver::Version;
use serde_json::json;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Args)]
pub struct UpdateArgs {
    /// Check the latest release without modifying the installation.
    #[arg(long)]
    check: bool,
}

pub fn run(args: &UpdateArgs, json_mode: bool) -> Result<()> {
    let binary = std::env::current_exe()?.canonicalize()?;
    let managed = manager::command(&binary);
    if !args.check
        && let Some(command) = managed
    {
        bail!(
            "this installation is package-managed; run `{command}`, then `cj setup agents --refresh`"
        );
    }
    let client = releases::client()?;
    let latest = releases::latest(&client)?;
    let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let available = latest > current;
    if !args.check && available {
        let directory = tempfile::Builder::new().prefix("cj-update-").tempdir()?;
        let path = archive::extract(&releases::download(&client, &latest)?, directory.path())?;
        validate(&path, &latest)?;
        self_replace::self_replace(&path)
            .context("cannot replace cj; check installation permissions")?;
        let status = Command::new(&binary)
            .args(["setup", "agents", "--refresh"])
            .stdout(Stdio::null())
            .status()?;
        if !status.success() {
            bail!(
                "cj was updated to {latest}, but agent refresh failed; run `cj setup agents --refresh`"
            );
        }
    } else if !args.check {
        crate::setup_agents::refresh()?;
    }
    let result = json!({"current": current.to_string(), "latest": latest.to_string(), "update_available": available,
        "updated": !args.check && available, "package_manager_command": managed});
    let installed = if available && !args.check {
        &latest
    } else {
        &current
    };
    let message = if args.check {
        format!("Installed cj {current}; latest release {latest}.")
    } else {
        format!("cj {installed} is installed; configured agent skills have been refreshed.")
    };
    crate::output::emit(&result, &message, json_mode)
}

fn validate(path: &Path, version: &Version) -> Result<()> {
    let output = Command::new(path).arg("--version").output()?;
    if !output.status.success()
        || !version::matches(String::from_utf8(output.stdout)?.trim(), version)?
    {
        bail!("downloaded executable does not match the release version; update aborted");
    }
    Ok(())
}
