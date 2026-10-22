use super::{args::StartArgs, context, delivery, store, workspace};
use anyhow::{Result, ensure};
use rusqlite::params;
use serde_json::{Value, json};

pub fn run(
    args: StartArgs,
    project: Option<String>,
    server: Option<String>,
    launch: bool,
    offline: bool,
) -> Result<Value> {
    context::host_only()?;
    ensure!(
        std::env::var("CJ_DELEGATION_DEPTH")
            .unwrap_or_default()
            .is_empty(),
        "nested delegation is disabled"
    );
    ensure!(
        args.stall_timeout.is_finite() && args.stall_timeout >= 0.0,
        "stall timeout must be a finite nonnegative duration"
    );
    ensure!(
        args.diff_timeout.is_finite() && args.diff_timeout > 0.0,
        "diff timeout must be a finite positive duration"
    );
    let prompt = crate::input::body(args.prompt.clone(), args.prompt_file.clone())?;
    ensure!(
        prompt.chars().count() <= 100_000,
        "delegation prompt exceeds 100000 characters"
    );
    let cwd = std::env::current_dir()?
        .canonicalize()?
        .to_string_lossy()
        .into_owned();
    ensure!(
        cwd.chars().count() <= 2000,
        "delegation working directory exceeds 2000 characters"
    );
    ensure!(
        args.model
            .as_ref()
            .is_none_or(|model| model.chars().count() <= 100),
        "model name exceeds 100 characters"
    );
    let mut session = json!({"target": args.target.name(), "mode": args.mode.name(),
        "model": args.model.clone().unwrap_or_default(), "cwd": cwd, "task": prompt});
    if args.dry_run {
        session["id"] = json!("<dry-run>");
        return super::providers::dry_run(&session);
    }
    if launch {
        super::providers::available(args.target.name())?;
    }
    let (root, head) = if args.mode == super::args::Mode::Write {
        workspace::inspect(&cwd)?
    } else {
        (String::new(), String::new())
    };
    let mut captured = context::capture(project, server);
    captured["offline"] = json!(offline);
    let id = uuid::Uuid::new_v4().to_string();
    store::transaction(|db| {
        if args.mode == super::args::Mode::Write {
            workspace::ensure_writer_available(db)?;
        }
        let now = super::now();
        db.execute("INSERT INTO delegations (id,target,mode,cwd,model,task,status,created_at,updated_at,last_activity,stall_timeout,diff_timeout,workspace_root,baseline_sha,journal_context) VALUES (?,?,?,?,?,?,'created',?,?,?,?,?,?,?,?)",
            params![id, args.target.name(), args.mode.name(), cwd, args.model.unwrap_or_default(), prompt,
                now, now, now, args.stall_timeout, if root.is_empty() {0.0} else {args.diff_timeout}, root, head, captured.to_string()])?;
        store::event(
            db,
            &id,
            "system",
            "created",
            json!({"target": args.target.name(), "mode": args.mode.name()}),
            None,
            None,
        )?;
        session = store::get(db, &id)?;
        delivery::stage(db, &session, "created", "host", "")?;
        Ok(())
    })?;
    if launch {
        super::worker::launch(&id, false)
    } else {
        Ok(json!({"session": store::public(session)}))
    }
}
