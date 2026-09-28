use crate::{api::Api, normalize_plan, output};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub fn run(api: &Api, tenant: &str, dry_run: bool, json_mode: bool) -> Result<()> {
    if !dry_run && api.offline() {
        bail!("normalize needs a writable journal; use --dry-run offline");
    }
    let mut plan = normalize_plan::build(api, tenant)?;
    let base = format!("/api/v1/tenants/{tenant}/projects");
    if !dry_run {
        for row in rows(&plan["merges"]) {
            api.post_noqueue(
                &format!("{base}/merge"),
                &json!({"source": row["from"], "into": row["into"]}),
            )?;
        }
        for row in rows(&plan["removals"]) {
            let slug = merged_slug(&plan, value(&row["slug"]));
            api.post_noqueue(
                &format!("{base}/{slug}/paths/remove"),
                &json!({"host": plan["host"], "path": row["path"]}),
            )?;
        }
        for row in rows(&plan["updates"]) {
            api.post_noqueue(
                &format!("{base}/{}/paths", value(&row["slug"])),
                &json!({"host": plan["host"], "path": row["path"], "kind": row["kind"],
                    "branch": row["branch"], "main_path": row["main_path"]}),
            )?;
        }
        for row in rows(&plan["add_main"]) {
            api.post_noqueue(
                &format!("{base}/{}/paths", value(&row["slug"])),
                &json!({"host": plan["host"], "path": row["path"], "kind": "main"}),
            )?;
        }
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
            lines.push(format!(
                "  {label:<16} {:<28} {}",
                value(&row["slug"]),
                value(&row["path"])
            ));
        }
    }
    if plan["changes"] == 0 {
        lines.push("  nothing to change".to_owned());
    }
    output::emit(&plan, &lines.join("\n"), json_mode)
}

fn merged_slug<'a>(plan: &'a Value, slug: &'a str) -> &'a str {
    rows(&plan["merges"])
        .iter()
        .find(|row| row["from"] == slug)
        .map(|row| value(&row["into"]))
        .unwrap_or(slug)
}

fn rows(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn value(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
