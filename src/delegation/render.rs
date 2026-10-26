use serde_json::{Value, json};

fn session(value: &mut Value) {
    if let Some(map) = value.as_object_mut() {
        if let Some(task) = map.remove("task") {
            map.insert(
                "task_title".to_owned(),
                json!(
                    task.as_str()
                        .unwrap_or("")
                        .lines()
                        .next()
                        .unwrap_or("")
                        .chars()
                        .take(160)
                        .collect::<String>()
                ),
            );
        }
    }
}

fn events(value: &mut Value, budget: &mut usize, bounded: bool) {
    match value {
        Value::Object(map) => {
            let mut hidden = false;
            for name in ["stdout_tail", "stderr_tail"] {
                hidden |= map.remove(name).is_some();
            }
            if hidden {
                map.insert("diagnostics_available".to_owned(), json!(true));
            }
            for (name, item) in map.iter_mut() {
                events(
                    item,
                    budget,
                    matches!(
                        name.as_str(),
                        "text" | "body" | "result" | "error" | "task_title" | "detail"
                    ),
                );
            }
        }
        Value::Array(items) => {
            for item in items {
                events(item, budget, bounded);
            }
        }
        Value::String(text) if bounded && text.chars().count() > (*budget).min(4000) => {
            let amount = (*budget).min(4000);
            *text = format!(
                "{}\n[truncated; request --full for complete content]",
                text.chars().take(amount).collect::<String>()
            );
            *budget = budget.saturating_sub(amount);
        }
        Value::String(text) if bounded => {
            *budget = budget.saturating_sub(text.chars().count());
        }
        _ => {}
    }
}

pub fn compact(mut value: Value, result_only: bool) -> Value {
    if value["session"].is_object() {
        session(&mut value["session"]);
        value["id"] = value["session"]["id"].clone();
        value["status"] = value["session"]["status"].clone();
        value["progress"] = json!({"last_guest_activity_at":value["session"]["last_activity"],
            "provider_output_at":value["session"]["provider_output_at"], "contact_at":value["session"]["guest_heartbeat"],
            "note":"Contact is not proof of progress; use delegation output ID for bounded diagnostics."});
    }
    if let Some(items) = value.get_mut("sessions").and_then(Value::as_array_mut) {
        for item in items {
            session(item);
        }
    }
    if result_only {
        let result = value["session"]["result"].take();
        value["session"]
            .as_object_mut()
            .map(|map| map.remove("result"));
        value["events"] = json!([]);
        value
            .as_object_mut()
            .map(|map| map.remove("last_checkpoint"));
        value.as_object_mut().map(|map| map.remove("result_event"));
        events(&mut value, &mut 16000, false);
        value["result"] = result;
    } else {
        events(&mut value, &mut 16000, false);
    }
    value
}
