use super::{
    args::{Until, WaitArgs},
    context, cursors, reads, text,
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub fn run(mut args: WaitArgs, result_only: bool) -> Result<Value> {
    ensure!(
        args.timeout.is_finite() && (0.0..=86400.0).contains(&args.timeout),
        "wait timeout must be between 0 and 86400 seconds"
    );
    let role = context::role(args.read.context.role.as_deref())?;
    let remember = args.read.after.is_none() && !args.read.full;
    if remember {
        args.read.after = Some(cursors::after(&args.read.context.id, &role)?);
    }
    let until = if result_only {
        Until::Result
    } else {
        args.until
    };
    let deadline = Instant::now() + Duration::from_secs_f64(args.timeout);
    loop {
        let mut value = reads::snapshot(&args.read, true)?;
        let terminal =
            super::terminal(&value["session"]) || text(&value["session"], "status") == "stalled";
        let interesting = match until {
            Until::Event => value["events"]
                .as_array()
                .is_some_and(|events| !events.is_empty()),
            Until::Question => {
                value["events"]
                    .as_array()
                    .is_some_and(|events| events.iter().any(|e| e["kind"] == "question"))
                    || value["session"]["status"] == "waiting_for_host"
            }
            Until::Result | Until::Terminal => terminal,
        };
        let timed_out = !terminal && !interesting && Instant::now() >= deadline;
        if terminal || interesting || timed_out {
            value["timed_out"] = json!(timed_out);
            if until == Until::Result {
                value["result"] = value["session"]["result"].clone();
                value["events"] = json!(
                    value["result_event"]
                        .as_object()
                        .map(|_| vec![value["result_event"].clone()])
                        .unwrap_or_default()
                );
            }
            if remember && !timed_out {
                let seq = value["events"]
                    .as_array()
                    .and_then(|events| events.last())
                    .and_then(|e| e["seq"].as_i64());
                if let Some(seq) = seq {
                    cursors::delivered(&args.read.context.id, &role, seq)?;
                }
            }
            return Ok(value);
        }
        if until != Until::Event && !args.read.full {
            args.read.after = value["next_after"]
                .as_i64()
                .or_else(|| value["latest_seq"].as_i64());
        }
        std::thread::sleep(
            Duration::from_millis(100).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
}
