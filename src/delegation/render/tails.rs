use serde_json::{Value, json};

/// Characters of each process tail kept in full output; `delegation output --raw` reads the rest.
const TAIL_CHARS: usize = 2000;

/// Full output keeps complete history, but raw provider tails stay bounded diagnostics.
pub fn bounded_tails(mut value: Value) -> Value {
    let id = value["id"].as_str().unwrap_or("ID").to_owned();
    tails(&mut value, &id);
    value
}

fn tails(value: &mut Value, id: &str) {
    match value {
        Value::Object(map) => {
            let mut shortened = false;
            for name in ["stdout_tail", "stderr_tail"] {
                let Some(Value::String(text)) = map.get_mut(name) else {
                    continue;
                };
                let length = text.chars().count();
                if length > TAIL_CHARS {
                    let kept = text.chars().skip(length - TAIL_CHARS).collect::<String>();
                    *text = format!(
                        "[{} earlier characters omitted; read diagnostics with cj delegation output {id} --raw]\n{kept}",
                        length - TAIL_CHARS
                    );
                    shortened = true;
                }
            }
            if shortened {
                map.insert("diagnostics_available".to_owned(), json!(true));
            }
            map.values_mut().for_each(|item| tails(item, id));
        }
        Value::Array(items) => items.iter_mut().for_each(|item| tails(item, id)),
        _ => {}
    }
}
