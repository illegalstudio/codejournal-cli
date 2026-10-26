use crate::{api::Api, normalize_plan, output};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub fn run(api: &Api, tenant: &str, dry_run: bool, json_mode: bool) -> Result<()> {
    if !dry_run && api.offline() {
        bail!("normalize needs a writable journal; use --dry-run offline");
    }
    let mut plan = normalize_plan::build(api, tenant)?;
    if !dry_run && plan["changes"] != 0 {
        api.post_noqueue(
            &format!("/api/v1/tenants/{tenant}/checkouts/normalize"),
            &plan,
        )?;
    }
    plan["dry_run"] = json!(dry_run);
    let mut lines = vec![format!(
        "Checkout records on host {}{}",
        value(&plan["host"]),
        if dry_run {
            " (dry run, nothing written)"
        } else {
            ""
        }
    )];
    for row in rows(&plan["merges"]) {
        lines.push(format!(
            "  merge project    {} into {}",
            value(&row["from"]),
            value(&row["into"])
        ));
    }
    for (label, key) in [
        ("remove", "removals"),
        ("reclassify", "updates"),
        ("add main", "add_main"),
        ("keep", "kept"),
    ] {
        for row in rows(&plan[key]) {
            let detail = match key {
                "removals" | "kept" => format!("  ({})", value(&row["reason"])),
                "updates" => format!(
                    "  {} -> {}{}",
                    value(&row["kind"]),
                    value(&row["new_kind"]),
                    row["new_branch"]
                        .as_str()
                        .map(|branch| format!(" {branch}"))
                        .unwrap_or_default()
                ),
                _ => String::new(),
            };
            lines.push(format!(
                "  {label:<16} {:<28} {}{detail}",
                value(&row["slug"]),
                value(&row["path"])
            ));
        }
    }
    if plan["changes"] == 0 {
        lines.push("  nothing to change".to_owned());
    }
    if let Some(other_hosts) = plan["other_hosts"].as_u64().filter(|count| *count > 0) {
        lines.push(format!(
            "  {other_hosts} record(s) on other hosts left untouched; run normalize there too"
        ));
    }
    output::emit(&plan, &lines.join("\n"), json_mode)
}

fn rows(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn value(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
