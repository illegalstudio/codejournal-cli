mod backend;
mod paths;
mod probe;
mod protection;
pub use protection::Protection;
#[cfg(target_os = "linux")]
mod resolver;
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;

pub fn prepare(session: &Value, dry: bool) -> Protection {
    if super::super::text(session, "mode") != "read-only" {
        let mut result = Protection::disabled("");
        result.report["status"] = json!("not_requested");
        return result;
    }
    prepare_available(session, dry, false)
        .unwrap_or_else(|error| Protection::disabled(&format!("{error:#}")))
}

fn prepare_available(session: &Value, dry: bool, skip_system: bool) -> Result<Protection> {
    let scratch = if dry {
        None
    } else {
        Some(
            tempfile::Builder::new()
                .prefix("sandbox-")
                .tempdir_in(super::super::store::directory()?)?,
        )
    };
    let root = scratch
        .as_ref()
        .map_or(Path::new("<private scratch directory>"), |dir| dir.path());
    let writable = paths::writable(session, root, dry)?;
    let (mut wrapper, source) = match backend::select(root, &writable, session, dry, skip_system) {
        Ok(value) => value,
        Err(error) => {
            let mut protection = Protection::disabled(&format!("{error:#}"));
            protection.scratch = scratch;
            return Ok(protection);
        }
    };
    wrapper.current_dir(super::super::text(session, "cwd"));
    let marker = if dry {
        None
    } else {
        Some(
            tempfile::Builder::new()
                .prefix("provider-start-")
                .tempfile_in(root)?,
        )
    };
    Ok(Protection {
        report: json!({"status":if dry { "not_checked" } else { "active" },
            "enforced":if dry { Value::Null } else { json!(true) },
            "backend":backend::name(),"source":source,"reason":null}),
        scratch,
        marker,
        wrapper: Some(wrapper),
    })
}

pub fn retry(session: &Value, report: &Value, reason: &str) -> Protection {
    #[cfg(target_os = "linux")]
    if report["source"] == "system" {
        let mut protection = prepare_available(session, false, true)
            .unwrap_or_else(|error| Protection::disabled(&format!("{error:#}")));
        protection.report["previous_failure"] = json!(
            crate::secret_redaction::text(reason)
                .0
                .chars()
                .take(2048)
                .collect::<String>()
        );
        if protection.report["status"] == "disabled" {
            protection.report["reason"] = json!(format!(
                "{reason}; {}",
                super::super::text(&protection.report, "reason")
            ));
        }
        return protection;
    }
    let _ = (session, report);
    Protection::disabled(reason)
}

pub fn notice(report: &Value) {
    if report["status"] == "disabled" {
        eprintln!(
            "Warning: proceeding without filesystem sandbox: {}. Read-only instructions still apply.",
            super::super::text(report, "reason")
        );
    }
}

pub fn summary(report: &Value) -> String {
    match super::super::text(report, "status") {
        "active" => format!(
            "Filesystem sandbox active: {} ({})",
            super::super::text(report, "backend"),
            super::super::text(report, "source")
        ),
        "disabled" => format!(
            "Proceeding without filesystem sandbox: {}. Read-only instructions still apply.",
            super::super::text(report, "reason")
        ),
        "not_requested" => "Filesystem sandbox not requested for this write delegation".to_owned(),
        _ => "Filesystem sandbox protection has not been checked".to_owned(),
    }
}
