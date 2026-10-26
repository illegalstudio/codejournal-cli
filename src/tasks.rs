use crate::api::Api;
use crate::task_args::TaskAction;
use crate::{task_add, task_change, task_read, task_show};
use anyhow::Result;

pub fn run(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    action: TaskAction,
    json_mode: bool,
) -> Result<()> {
    match action {
        TaskAction::Add(args) => task_add::run(api, tenant, project, args, json_mode),
        TaskAction::List(args) => task_read::list(api, tenant, project, args, json_mode),
        TaskAction::Show { id } => task_show::show(api, tenant, &id, json_mode),
        TaskAction::Start { id, note, agent } => {
            task_change::status(api, tenant, project, &id, "start", note, agent, json_mode)
        }
        TaskAction::Done { id, note, agent } => {
            task_change::status(api, tenant, project, &id, "done", note, agent, json_mode)
        }
        TaskAction::Dismiss { id, note, agent } => {
            task_change::status(api, tenant, project, &id, "dismiss", note, agent, json_mode)
        }
        TaskAction::Reopen { id, note, agent } => {
            task_change::status(api, tenant, project, &id, "reopen", note, agent, json_mode)
        }
        TaskAction::Note {
            id,
            note,
            body_file,
            agent,
        } => task_change::note(api, tenant, project, &id, note, body_file, agent, json_mode),
        TaskAction::Link { id, refs, agent } => {
            task_change::refs(api, tenant, project, &id, refs, agent, false, json_mode)
        }
        TaskAction::Unlink { id, refs, agent } => {
            task_change::refs(api, tenant, project, &id, refs, agent, true, json_mode)
        }
        TaskAction::Edit(args) => task_change::edit(api, tenant, project, args, json_mode),
    }
}
