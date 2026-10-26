use super::args::ReviewArgs;
use crate::{api::Api, attribution, output};
use anyhow::{Result, bail};
use serde_json::json;

pub fn run(api: &Api, base: &str, args: ReviewArgs, json_mode: bool) -> Result<()> {
    if api.offline() {
        bail!("Garden review outcomes require an online connection");
    }
    if args.note.trim().is_empty() {
        bail!("A review outcome needs an evidence note");
    }
    let result = api.post(&format!("{base}/garden/review/{}", args.id), &json!({"outcome": args.outcome.name(),
        "note": args.note, "until": args.until, "agent": attribution::agent(args.agent.as_deref())}))?;
    output::emit(
        &result,
        &format!(
            "Garden finding {}: {}. {} pending reviews remain.",
            args.id,
            args.outcome.name(),
            result["counts"]["pending"]
        ),
        json_mode,
    )
}
