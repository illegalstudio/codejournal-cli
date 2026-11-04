---
name: code-journal-guest
description: Participate as the guest in a Code Journal delegation when CJ_DELEGATION_ID and CJ_DELEGATION_CLIENT identify the delegated session. Recover state, exchange progress and questions, and publish one explicit result or failure.
---

# Code Journal guest protocol

Use this protocol only in the delegated session identified by `CJ_DELEGATION_ID`. The installed `cj` executable, also available at `CJ_DELEGATION_CLIENT`, owns the local broker. Do not invoke other scripts or inspect its private database or tokens.

Start by retrieving canonical state and history. With MCP use `delegation_sync` with `full: true`. With the CLI:

```sh
"$CJ_DELEGATION_CLIENT" delegation sync "$CJ_DELEGATION_ID" --role guest --full
"$CJ_DELEGATION_CLIENT" delegation publish "$CJ_DELEGATION_ID" --role guest --kind ready --body 'Protocol loaded'
```

The environment supplies the attempt identity. Never print it, override it, impersonate the host or publish to another delegation. A superseded attempt must stop rather than retry with a different identity. Nested delegation is disabled.

Read the task, mode, checkpoint and host answers before acting. Honor the exact delegated scope. Read-only mode means no edits or commits. Write mode permits only the authorized changes and never commits or pushes.

Read-only providers inherit a filesystem sandbox. Use the private `TMPDIR` only for probes within the authorized scope, and clean them up. The checkout and its shared Git metadata remain read-only. Linux requires bubblewrap; macOS uses sandbox-exec. An unavailable sandbox blocks launch.

After compaction, reuse this delegation's own durable read coverage for the same immutable commit/tree and file content. This preserves independence: do not borrow unverified coverage from peers or earlier review rounds. Record paths, ranges and the immutable SHA in checkpoints; reread ranges when their content changed or coverage evidence is incomplete.

When the task declares immutable context attachments, list them with `cj delegation context ID` and read each through `--name NAME --offset OFFSET --limit 12000` until `next_offset` is null. Verify the complete SHA256. Reads record exact hashes and character ranges; required unread ranges block a result. An attached prior result is available only as a host-captured snapshot, without permission to read another delegation directly.

## Progress and questions

Publish meaningful progress with `delegation_status_update`, information with `delegation_message`, a recoverable checkpoint with `delegation_checkpoint`, and questions with `delegation_question`. CLI equivalents use `delegation publish ID --role guest --kind status|message|checkpoint|question --body TEXT`. Prefer `--body-file` or stdin for long text. Check `host_messages` in responses for pending corrections and read them before dependent work.

When a question is required, wait for the host's answer before dependent work. Use `delegation_wait` or the CLI in bounded intervals:

```sh
"$CJ_DELEGATION_CLIENT" delegation wait "$CJ_DELEGATION_ID" --role guest --after SEQ --timeout 30
"$CJ_DELEGATION_CLIENT" delegation ack "$CJ_DELEGATION_ID" --role guest --through SEQ
```

Check cancellation, failure and supersession on each response. Acknowledge only events already inspected. Large histories expose `next_after`; continue with `--after NEXT_AFTER` (MCP `after`) until all required context is read. Default pages contain 25 events; `--full` reads complete content and up to 500 per page. A wait's delivered cursor does not acknowledge events.

## Writing and validation

Produce the smallest reviewable diff before testing. Publish a checkpoint containing `git status --short`, `git diff --stat`, changed paths and the reason for each change. Ask the host to review it and wait for an answer after that checkpoint. Run only the exact approved tests, once. Report failures and ask before expanding scope or testing. Do not change HEAD, commit or push. Keep the host responsible for final review and integration.

## Finish explicitly

Before finishing, sync all pages of unread host messages and answers, apply their corrections or ask for clarification, and acknowledge the inspected sequence with `delegation_ack` or CLI `ack --through SEQ`. The runtime rejects a result while host messages remain unacknowledged. New messages may arrive before publication; if rejected, read them and reconsider the result.

Publish one authoritative result using `delegation_finish`, or a terminal failure using `delegation_fail`. CLI equivalents:

```sh
"$CJ_DELEGATION_CLIENT" delegation publish "$CJ_DELEGATION_ID" --role guest --kind result --body 'Outcome, evidence, validation and remaining limitations'
"$CJ_DELEGATION_CLIENT" delegation publish "$CJ_DELEGATION_ID" --role guest --kind error --body 'Failure and what the host needs to know'
```

Use only the appropriate terminal event. A final chat message, stdout or successful process exit does not complete a delegation. If terminal publication fails, report the error instead of claiming success. Exit after a confirmed result, failure or cancellation.
