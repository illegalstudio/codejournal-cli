use crate::api::Api;
use crate::plan_args::PlanAction;
use crate::{plan_changes, plan_read, plan_show, plan_write};
use anyhow::{Result, bail};

pub fn run(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    kind: &str,
    action: PlanAction,
    json_mode: bool,
) -> Result<()> {
    if kind != "docs"
        && matches!(
            &action,
            PlanAction::Create { global: true, .. } | PlanAction::List { global: true, .. }
        )
    {
        bail!("--global is only supported for docs");
    }
    if kind != "docs" && matches!(&action, PlanAction::List { local: true, .. }) {
        bail!("--local is only supported for docs");
    }
    match action {
        PlanAction::List {
            status,
            all,
            verbose,
            grep,
            path,
            all_projects,
            global,
            local,
        } => plan_read::list(
            api,
            tenant,
            project,
            kind,
            status,
            all,
            verbose,
            grep,
            path,
            all_projects,
            global,
            local,
            json_mode,
        ),
        PlanAction::Show {
            id,
            revision,
            history,
            all_revisions,
            before_revision,
            body,
            current_only: _,
        } => plan_show::show(
            api,
            tenant,
            kind,
            &id,
            revision,
            history,
            all_revisions,
            before_revision,
            body,
            json_mode,
        ),
        PlanAction::Create {
            global,
            title,
            body,
            body_file,
            status,
            not_before,
            refs,
            agent,
        } => plan_write::create(
            api, tenant, project, kind, global, title, body, body_file, status, not_before, refs,
            agent, json_mode,
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
            with_logs,
            note,
            agent,
        } => plan_changes::move_to(
            api, tenant, kind, &id, &target, with_logs, note, agent, json_mode,
        ),
        PlanAction::Step {
            id,
            number,
            done,
            undone: _,
            note,
        } => crate::plan_step::run(api, tenant, kind, &id, number, done, note, json_mode),
        PlanAction::Schedule { id, date } => {
            plan_changes::schedule(api, tenant, kind, &id, &date, json_mode)
        }
    }
}
