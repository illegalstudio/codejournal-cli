use serde_json::{Value, json};

pub fn definitions() -> Vec<Value> {
    let mut tools = Vec::new();
    for (name, description, body) in [
        (
            "delegation_sync",
            "Recover canonical state and unread events.",
            false,
        ),
        (
            "delegation_wait",
            "Wait for the other endpoint's events.",
            false,
        ),
        (
            "delegation_ack",
            "Acknowledge events through a sequence.",
            false,
        ),
        (
            "delegation_status",
            "Read durable state and history.",
            false,
        ),
        ("delegation_ready", "Announce protocol readiness.", true),
        (
            "delegation_status_update",
            "Send progress without completing the task.",
            true,
        ),
        (
            "delegation_message",
            "Send information to the other endpoint.",
            true,
        ),
        (
            "delegation_question",
            "Ask the other endpoint a question.",
            true,
        ),
        ("delegation_answer", "Answer a specific question.", true),
        ("delegation_checkpoint", "Save a recovery checkpoint.", true),
        (
            "delegation_finish",
            "Publish the guest's final result.",
            true,
        ),
        ("delegation_fail", "Publish a terminal failure.", true),
    ] {
        tools.push(json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":{
            "delegation_id":{"type":"string"},"role":{"type":"string","enum":["host","guest"]},
            "full":{"type":"boolean"},"after":{"type":"integer","minimum":0},
            "timeout":{"type":"number","minimum":0,"maximum":300},"through":{"type":"integer","minimum":0},
            "body":{"type":"string","maxLength":100000},"reply_to":{"type":"integer","minimum":1},"idempotency_key":{"type":"string"}},
            "required":if body {vec!["body"]} else if name=="delegation_ack" {vec!["through"]} else {vec![]}}}));
    }
    tools
}
