use crate::{api::Api, output, project_bootstrap};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub(crate) mod args;
mod code_hash;
mod directory_files;
mod format;
mod legacy;
mod review;
mod scan;
mod scan_batches;

pub fn run(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    mut args: args::GardenArgs,
    json_mode: bool,
) -> Result<()> {
    if api.offline() && !args.dry_run {
        bail!("Garden needs an online connection; use --dry-run for a cached preview");
    }
    let slug = project_bootstrap::resolved_slug(api, tenant, project)?;
    let base = format!("/api/v1/tenants/{tenant}/projects/{slug}");
    if let Some(args::GardenAction::Review(action)) = args.action.take() {
        if args.dry_run || args.all || args.after.is_some() || args.limit != 5 {
            bail!("List options cannot be combined with garden review");
        }
        return review::run(api, &base, action, json_mode);
    }
    let limit = if args.all { 25 } else { args.limit };
    let mut path = format!("{base}/garden/review?limit={limit}");
    if let Some(after) = &args.after {
        path.push_str(&format!("&after={after}"));
    }
    let mut result = match api.get(&path) {
        Ok(value) if value["findings"].is_array() && value["counts"].is_object() => value,
        Ok(_) => {
            if args.after.is_some() {
                bail!("This server does not support persistent garden reviews");
            }
            return legacy::run(
                api,
                tenant,
                project,
                args.dry_run,
                json_mode,
                args.all,
                args.limit as usize,
            );
        }
        Err(error)
            if error
                .downcast_ref::<crate::api::response_error::ResponseError>()
                .is_some_and(|response| {
                    [
                        reqwest::StatusCode::NOT_FOUND,
                        reqwest::StatusCode::METHOD_NOT_ALLOWED,
                    ]
                    .contains(&response.status)
                }) =>
        {
            if args.after.is_some() {
                bail!("This server does not support persistent garden reviews");
            }
            return legacy::run(
                api,
                tenant,
                project,
                args.dry_run,
                json_mode,
                args.all,
                args.limit as usize,
            );
        }
        Err(error) => return Err(error),
    };
    let mut applied = json!([]);
    let mut automatic = json!({});
    let mut partial = false;
    let mut snapshots =
        json!({"code_scan_partial": false, "snapshot_failure_count": 0, "scan_failures": []});
    if !api.offline() && args.after.is_none() {
        if !args.dry_run {
            applied =
                api.post(&format!("{base}/garden/maintenance"), &json!({}))?["applied"].clone();
        }
        let scan = scan::run(api, &base, args.dry_run)?;
        automatic = scan["automatic"].clone();
        partial = scan["partial"] == true;
        for key in [
            "code_scan_partial",
            "snapshot_failure_count",
            "scan_failures",
        ] {
            snapshots[key] = scan[key].clone();
        }
        result = if args.dry_run {
            preview(scan, args.limit as usize, args.all)
        } else {
            api.get(&path)?
        };
    }
    if args.all && !args.dry_run {
        complete(api, &base, &mut result)?;
    }
    result["project"] = json!(slug);
    result["dry_run"] = json!(args.dry_run);
    result["applied"] = applied;
    result["automatic"] = automatic;
    result["partial"] = json!(partial);
    result["topic_analysis_partial"] = json!(partial);
    for key in [
        "code_scan_partial",
        "snapshot_failure_count",
        "scan_failures",
    ] {
        result[key] = snapshots[key].clone();
    }
    result["review_queue_complete"] = json!(
        result["next"].is_null()
            && (!args.dry_run
                || args.all
                || result["counts"]["pending"].as_u64().unwrap_or(0)
                    <= result["findings"].as_array().map_or(0, Vec::len) as u64)
    );
    output::emit(&result, &format::render(&result), json_mode)
}

fn preview(mut result: Value, limit: usize, all: bool) -> Value {
    let mut rows = result["findings"].as_array().cloned().unwrap_or_default();
    if !all {
        rows.truncate(limit);
    }
    result["next"] = Value::Null;
    result["findings"] = json!(rows);
    if let Some(value) = result.as_object_mut() {
        value.remove("reviewed");
        value.remove("deferred");
    }
    result
}

fn complete(api: &Api, base: &str, result: &mut Value) -> Result<()> {
    let mut rows = result["findings"].as_array().cloned().unwrap_or_default();
    let mut next = result["next"].as_str().map(str::to_owned);
    let mut seen = std::collections::HashSet::new();
    while let Some(after) = next {
        if !seen.insert(after.clone()) {
            bail!("Invalid garden continuation");
        }
        let page = api.get(&format!("{base}/garden/review?limit=25&after={after}"))?;
        rows.extend(page["findings"].as_array().into_iter().flatten().cloned());
        next = page["next"].as_str().map(str::to_owned);
    }
    result["findings"] = json!(rows);
    result["next"] = Value::Null;
    Ok(())
}
