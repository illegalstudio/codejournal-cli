//! Reading a task works across projects, changing one is scoped to a project. When the task
//! belongs to another project, show the command that would change it from here.
use serde_json::{Value, json};

/// The owning project when it is not the one a change would be sent to.
fn owner<'a>(task: &'a Value, explicit_project: Option<&str>) -> Option<&'a str> {
    let owner = task["project_slug"]
        .as_str()
        .filter(|slug| !slug.is_empty())?;
    let current = crate::project::slug(explicit_project).ok();
    (current.as_deref() != Some(owner)).then_some(owner)
}

/// JSON fields and a text line for a task owned by another project.
pub fn hint(task: &Value, explicit_project: Option<&str>) -> Option<(Value, String)> {
    let owner = owner(task, explicit_project)?;
    let id: String = task["id"]
        .as_str()
        .unwrap_or("")
        .replace('-', "")
        .chars()
        .take(8)
        .collect();
    let base = task["body_version"]
        .as_str()
        .filter(|version| !version.is_empty())
        .unwrap_or("BODY_VERSION");
    let edit = format!("cj task edit {id} --project {owner} --base {base} --append-body TEXT");
    let readback = format!("cj task show {id}");
    let line = format!(
        "scope:     belongs to {owner}; commands that change it from here need --project {owner}, for example: {edit}"
    );
    Some((
        json!({"project": owner, "edit_command": edit, "readback_command": readback}),
        line,
    ))
}
