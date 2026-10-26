---
name: code-journal-delegation
description: Delegate a bounded task to an installed Claude Code, Codex, Cursor Agent, Grok, Kimi Code or Pi CLI through Code Journal. Use when the user requests another agent or provider, including a choice of subscription. Does not apply to ordinary same-runtime subagents.
---

# Code Journal delegation

`cj delegation` contains the broker, supervisor, provider adapters and guest protocol. Only `cj` and the selected agent CLI are needed. No external scripts or configuration repository are required. Provider authentication and billing remain with that agent's installed CLI.

Use the provider the user requested. When the provider is unspecified, clarify only if choosing one changes the requested subscription or workflow. Do not launch nested delegations. Keep the host responsible for scope, review, commits and pushes.

## Start and supervise

Use a durable start for ongoing work. The default mode is read-only:

```sh
cj delegation models codex
cj delegation start codex --cwd "$PWD" --mode read-only <<'TASK'
Inspect the named behavior and return evidence and findings. Do not edit files.
TASK
cj delegation wait ID --until question --timeout 30
cj delegation status ID
```

Record the returned session ID. Examine status and events after each bounded wait, answer questions with their sequence and acknowledge consumed events:

```sh
cj delegation answer ID --reply-to SEQ --body 'The answer or approved next step'
cj delegation send ID --body 'Additional context within the authorized scope'
cj delegation ack ID --through SEQ
cj delegation result ID --timeout 60
```

Normal standard output is a diagnostic log. Completion requires a `result` event; `failed`, `cancelled` and `stalled` require inspecting the reason. Report completion and any material limitation to the user. Do not start duplicate attempts while a supervisor or guest is alive.

Provider output counts as activity; Claude and Grok stream tool events and partial responses during managed runs. Supervisor heartbeats alone do not prove guest progress or reset inactivity. Choose `--stall-timeout` for the task's expected silent intervals; `0` explicitly disables that deadline. A long review may need a larger budget than the default. Inspect `cj delegation output ID --lines 40` for bounded, redacted stdout/stderr. Silence does not identify a provider API wait or its cause. A busy local database preserves the guest while monitoring retries.

IDs accept a full UUID or a unique prefix of at least eight hex characters. Start with `--id-only` when only the identifier is needed. Default responses omit the full task and diagnostic tails; `--full` retrieves complete content and up to 500 events per page, versus 25 normally. Follow `next_after` for remaining pages.

Bare `wait` remembers delivered events separately from acknowledgement, so repeated waits do not spin on an unacknowledged event. `--after SEQ` explicitly replays from a cursor. `--until event|question|result|terminal` selects the wake condition; waits accept up to 86400 seconds. `result` waits through interim events and returns one top-level result or null on timeout. Inspect status and `timed_out` too. `sync` continues to retrieve unacknowledged events. `list` shows machine-wide active or unresolved delegations; explicit `--cwd PATH` scopes it to that checkout. Use `list --all` for terminal records.

## Write delegations

Use `--mode write` only when the user authorized the bounded edits. Start in a clean Git checkout and name the allowed paths, required behavior and narrowly relevant validation. Only one unresolved writer is allowed across the local broker.

The guest must produce a minimal diff, publish a checkpoint with changed paths, status, diff statistics and explanation, then ask for review. Review the actual diff before answering with the specific tests approved for one run. Existing user authorization may supply that approval; avoid unnecessary user confirmation. The guest must ask before expanding scope or validation after failure, and cannot commit or push.

A write result requires a real diff, a guest checkpoint and the host's answer to a subsequent guest question. After completion, review and commit the result yourself. Wait for the guest process to exit, then run `cj delegation integrate ID`. Completion alone retains the write lock until integration. Integration verifies a clean checkout and a host commit descending from the original baseline.

## Recovery and cleanup

```sh
cj delegation sync ID --full
cj delegation recover ID --stall-timeout 3600
cj delegation cancel ID --reason 'Why this delegation is no longer needed'
```

Recovery keeps the same task, identity and history, increments the attempt and fences old guest tokens. An optional stall budget applies atomically to the new attempt; prior failure and budget remain in history. A missing supervisor with a surviving guest becomes stalled; cancel the confirmed delegation before starting a replacement. Never kill unrelated agent processes. A recovered guest reads the checkpoint and answers before continuing.

`purge` deletes local history and logs only after processes stop; unresolved records require explicit `--force`. `gc` applies a retention interval to terminal history and keeps completed write delegations awaiting integration. Cleanup does not remove the hosted journal's historical activity.

`run TARGET` is a synchronous read-only invocation. `--dry-run` inspects provider arguments without launching an agent or creating a delegation. Use `cj delegation guide --guest` for the embedded guest protocol and `cj delegation --help` for the complete interface.

Lifecycle delivery is queued through the authenticated Code Journal outbox, bound to the account captured at creation. Local delegation works without a journal login; unavailable credentials remain explicit, and do not prevent the guest from running. Never claim unbound or queued events are synchronized.
