use serde_json::Value;

pub fn list(result: &Value) -> String {
    let lines = result["watches"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|watch| {
            let id = short(watch["id"].as_str().unwrap_or(""));
            let status = watch["status"].as_str().unwrap_or("?");
            let started = watch["started_at"].as_str().unwrap_or("");
            let started = started
                .get(..started.len().min(16))
                .unwrap_or("")
                .replace('T', " ");
            let title = watch["title"].as_str().unwrap_or("");
            let command = command(&watch["command"]);
            let command = if command.is_empty() {
                String::new()
            } else {
                format!("  [{command}]")
            };
            let exit = if status == "finished" {
                format!("  exit {}", watch["exit_code"])
            } else {
                String::new()
            };
            format!("  {id}  {status:<10} {started}  {title}{command}{exit}")
        })
        .collect::<Vec<_>>();
    if lines.is_empty() {
        "  (no running watches; --all shows finished ones)".to_owned()
    } else {
        lines.join("\n")
    }
}

pub fn cancel(watch: &Value, cancelled: bool) -> String {
    let id = short(watch["id"].as_str().unwrap_or(""));
    let title = watch["title"].as_str().unwrap_or("");
    if cancelled {
        format!("Cancelled watch {id}: {title}")
    } else {
        format!(
            "Watch {id} is not running ({}).",
            watch["status"].as_str().unwrap_or("?")
        )
    }
}

fn short(id: &str) -> String {
    id.chars()
        .filter(|character| *character != '-')
        .take(8)
        .collect()
}

fn command(value: &Value) -> String {
    let parsed = value
        .as_str()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok());
    let value = parsed.as_ref().unwrap_or(value);
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(" ")
}
