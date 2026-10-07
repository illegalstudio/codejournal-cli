use crate::{api::Api, output, project_bootstrap};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub(crate) mod args;
mod code_hash;
mod format;
mod legacy;
mod review;
mod scan;

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
            if error.to_string().starts_with("API returned 404")
                || error.to_string().starts_with("API returned 405") =>
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
    if !api.offline() && args.after.is_none() {
        if !args.dry_run {
            applied =
                api.post(&format!("{base}/garden/maintenance"), &json!({}))?["applied"].clone();
        }
        let scan = scan::run(api, &base, args.dry_run)?;
        automatic = scan["automatic"].clone();
        partial = scan["partial"] == true;
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
