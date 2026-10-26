use serde_json::Value;

pub fn build(session: &Value) -> String {
    let mode = if super::super::text(session, "mode") == "write" {
        "Change only the paths and behavior authorized in the task. Ask the host before expanding scope. Do not commit or push. Produce the smallest reviewable diff first; publish a checkpoint with git status --short, git diff --stat, changed files and a summary. Then publish a question requesting host review and wait for its answer. Run only the specific tests the host approves, once. Report failures before expanding the test scope."
    } else {
        "Do not modify files or create commits. Inspect and report only."
    };
    let protocol = if let Some(id) = session["id"].as_str() {
        format!(
            "This is durable Code Journal delegation {id}, attempt {}. Read the guest protocol with cj delegation guide --guest. Use the delegation MCP tools when available. Otherwise invoke the executable in CJ_DELEGATION_CLIENT with delegation sync {id} --role guest --full, and delegation publish {id} --role guest --kind KIND --body TEXT. Recover canonical state before acting. Communicate progress and questions. Before a result, sync all event pages and acknowledge inspected host messages; apply their corrections or ask for clarification. Publish one result or error before exit. Standard output is diagnostic and never completes a delegation.",
            session["attempt"]
        )
    } else {
        String::new()
    };
    format!(
        "You are an external delegated coding agent. Complete only the bounded task below.\nDo not delegate to another AI agent or launch another agent CLI.\n{mode}\n{protocol}\n\n{}",
        super::super::text(session, "task")
    )
}
