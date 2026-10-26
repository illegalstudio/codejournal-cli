//! Actionable upgrade notices survive cached reads and reach agent session context.

use crate::api_version::VERSION;
use reqwest::StatusCode;
use serde_json::{Value, json};
use std::cell::RefCell;

const REPOSITORY: &str = "https://github.com/illegalstudio/codejournal-cli";
thread_local! {
    static UPGRADE: RefCell<Option<Value>> = const { RefCell::new(None) };
}

pub fn message(status: StatusCode, body: &Value) -> Option<String> {
    if status != StatusCode::UPGRADE_REQUIRED && body["upgrade_required"] != true {
        return None;
    }
    let minimum = body["minimum_version"]
        .as_str()
        .filter(|value| semver::Version::parse(value).is_ok());
    let explanation = body["message"]
        .as_str()
        .unwrap_or("This server requires a CLI update.");
    let required = minimum.map_or_else(String::new, |version| {
        format!(" Required: {version} or newer.")
    });
    let target = minimum.unwrap_or("latest");
    let message = format!(
        "{explanation} Installed cj: {VERSION}.{required}\n\
         Update with your installation method: `brew update && brew upgrade illegalstudio/tap/codejournal-cli`, \
         `mise use -g github:illegalstudio/codejournal-cli@{target}`, or `cj update` for a direct installation.\n\
         Then run `cj --version`, `cj setup agents --refresh`, and `cj sync`. CLI: {REPOSITORY}#update\n\
         Agents: tell the user an update is required. Cached reads are not fresh; queued writes have not reached the server."
    );
    UPGRADE.with(|saved| *saved.borrow_mut() = Some(json!({
        "error": body["error"].as_str().unwrap_or("client_upgrade_required"),
        "upgrade_required": true, "client_version": VERSION, "minimum_version": minimum,
        "repository_url": REPOSITORY, "update_url": format!("{REPOSITORY}#update"), "message": message,
    })));
    Some(message)
}

pub fn details() -> Option<Value> {
    UPGRADE.with(|saved| saved.borrow().clone())
}

pub fn notice() -> Option<String> {
    details().and_then(|details| details["message"].as_str().map(str::to_owned))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_context_identifies_the_release_and_fixed_installation_sources() {
        let message = message(
            StatusCode::UPGRADE_REQUIRED,
            &json!({
                "message": "Update required", "minimum_version": "1.2.3",
            }),
        )
        .unwrap();
        for expected in [
            VERSION,
            "1.2.3",
            REPOSITORY,
            "brew upgrade",
            "mise use",
            "cj update",
            "cj sync",
            "Agents:",
        ] {
            assert!(message.contains(expected));
        }
        assert_eq!(details().unwrap()["upgrade_required"], true);
        assert!(notice().unwrap().contains("not fresh"));
    }
}
