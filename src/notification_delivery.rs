use crate::config::Config;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::time::Duration;

pub fn configure(desktop: Option<String>, ntfy: Option<String>, json_mode: bool) -> Result<()> {
    let mut config = Config::load()?;
    if let Some(value) = desktop {
        config.notifications.desktop = value == "on";
    }
    if let Some(value) = ntfy {
        if value == "off" {
            config.notifications.ntfy_url = None;
        } else {
            let url = reqwest::Url::parse(&value).context("--ntfy needs a full topic URL")?;
            if !["http", "https"].contains(&url.scheme()) {
                bail!("--ntfy needs an HTTP topic URL");
            }
            config.notifications.ntfy_url = Some(value);
        }
    }
    config.save()?;
    if json_mode {
        println!(
            "{}",
            json!({"notifications": {
                "desktop": config.notifications.desktop, "ntfy_url": config.notifications.ntfy_url
            }})
        );
    } else {
        println!(
            "desktop: {}\nntfy:    {}",
            if config.notifications.desktop {
                "on"
            } else {
                "off"
            },
            config.notifications.ntfy_url.as_deref().unwrap_or("off")
        );
    }
    Ok(())
}

pub fn available() -> Result<Value> {
    let config = Config::load()?;
    let mut channels = vec!["dashboard bell"];
    if config.notifications.desktop {
        channels.push("desktop");
    }
    if config.notifications.ntfy_url.is_some() {
        channels.push("ntfy");
    }
    let note = (channels.len() == 1).then_some("desktop and ntfy delivery are off on this machine; the user sees the dashboard bell when the dashboard is open");
    Ok(
        json!({"channels": channels, "desktop": config.notifications.desktop,
        "ntfy": config.notifications.ntfy_url.is_some(), "dashboard_running": false, "note": note}),
    )
}

pub fn deliver(item: &Value, where_text: &str) -> Result<(Value, Vec<String>)> {
    let config = Config::load()?;
    let mut notes = Vec::new();
    let mut channels = vec!["dashboard bell"];
    let title = item["title"].as_str().unwrap_or("");
    let kind = item["kind"].as_str().unwrap_or("info");
    let body = item["body"].as_str().unwrap_or("");
    let agent = item["agent"].as_str().unwrap_or("agent");
    if config.notifications.desktop {
        channels.push("desktop");
        let urgency = if ["needs_input", "error"].contains(&kind) {
            "critical"
        } else {
            "normal"
        };
        let message = format!("{agent} in {where_text}\n{body}");
        if let Err(error) = crate::desktop_delivery::send(
            &format!("cj: {title}"),
            &message.chars().take(500).collect::<String>(),
            urgency,
        ) {
            notes.push(error);
        }
    }
    if let Some(url) = &config.notifications.ntfy_url {
        channels.push("ntfy");
        let priority = if ["needs_input", "error"].contains(&kind) {
            "high"
        } else if kind == "info" {
            "low"
        } else {
            "default"
        };
        let text = if body.is_empty() {
            format!("{title} ({agent}, {where_text})")
        } else {
            body.to_owned()
        };
        let response = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()?
            .post(url)
            .header("Title", format!("cj: {title}"))
            .header("Priority", priority)
            .header("Tags", kind)
            .body(text.chars().take(4000).collect::<String>())
            .send();
        match response {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => notes.push(format!("ntfy delivery failed: HTTP {}", response.status())),
            Err(error) => notes.push(format!("ntfy delivery failed: {error}")),
        }
    }
    let note = (channels.len() == 1).then_some("desktop and ntfy delivery are off on this machine; the user sees the dashboard bell when the dashboard is open");
    Ok((
        json!({"channels": channels, "desktop": config.notifications.desktop,
        "ntfy": config.notifications.ntfy_url.is_some(), "dashboard_running": false, "note": note}),
        notes,
    ))
}

pub fn send_digest(markdown: &str, label: &str) -> Result<()> {
    let url = Config::load()?
        .notifications
        .ntfy_url
        .context("--send needs an ntfy URL: `cj notifications config --ntfy URL`")?;
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()?
        .post(url)
        .header("Title", format!("cj digest: {label}"))
        .header("Markdown", "yes")
        .header("Tags", "digest")
        .body(markdown.chars().take(4000).collect::<String>())
        .send()?
        .error_for_status()?;
    Ok(())
}
