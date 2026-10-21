mod tools;
use super::args;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::io::{BufRead, Write};

pub fn run(offline: bool) -> Result<()> {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut stdout = std::io::stdout().lock();
    loop {
        let mut bytes = Vec::new();
        let size = std::io::Read::take(&mut input, 1_048_577).read_until(b'\n', &mut bytes)?;
        if size == 0 {
            break;
        }
        anyhow::ensure!(size <= 1_048_576, "MCP request exceeds 1 MiB");
        let request: Value = serde_json::from_slice(&bytes)?;
        if request["id"].is_null() {
            continue;
        }
        let result=match super::text(&request,"method") {
            "initialize"=>Ok(json!({"protocolVersion":request["params"]["protocolVersion"].as_str().unwrap_or("2024-11-05"),
                "capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"codejournal-delegation","version":env!("CARGO_PKG_VERSION")}})),
            "ping"=>Ok(json!({})),
            "tools/list"=>Ok(json!({"tools":tools::definitions()})),
            "tools/call"=>call(super::text(&request["params"],"name"),&request["params"]["arguments"])
                .map(|value|json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":value})),
            method=>Err(anyhow::anyhow!("unsupported MCP method: {method}")),
        };
        if !offline && std::env::var("CJ_DELEGATION_JOURNAL").as_deref() != Ok("off") {
            let _ = crate::outbox::spawn_flush();
        }
        let response = match result {
            Ok(value) => json!({"jsonrpc":"2.0","id":request["id"],"result":value}),
            Err(error) => {
                json!({"jsonrpc":"2.0","id":request["id"],"error":{"code":-32000,"message":error.to_string()}})
            }
        };
        writeln!(stdout, "{response}")?;
        stdout.flush()?;
    }
    Ok(())
}

fn call(name: &str, value: &Value) -> Result<Value> {
    let id = value["delegation_id"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| std::env::var("CJ_DELEGATION_ID").ok())
        .context("delegation_id is required")?;
    let context = args::ContextArgs {
        id,
        role: value["role"].as_str().map(str::to_owned),
    };
    match name {
        "delegation_sync" | "delegation_status" => super::reads::snapshot(
            &args::ReadArgs {
                context,
                after: value["after"].as_i64(),
                full: value["full"] == true || name == "delegation_status",
            },
            false,
        ),
        "delegation_wait" => super::reads::wait(
            args::WaitArgs {
                read: args::ReadArgs {
                    context,
                    after: value["after"].as_i64(),
                    full: false,
                },
                timeout: value["timeout"].as_f64().unwrap_or(30.0),
            },
            false,
        ),
        "delegation_ack" => super::reads::ack(
            context,
            value["through"].as_i64().context("through is required")?,
        ),
        _ => {
            let kind = match name {
                "delegation_ready" => "ready",
                "delegation_status_update" => "status",
                "delegation_message" => "message",
                "delegation_question" => "question",
                "delegation_answer" => "answer",
                "delegation_checkpoint" => "checkpoint",
                "delegation_finish" => "result",
                "delegation_fail" => "error",
                _ => bail!("unknown delegation tool: {name}"),
            };
            super::publish::run(args::PublishArgs {
                body: args::BodyArgs {
                    context,
                    body: Some(super::text(value, "body").to_owned()),
                    body_file: None,
                    reply_to: value["reply_to"].as_i64(),
                    idempotency_key: value["idempotency_key"].as_str().map(str::to_owned),
                },
                kind: kind.to_owned(),
            })
        }
    }
}
