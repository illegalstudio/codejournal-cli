use super::{noun, path, short};
use crate::api::Api;
use crate::{attribution, input, refs};
use anyhow::{Result, bail, ensure};
use serde_json::{Value, json};

/// What an update changes; absent parts keep their stored value.
pub struct Content {
    pub title: Option<String>,
    pub body: Option<String>,
    pub body_file: Option<String>,
    /// Replaces the whole reference list.
    pub refs: Vec<String>,
    /// Extends the reference list.
    pub add_refs: Vec<String>,
    /// Replaces the topics a doc is declared to cover; an empty list clears them.
    pub topics: Option<Vec<String>>,
    pub note: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    api: &Api,
    tenant: &str,
    kind: &str,
    id: &str,
    content: Content,
    based_on: Option<u32>,
    agent: Option<String>,
    json_mode: bool,
) -> Result<()> {
    let markdown = input::optional_body(content.body, content.body_file)?;
    crate::plan_validation::body(&markdown)?;
    ensure!(
        content.topics.is_none() || kind == "docs",
        "--topics and --clear-topics are only supported for docs"
    );
    if content.title.is_none()
        && markdown.is_empty()
        && content.refs.is_empty()
        && content.add_refs.is_empty()
        && content.topics.is_none()
        && content.note.as_deref().is_none_or(str::is_empty)
    {
        bail!("nothing to update; pass Markdown, --title, --ref, --add-ref, --topics, or --note");
    }
    let mut payload = json!({"note": content.note, "based_on": based_on,
        "agent": attribution::agent(agent.as_deref()), "host": attribution::host()});
    if let Some(title) = content.title {
        payload["title"] = Value::String(title);
    }
    if !markdown.is_empty() {
        payload["body"] = Value::String(markdown);
    }
    if !content.refs.is_empty() {
        payload["refs"] = json!(refs::parse_all(&content.refs)?);
    }
    if !content.add_refs.is_empty() {
        payload["add_refs"] = json!(refs::parse_all(&content.add_refs)?);
    }
    if let Some(topics) = &content.topics {
        payload["topics"] = json!(topics);
    }
    let result = api.patch(&path(tenant, kind, id), &payload)?;
    let item = &result[noun(kind)];
    let mut text = format!(
        "{} {} is now at revision {}.",
        noun(kind),
        short(item["id"].as_str().unwrap_or(id)),
        item["revision"]
    );
    let changes_refs = !content.refs.is_empty() || !content.add_refs.is_empty();
    match (
        result["refs_before"].as_u64(),
        result["refs_after"].as_u64(),
    ) {
        (Some(before), Some(after)) => {
            text.push_str(&refs_report(before, after, !content.refs.is_empty()))
        }
        _ if !content.add_refs.is_empty() => bail!(
            "this server does not support --add-ref and left the references unchanged; update the server, or resend the complete list with --ref"
        ),
        _ if changes_refs => text.push_str(" The reference list was replaced."),
        _ => {}
    }
    if result["conflict"].as_bool().unwrap_or(false) {
        text.push_str(" Warning: the base revision changed; review history.");
    }
    crate::plan_show::size::emit(&result, &text, json_mode)
}

/// Says what happened to the reference list, so a replacement that dropped refs is visible.
fn refs_report(before: u64, after: u64, replaced: bool) -> String {
    if !replaced {
        return format!(" Refs: {before} before, {after} now.");
    }
    let mut text = format!(" Refs: replaced {before} with {after}.");
    if after < before {
        text.push_str(
            " --ref replaces the whole list; use --add-ref to keep the existing references.",
        );
    }
    text
}

#[cfg(test)]
mod tests {
    use super::refs_report;

    #[test]
    fn a_replacement_that_drops_references_says_so() {
        assert_eq!(refs_report(10, 12, false), " Refs: 10 before, 12 now.");
        assert_eq!(refs_report(2, 3, true), " Refs: replaced 2 with 3.");
        assert!(refs_report(10, 2, true).contains("replaced 10 with 2. --ref replaces"));
    }
}
