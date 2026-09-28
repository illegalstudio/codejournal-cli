use crate::api::Api;
use crate::project_args::ProjectAction;
use crate::{
    attribution, checkout_identity, git, output, project, project_detail, project_paths,
    project_provides,
};
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
            let checkout = checkout_identity::current();
            let mut body = json!({"slug": slug, "name": project::name(name.as_deref())?,
                "remote_url": checkout.as_ref().and_then(|item| item.origin.clone()),
                "provides": {"auto": project_provides::detect(), "manual": []}});
            if let Some(checkout) = &checkout {
                body["host"] = json!(attribution::host());
                body["path"] = json!(checkout.root);
                body["kind"] = json!(checkout.kind);
                body["branch"] = json!(checkout.branch);
                body["main_path"] = json!(checkout.main_path());
            }
            let result = api.post(&base, &body)?;
            let recorded_slug = result["project"]["slug"].as_str().unwrap_or(&slug);
            output::emit(
                &result,
                &format!("Initialized project {recorded_slug}."),
                json_mode,
            )
        }
        ProjectAction::Show => {
            let result = api.get(&format!("{base}/{slug}"))?;
            output::emit(
                &result,
                &project_detail::format(&result["project"]),
                json_mode,
            )
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
                input.insert("slug".into(), json!(slug.trim().to_ascii_lowercase()));
            }
            if let Some(name) = name {
                input.insert("name".into(), json!(name.trim()));
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
                let mut manual = provides
                    .split(',')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_ascii_lowercase)
                    .collect::<Vec<_>>();
                manual.sort();
                manual.dedup();
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
                let updated_slug = result["project"]["slug"].as_str().unwrap_or(&slug);
                project_paths::record(api, tenant, updated_slug, &path)?;
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
