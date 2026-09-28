use crate::api::Api;
use crate::project_args::ProjectAction;
use crate::{attribution, git, output, project, project_paths, project_provides};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub fn run(
    api: &Api,
    tenant: &str,
    explicit: Option<&str>,
    action: ProjectAction,
    json_mode: bool,
) -> Result<()> {
    let slug = project::slug(explicit)?;
    let base = format!("/api/v1/tenants/{tenant}/projects");
    match action {
        ProjectAction::List => crate::project_list::run(api, tenant, json_mode),
        ProjectAction::Init { name } => {
            let remote = git::output(&["remote", "get-url", "origin"])
                .map(|url| project::normalize_remote(&url));
            let result = api.post(&base, &json!({"slug": slug, "name": project::name(name.as_deref())?,
                "remote_url": remote, "provides": {"auto": project_provides::detect(), "manual": []}}))?;
            if let Ok(path) = project_paths::current() {
                let _ = project_paths::record(api, tenant, &slug, &path);
            }
            output::emit(&result, &format!("Initialized project {slug}."), json_mode)
        }
        ProjectAction::Show => {
            let result = api.get(&format!("{base}/{slug}"))?;
            let p = &result["project"];
            let mut lines = vec![
                format!("slug:            {slug}"),
                format!("name:            {}", value(&p["name"])),
                format!(
                    "remote:          {}",
                    p["remote_url"].as_str().unwrap_or("(none)")
                ),
                format!("id:              {}", value(&p["id"])),
                format!("created:         {}", value(&p["created_at"])),
                "checkouts:".to_owned(),
            ];
            for path in p["paths"].as_array().into_iter().flatten() {
                lines.push(format!(
                    "  {:<24} {:<10} {}",
                    value(&path["kind"]),
                    value(&path["host"]),
                    value(&path["path"])
                ));
            }
            output::emit(&result, &lines.join("\n"), json_mode)
        }
        ProjectAction::Edit {
            slug: new_slug,
            name,
            remote,
            no_remote,
            from_git,
            provides,
        } => {
            let mut input = serde_json::Map::new();
            if let Some(slug) = new_slug.as_ref() {
                input.insert("slug".into(), json!(slug));
            }
            if let Some(name) = name {
                input.insert("name".into(), json!(name));
            }
            if let Some(remote) = remote {
                input.insert(
                    "remote_url".into(),
                    json!(project::normalize_remote(&remote)),
                );
            }
            if no_remote {
                input.insert("remote_url".into(), Value::Null);
            }
            if from_git {
                let raw = git::output(&["remote", "get-url", "origin"]);
                if let Some(raw) = raw {
                    input.insert("remote_url".into(), json!(project::normalize_remote(&raw)));
                }
                input.insert("name".into(), json!(project::name(None)?));
            }
            if let Some(provides) = provides {
                let existing = api.get(&format!("{base}/{slug}"))?;
                let current = &existing["project"]["provides"];
                let auto = current["auto"].as_array().cloned().unwrap_or_default();
                let manual = provides
                    .split(',')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_ascii_lowercase)
                    .collect::<Vec<_>>();
                input.insert("provides".into(), json!({"auto": auto, "manual": manual}));
            }
            if input.is_empty() {
                bail!(
                    "nothing to change; pass --slug, --name, --remote, --no-remote, --provides, or --from-git"
                );
            }
            let result = api.patch(&format!("{base}/{slug}"), &Value::Object(input))?;
            if from_git {
                let path = project_paths::current()?;
                project_paths::record(api, tenant, new_slug.as_deref().unwrap_or(&slug), &path)?;
            }
            output::emit(
                &result,
                &format!(
                    "Updated project {}.",
                    result["project"]["slug"].as_str().unwrap_or(&slug)
                ),
                json_mode,
            )
        }
        ProjectAction::PathAdd { path } => project_paths::add(api, tenant, &slug, &path, json_mode),
        ProjectAction::PathRemove { path, host } => project_paths::remove(
            api,
            tenant,
            &slug,
            &path,
            host.unwrap_or_else(attribution::host),
            json_mode,
        ),
        ProjectAction::Merge { source, target } => {
            let result = api.post(
                &format!("{base}/merge"),
                &json!({"source": source, "into": target}),
            )?;
            output::emit(
                &result,
                &format!(
                    "Merged {source} into {target}: {} entries moved.",
                    result["entries"]
                ),
                json_mode,
            )
        }
    }
}

fn value(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
