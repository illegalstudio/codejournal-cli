use crate::api::Api;
use crate::plan_args::PlanAction;
use crate::{plan_changes, plan_read, plan_write};
use anyhow::Result;

pub fn run(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    kind: &str,
    action: PlanAction,
    json_mode: bool,
) -> Result<()> {
    match action {
        PlanAction::List {
            status,
            grep,
            path,
            all_projects,
        } => plan_read::list(
            api,
            tenant,
            project,
            kind,
            status,
            grep,
            path,
            all_projects,
            json_mode,
        ),
        PlanAction::Show {
            id,
            revision,
            history,
            body,
        } => plan_read::show(api, tenant, kind, &id, revision, history, body, json_mode),
        PlanAction::Create {
            title,
            body,
            body_file,
            status,
            not_before,
            refs,
            agent,
        } => plan_write::create(
            api, tenant, project, kind, title, body, body_file, status, not_before, refs, agent,
            json_mode,
        ),
        PlanAction::Update {
            id,
            title,
            body,
            body_file,
            refs,
            note,
            based_on,
            agent,
        } => plan_write::update(
            api, tenant, kind, &id, title, body, body_file, refs, note, based_on, agent, json_mode,
        ),
        PlanAction::Status {
            id,
            status,
            note,
            agent,
        } => plan_changes::status(api, tenant, kind, &id, &status, note, agent, json_mode),
        PlanAction::Move {
            id,
            target,
            note,
            agent,
        } => plan_changes::move_to(api, tenant, kind, &id, &target, note, agent, json_mode),
        PlanAction::Schedule { id, date } => {
            plan_changes::schedule(api, tenant, &id, &date, json_mode)
        }
    }
}
