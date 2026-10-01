use crate::{output, request_outbox};
use anyhow::{Result, bail};
use clap::Subcommand;
use fs2::FileExt;
use serde_json::json;
use std::fs::{self, OpenOptions};

#[derive(Subcommand)]
pub enum OutboxAction {
    /// Inspect pending writes without authenticating. Bodies may contain private journal data.
    List {
        #[arg(long)]
        server: Option<String>,
        #[arg(long, help = "Include redacted write bodies")]
        body: bool,
    },
    /// Permanently discard one queued write by its unique request ID prefix.
    Drop { id: String },
}

pub fn run(action: OutboxAction, json_mode: bool) -> Result<()> {
    match action {
        OutboxAction::List { server, body } => {
            let requests = request_outbox::entries()?.into_iter()
                .filter(|(_, item)| server.as_ref().is_none_or(|server| item.server == server.trim_end_matches('/')))
                .map(|(_, item)| {
                    let mut value = json!({"id": item.id, "server": item.server, "method": item.method, "path": item.path});
                    if body { value["body"] = crate::api::sanitized(item.body.unwrap_or_default()); }
                    value
                }).collect::<Vec<_>>();
            let text = requests
                .iter()
                .map(|item| {
                    let mut text = format!(
                        "{}  {} {}  {}",
                        item["id"].as_str().unwrap_or(""),
                        item["method"].as_str().unwrap_or(""),
                        item["path"].as_str().unwrap_or(""),
                        item["server"].as_str().unwrap_or("")
                    );
                    if body {
                        text.push_str(&format!(
                            "\n{}",
                            serde_json::to_string_pretty(&item["body"]).unwrap_or_default()
                        ));
                    }
                    text
                })
                .collect::<Vec<_>>()
                .join("\n");
            output::emit(&json!({"requests": requests}), &text, json_mode)
        }
        OutboxAction::Drop { id } => {
            let prefix = id.replace('-', "").to_lowercase();
            if prefix.len() < 8
                || !prefix
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() || byte == b'-')
            {
                bail!("request ID must be at least eight hexadecimal characters");
            }
            let entries = request_outbox::entries()?;
            let matches = entries
                .iter()
                .filter(|(_, item)| item.id.replace('-', "").starts_with(&prefix))
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                bail!("request ID is missing or ambiguous");
            }
            let (path, request) = matches[0];
            let lock = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(path.parent().unwrap().join(".flush.lock"))?;
            if lock.try_lock_exclusive().is_err() {
                bail!("outbox synchronization is running; retry later");
            }
            fs::remove_file(path)?;
            output::emit(
                &json!({"dropped": request.id}),
                &format!("Dropped queued write {}.", request.id),
                json_mode,
            )
        }
    }
}
