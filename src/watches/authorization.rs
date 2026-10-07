use crate::{api::Api, outbox, watch_state};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Authorization {
    scope: String,
    pub endpoint: String,
    pub command: Vec<String>,
    pub cwd: String,
    pub timeout: Option<u64>,
    #[serde(default)]
    pub runner_id: Option<String>,
}

fn file(id: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(id).context("invalid watch ID")?;
    Ok(watch_state::directory()?.join(format!("{id}.authorized.json")))
}

pub fn save(
    api: &Api,
    tenant: &str,
    endpoint: &str,
    id: &str,
    command: &[String],
    cwd: &str,
    timeout: Option<u64>,
) -> Result<()> {
    let target = file(id)?;
    let authorization = Authorization {
        scope: outbox::fingerprint(&api.server, tenant, &api.token),
        endpoint: endpoint.to_owned(),
        command: command.to_vec(),
        cwd: cwd.to_owned(),
        timeout,
        runner_id: Some(uuid::Uuid::new_v4().to_string()),
    };
    let mut temporary = tempfile::NamedTempFile::new_in(watch_state::directory()?)?;
    temporary.write_all(&serde_json::to_vec(&authorization)?)?;
    temporary.as_file().sync_all()?;
    temporary.persist_noclobber(target)?;
    watch_state::sync_directory()
}

pub fn load(api: &Api, tenant: &str, endpoint: &str, id: &str) -> Result<Authorization> {
    let authorization: Authorization = serde_json::from_slice(
        &fs::read(file(id)?).context("watch has no unused local execution authorization")?,
    )?;
    if authorization.scope != outbox::fingerprint(&api.server, tenant, &api.token)
        || authorization.endpoint != endpoint
        || authorization.command.is_empty()
    {
        bail!("watch execution authorization belongs to another local scope");
    }
    Ok(authorization)
}

pub fn pending(api: &Api, tenant: &str) -> Result<Vec<(String, Authorization)>> {
    let mut pending = Vec::new();
    for entry in fs::read_dir(watch_state::directory()?)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = name.strip_suffix(".authorized.json") else {
            continue;
        };
        let authorization: Authorization = match serde_json::from_slice(&fs::read(entry.path())?) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if authorization.scope == outbox::fingerprint(&api.server, tenant, &api.token)
            && authorization
                .endpoint
                .starts_with(&format!("/api/v1/tenants/{tenant}/projects/"))
        {
            pending.push((id.to_owned(), authorization));
        }
    }
    Ok(pending)
}

pub fn take(api: &Api, tenant: &str, endpoint: &str, id: &str) -> Result<Authorization> {
    load(api, tenant, endpoint, id)?;
    let target = file(id)?;
    let claimed = target.with_extension(format!("claimed-{}", uuid::Uuid::new_v4()));
    fs::rename(&target, &claimed).context("watch has no unused local execution authorization")?;
    let bytes = fs::read(&claimed);
    fs::remove_file(claimed)?;
    watch_state::sync_directory()?;
    let authorization: Authorization = serde_json::from_slice(&bytes?)?;
    if authorization.scope != outbox::fingerprint(&api.server, tenant, &api.token)
        || authorization.endpoint != endpoint
        || authorization.command.is_empty()
    {
        bail!("watch execution authorization belongs to another local scope");
    }
    Ok(authorization)
}

pub fn verify(authorization: &Authorization, watch: &Value, id: &str) -> Result<()> {
    let remote: Value =
        serde_json::from_str(watch["command"].as_str().context("watch command missing")?)?;
    if watch["id"] != id
        || remote != crate::api::sanitized(json!(authorization.command))
        || watch["cwd"] != crate::api::sanitized(json!(authorization.cwd))
        || watch["timeout"] != json!(authorization.timeout)
    {
        bail!("remote watch definition differs from the locally authorized command");
    }
    Ok(())
}

pub fn remove(id: &str) {
    if let Ok(target) = file(id) {
        let _ = fs::remove_file(target);
        let _ = watch_state::sync_directory();
    }
}
