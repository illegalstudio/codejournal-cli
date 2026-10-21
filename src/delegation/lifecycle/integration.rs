use super::super::{context, delivery, now, process, store, text, workspace};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

pub fn run(id: &str) -> Result<Value> {
    context::host_only()?;
    let initial = store::get(&store::open()?, id)?;
    validate(&initial)?;
    let head = if initial["integrated_at"].is_null() {
        Some(prepare(&initial)?)
    } else {
        None
    };
    store::transaction(|db| {
        let session = store::get(db, id)?;
        validate(&session)?;
        if session["integrated_at"].is_null() {
            let head = head
                .as_deref()
                .context("integration commit was not verified")?;
            store::event(
                db,
                id,
                "system",
                "integrated",
                json!({"commit_sha": head}),
                None,
                Some("host-integration"),
            )?;
            store::update(db, id, json!({"integrated_at": now(), "updated_at": now()}))?;
            delivery::stage(db, &store::get(db, id)?, "integrated", "host", "")?;
        }
        Ok(json!({"session": store::public(store::get(db, id)?)}))
    })
}

fn validate(session: &Value) -> Result<()> {
    ensure!(
        text(session, "mode") == "write" && text(session, "status") == "completed",
        "only a completed write delegation can be integrated"
    );
    Ok(())
}

fn prepare(session: &Value) -> Result<String> {
    ensure!(
        !process::matches(&session["guest_pid"], &session["guest_identity"]),
        "wait for the guest process to exit before integration"
    );
    ensure!(
        workspace::changed(&session, false)?.is_empty(),
        "review and commit the delegated result before integration"
    );
    let head = workspace::git(
        text(&session, "workspace_root"),
        &["rev-parse", "--verify", "HEAD"],
    )?;
    ensure!(
        head != text(&session, "baseline_sha"),
        "the host must create a commit before integration"
    );
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(text(&session, "workspace_root"))
        .args([
            "merge-base",
            "--is-ancestor",
            text(&session, "baseline_sha"),
            &head,
        ])
        .status()?;
    ensure!(
        status.success(),
        "integration commit must descend from the delegated baseline"
    );
    Ok(head)
}
