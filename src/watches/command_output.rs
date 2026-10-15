use super::{process, tail};
use crate::{api::Api, attribution, outbox, output, watch_state};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::io::Read;

pub fn run(
    api: &Api,
    tenant: &str,
    path: &str,
    prefix: &str,
    lines: usize,
    json_mode: bool,
) -> Result<()> {
    let normalized = prefix.replace('-', "").to_lowercase();
    ensure!(
        (8..=32).contains(&normalized.len()) && normalized.bytes().all(|ch| ch.is_ascii_hexdigit()),
        "watch ID must be a UUID or a prefix of at least 8 hex characters"
    );
    let host = attribution::host();
    let url = reqwest::Url::parse_with_params("http://local/", [("host", &host)])?;
    let response = api.get(&format!(
        "{path}/{normalized}?{}",
        url.query().unwrap_or("")
    ))?;
    let watch = &response["watch"];
    let id = watch["id"].as_str().context("watch response has no ID")?;
    uuid::Uuid::parse_str(id).context("invalid watch ID in response")?;
    ensure!(
        id.replace('-', "").to_lowercase().starts_with(&normalized) && watch["host"] == host,
        "watch response does not match this ID and host"
    );
    let scope = outbox::fingerprint(api.server(), tenant, &api.token);
    let pid = verified_pid(id, &scope);
    let local = pid.and_then(|_| {
        tail::read(
            &watch_state::pid_path(id).ok()?.with_extension("out"),
            lines,
        )
        .ok()
    });
    let mut snapshot = if let Some(mut snapshot) = local {
        snapshot["source"] = json!("local");
        snapshot
    } else if let Some(text) = watch["tail"].as_str() {
        let mut snapshot = tail::bounded(text, lines);
        snapshot["source"] = json!("server");
        snapshot
    } else {
        json!({"source": "unavailable", "tail": null, "truncated": false})
    };
    snapshot["lines_requested"] = json!(lines);
    let result = json!({
        "watch": {"id": id, "title": watch["title"], "status": watch["status"],
            "host": watch["host"], "project_slug": watch["project_slug"], "exit_code": watch["exit_code"],
            "lease_expires_at": watch["lease_expires_at"]},
        "cached": response["cached"] == true, "runner_pid": pid, "runner_verified": pid.is_some(),
        "output": snapshot,
        "notices": ["Lease renewal does not prove command progress. Output is a bounded snapshot; a quiet command may still be running."],
    });
    output::emit(&result, &format(&result), json_mode)
}

fn verified_pid(id: &str, scope: &str) -> Option<u32> {
    let file = tail::regular_file(&watch_state::pid_path(id).ok()?).ok()?;
    let mut bytes = Vec::new();
    file.take(4096).read_to_end(&mut bytes).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    if value["scope"] != scope {
        return None;
    }
    let pid = u32::try_from(value["pid"].as_u64()?).ok()?;
    process::owns(pid, id).then_some(pid)
}

fn format(value: &Value) -> String {
    let watch = &value["watch"];
    let output = &value["output"];
    let cached = if value["cached"] == true {
        " (cached watch metadata)"
    } else {
        ""
    };
    let runner = value["runner_pid"]
        .as_u64()
        .map_or("unverified".to_owned(), |pid| format!("verified PID {pid}"));
    let mut text = format!(
        "{}  {}{cached}\nRunner: {runner}\nOutput: {}",
        watch["id"].as_str().unwrap_or(""),
        watch["status"].as_str().unwrap_or("unknown"),
        output["source"].as_str().unwrap_or("unavailable")
    );
    if let Some(time) = output["last_output_at"].as_str() {
        text.push_str(&format!(", last file update {time}"));
    }
    if output["truncated"] == true {
        text.push_str(" (truncated)");
    }
    text.push_str(
        "\nLease renewal does not prove command progress. A quiet command may still be running.\n",
    );
    text.push_str(
        output["tail"]
            .as_str()
            .filter(|text| !text.is_empty())
            .unwrap_or("No captured output available."),
    );
    text
}
