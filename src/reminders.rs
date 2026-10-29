use crate::api::Api;
use crate::reminder_args::{ReminderAction, ReminderAdd};
use crate::{input, output, project, reminder_format};
use anyhow::{Result, bail};
use serde_json::{Value, json};

mod read;

pub fn run(
    api: &Api,
    tenant: &str,
    project: Option<&str>,
    action: ReminderAction,
    json_mode: bool,
) -> Result<()> {
    let root = format!("/api/v1/tenants/{tenant}/reminders");
    match action {
        ReminderAction::Add(args) => add(api, &root, project, args, json_mode),
        ReminderAction::List(args) => read::list(api, &root, project, args, json_mode),
        ReminderAction::Show { id } => {
            let result = api.get(&endpoint(&root, &id)?)?;
            output::emit(&result, &reminder_format::detail(&result), json_mode)
        }
        ReminderAction::Close { id } => {
            change(api, &root, &id, json!({"action": "close"}), json_mode)
        }
        ReminderAction::Reschedule { id, at, timezone } => change(
            api,
            &root,
            &id,
            json!({"action": "reschedule", "at": at, "timezone": timezone}),
            json_mode,
        ),
        ReminderAction::Claim { id } => {
            if api.offline() {
                bail!("reminder announcements require an online claim; nothing was queued");
            }
            let result = api.post_noqueue(
                &format!("{}/claim", endpoint(&root, &id)?),
                &json!({"project": project::slug(project).ok()}),
            )?;
            let message = if result["claimed"] == true {
                format!(
                    "{}\nDelivery token: {}\nClaim expires: {}",
                    reminder_format::detail(&result),
                    result["delivery_token"].as_str().unwrap_or(""),
                    result["claim_until"].as_str().unwrap_or("")
                )
            } else {
                "Reminder is not available for announcement. Do not announce it.".into()
            };
            output::emit(&result, &message, json_mode)
        }
        ReminderAction::Presented { id, delivery_token } => {
            if api.offline() {
                bail!("recording an announcement requires an online response; nothing was queued");
            }
            let result = api.post_noqueue(
                &format!("{}/presented", endpoint(&root, &id)?),
                &json!({"delivery_token": delivery_token}),
            )?;
            output::emit(
                &result,
                "Announcement recorded. The reminder remains open and may be announced again after 24 hours.",
                json_mode,
            )
        }
    }
}

fn endpoint(root: &str, id: &str) -> Result<String> {
    let compact = id.replace('-', "");
    if !(8..=32).contains(&compact.len()) || !compact.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!(
            "use the reminder UUID or an unambiguous prefix of at least eight hexadecimal characters"
        );
    }
    Ok(format!("{root}/{id}"))
}

fn add(
    api: &Api,
    root: &str,
    project: Option<&str>,
    args: ReminderAdd,
    json_mode: bool,
) -> Result<()> {
    let slug = if args.global {
        None
    } else {
        Some(project::slug(project)?)
    };
    let body = input::optional_body(args.body, args.body_file)?;
    let result = api.post(
        root,
        &json!({"project": slug, "title": args.title, "body": body, "at": args.at,
        "timezone": args.timezone, "reference_url": args.reference_url,
        "delivery_scope": if args.project_only { "project" } else { "workspace" }}),
    )?;
    output::emit(
        &result,
        &format!("Reminder saved:\n{}", reminder_format::detail(&result)),
        json_mode,
    )
}

fn change(api: &Api, root: &str, id: &str, mut payload: Value, json_mode: bool) -> Result<()> {
    let path = endpoint(root, id)?;
    let current = if api.offline() {
        api.get(&path)?
    } else {
        api.get_fresh(&path)?
    };
    let base = current["reminder"]["revision"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("reminder revision is missing"))?;
    payload["base"] = json!(base);
    let result = api.patch(&path, &payload)?;
    output::emit(&result, &reminder_format::detail(&result), json_mode)
}
