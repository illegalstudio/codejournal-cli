use crate::{api::Api, output};
use anyhow::{Result, ensure};

pub fn run(
    api: &Api,
    tenant: &str,
    slug: &str,
    query: &str,
    all: bool,
    json_mode: bool,
) -> Result<()> {
    ensure!(
        !query.trim().is_empty() && query.chars().count() <= 100,
        "topic query must contain 1 to 100 characters"
    );
    let base = if all {
        format!("/api/v1/tenants/{tenant}/topics")
    } else {
        format!("/api/v1/tenants/{tenant}/projects/{slug}/topics")
    };
    let url = reqwest::Url::parse_with_params("http://local/", &[("grep", query)])?;
    let result = api.get(&format!("{base}?{}", url.query().unwrap_or("")))?;
    ensure!(
        result["filter_applied"] == true,
        "this server does not support topic-name queries; update the server"
    );
    let lines = result["topics"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|item| {
            format!(
                "  {} ({})",
                item["name"].as_str().unwrap_or(""),
                item["count"]
            )
        })
        .collect::<Vec<_>>();
    output::emit(
        &result,
        &if lines.is_empty() {
            "  (no matching topics)".to_owned()
        } else {
            lines.join("\n")
        },
        json_mode,
    )
}
