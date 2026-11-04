use crate::api::Api;

pub fn missing(api: &Api, tenant: &str, id: &str, error: anyhow::Error) -> anyhow::Error {
    if !api.offline() {
        if let Ok(found) = api.get(&format!("/api/v1/tenants/{tenant}/records/{id}")) {
            if let Some(items) = found["matches"].as_array() {
                if items.len() == 1 {
                    let item = &items[0];
                    if let (Some(kind), Some(command)) =
                        (item["type"].as_str(), item["command"].as_str())
                    {
                        return error.context(format!(
                            "This ID identifies a {kind}; read it with {command}"
                        ));
                    }
                }
            }
        }
    }
    error.context("cj show reads knowledge entries; use cj log show ID for a work log, or cj task show ID, cj plan show ID, cj doc show ID or cj feedback show ID for other records")
}
