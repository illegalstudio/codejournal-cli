---
name: code-journal
description: Shared project memory and work tracking through the cj command. Read the project brief at session start, search before investigating known behavior, and record durable discoveries and completed work.
---

# Code Journal

Code Journal is shared across agents working on the same Git repository. The `cj` executable connects to the user's hosted journal. It detects the project from the Git remote and can queue writes while offline. Never include secrets or private source content in journal entries.

## Where it applies

Code Journal records belong to physical projects: a Git repository, or a plain folder the user registered with `cj project init`, including its subfolders. Anywhere else, such as a browser chat workspace, a temporary directory, or the home directory, there is no journal: the session hook says so, and `cj` refuses project commands. Do not use Code Journal in that session. Run `cj project init` in a plain folder only when the user asks to register it, and never for temporary or scratch directories; you may ask the user when the folder is clearly a real project.

## Start

The CLI repository and installation guide are at https://github.com/illegalstudio/codejournal-cli. Check the installed release with `cj --version`. Every API request identifies that release; the server can require a newer version.

Install with `brew install illegalstudio/tap/codejournal-cli` or `mise use -g github:illegalstudio/codejournal-cli@latest`. Direct downloads and checksum instructions are at https://github.com/illegalstudio/codejournal-cli/releases.

If a command or session hook reports `client_upgrade_required`, `client_version_required`, or an unsupported client, tell the user the installed and minimum versions and that an update is required. Do not hide the notice, repeatedly retry the rejected request, spoof the version header, or present cached data or queued writes as synchronized. Preserve the local outbox.

Use the installation's own updater: Homebrew uses `brew update && brew upgrade illegalstudio/tap/codejournal-cli`; mise uses `mise use -g github:illegalstudio/codejournal-cli@VERSION` with the required release, or `@latest`; direct installations use `cj update`. An explicit mise version avoids its default release-age delay. After updating, run `cj --version`, `cj setup agents --refresh`, and `cj sync`. Follow the user's installation permissions; do not run an installer automatically merely because the server requested an upgrade.

Run `cj brief` before exploring a repository unless the session hook already injected the brief. Follow its project rules, active plans, and open tasks for the whole session. If the rules are empty, summarize the repository's instruction files with `cj rules set` and tell the user.

Search before rediscovering a behavior or debugging a known failure:

```sh
cj search "keyword"
cj search "keyword" --all-projects
cj show ENTRY_ID
cj topics
```

`cj browse` is interactive and should be suggested to the user, not run by an agent. `cj open` opens the hosted dashboard for this project.

## Archived projects

A brief with `archived: true`, an `Archived: read-only` notice, or a `project_archived` error means the project is intentionally inactive. Do not initialize it again, write missing rules, run maintenance or record work there. Do not restore it automatically to satisfy a journal instruction. Tell the user and restore only when they explicitly ask, using `cj project restore --project SLUG` while online. Explicit history reads remain available; default project lists, discovery and briefs hide archived context.

Use `cj project archive --project SLUG` when the user asks to archive a completed project, `cj projects --archived` to find archives, and `cj projects --all` to inspect every project. Finish or cancel its active watches first. Preserve queued writes after an archive rejection; the user can restore the project before retrying synchronization.

## Record knowledge

Add one verifiable, non-obvious fact per entry. Write entries in English, reuse existing topics, and reference the affected path or commit:

```sh
cj add --kind gotcha --title "The migration needs a special role" \
  --topics database --ref path:database/migrations/example.php \
  --body "What failed, why, and how to run it correctly."
```

Repository refs use `path:relative/file`. For files outside the checkout use `file:/absolute/path` or `file:~/path` for this machine, and `host:HOST:/absolute/path` for a remote machine. These become `url:file://HOST/path` refs, retaining the host and encoded path without triggering repository staleness checks or reading remote files. Verify them manually on the named host. Do not cite credentials or personal files.

Use `--global` only for facts that apply beyond this repository. Correct stale knowledge with `cj supersede OLD --by NEW`, `cj obsolete ID`, or `cj entry flag ID --wrong --note "..."`. Answer open questions with `cj answer ID`.

## Log work

At the end of a turn that changed anything durable, record one log with the goal, result, and remaining work:

```sh
cj log add --title "Implemented the device flow" --status done \
  --plan PLAN_ID --body "What changed, what passed, what remains."
```

The client links session commits where possible. Use `--no-auto-commits` when the log must not cite them.

Read a previous work log with `cj log show LOG_ID`, or add `--body` for its body alone. `cj show` reads knowledge entries. For automatic commit linking, run the commit and the log in separate tool calls with enabled, trusted commit-tracking hooks so the post-tool hook can record the commit first. In one shell invocation, supply `--ref commit:HEAD` (resolved immediately to a full SHA) or an explicit SHA. If HEAD changed without being captured, retry with an explicit commit ref or `--no-auto-commits` to omit commits. Repeating the log command alone cannot recover missing hook data. A queued log reserves its commit refs too, avoiding duplicate automatic links. Correct an existing log with `cj log update LOG_ID --ref commit:SHA`; this replaces all refs, so include paths or URLs you want to retain. `--clear-refs` removes them. Archived logs are read-only.

## Track tasks and plans

Start a matching open task before doing it, then close it with a useful note:

```sh
cj task start ID
cj task done ID --note "What changed"
cj task add --title "Follow-up" --body "Why it matters"
cj plan show ID
cj plan show ID --json
cj plan update ID --body-file PLAN.md
cj plan step ID 2 --done --note "Validated"
cj plan step ID 2 --undone
```

Use `cj task add --to PROJECT` to forward work to another project. Keep longer plans and subsystem documentation in Code Journal and in repository files when the repository requires them.

Checklist items are numbered from one in Markdown order, excluding fenced examples. Step updates send an atomic change with the current base revision; a concurrent edit returns a conflict instead of replacing the body. Read the plan again and retry after reviewing it.

Plan statuses are `draft`, `active`, `done`, and `abandoned`; doc statuses are `draft`, `current`, and `outdated`. Help lists accepted statuses and feedback categories, and invalid values fail before an API request. Plan/doc show returns current content and lightweight revision counts by default, including JSON. Use `--revision N` for one specific version, `--history` for a paged index without old bodies, or `--all-revisions` to explicitly fetch all content within the server response budget. `--current-only` remains compatible. Updating a plan or doc with `--ref` replaces its entire ref list; omit the flag to retain existing refs.

## Active discovery and detail

Default discovery in text and JSON returns current docs, active plans, open/in-progress tasks, open feedback, active knowledge and watches, and unread notifications. Recent work logs remain useful context, including completed work. Draft or terminal records require an explicit lifecycle flag. Use `cj doc list --status outdated`, `cj plan list --status draft`, `cj task list --status done`, or list `--all` for every status. Knowledge search accepts `--all-statuses` (alias `--all`), and notifications accept `--read` or `--all`.

Lists and normal briefs return summaries without record bodies. Request `--verbose` for content and full metadata, or deliberately read a chosen ID. Visibility and detail are independent: `--all` still uses summaries. `--global`, `--local` and `--all-projects` change scope without widening lifecycle visibility. `cj brief --all` opts into inactive records; `--verbose` supplies full JSON content. Hooks and cached briefs preserve complete rules, active coordination and recent work while following the same defaults. Exports and backups preserve all data.

## Incremental maintenance

When the brief says maintenance is due or reviews are pending, run `cj garden` and work through its small batch. The default is five pending findings, prioritized by severity. A scan and safe automatic repairs do not count as reviewing the content. Read the target entry, doc or plan and check the cited implementation before deciding.

Fix inaccurate content with its normal update, supersede or obsolete command, then record the finding's outcome with evidence:

```sh
cj garden review FINDING_ID --outcome verified --note "Checked the cited code and regression."
cj garden review FINDING_ID --outcome corrected --note "Updated doc DOC_ID to match the implementation."
cj garden review FINDING_ID --outcome deferred --until 2026-10-14T00:00:00Z --note "Awaiting the migration."
```

Use `verified`, `corrected`, `superseded`, `obsolete`, `dismissed` or `deferred` to describe the actual result. Notes are required; deferral needs a future timestamp. A review records a decision without changing its underlying content. Never acknowledge unverified findings or mass-dismiss a long queue. Leave unrelated items pending when the current task does not justify their review.

Use `--after FINDING_UUID` to continue the stored queue without another scan, `--limit N` for 1 to 25 items, and `--all` only when the complete pending report is needed. `--dry-run` writes nothing; `--dry-run --all` explicitly previews every finding. Preview IDs become reviewable after a matching online scan. Offline previews are cached and may be stale; record outcomes online. Older servers explicitly report that persistent reviews are unavailable.

Reviewed evidence stays suppressed across sessions, unrelated commits and committing already reviewed changes. A relevant content or cited-file change creates a new finding. The brief keeps pending counts visible, and scan/review timestamps remain separate. Preserve existing pending requests and let `cj sync` retry review writes safely.

## Notifications and offline work

Online commands and hooks automatically resume credential-scoped writes and hook events in a detached worker. Transient failures back off and honor server retry delays; the next online command/hook restarts recovery after a machine restart. `cj status` shows separate write/event counts, age, last success, next retry and blockers. New writes join older pending requests in order. Explicit `--offline` does not wake a worker; `CODE_JOURNAL_AUTO_SYNC=off` prevents automatic startup. Existing workers continue independently. Unbound older requests require explicit `cj sync` under their original account. Permanent rejections, required upgrades and expired retry windows stay pending and need attention. Background delivery never runs watched commands; their explicit recovery protocol below remains required.

Use `cj notify --kind needs_input --title "..."` when the user may not be watching and their input is required. Use `cj watch start --title "..." -- COMMAND` when asked to report when a long command ends. `cj status` shows pending writes by server; `cj sync` replays only the configured server's writes. Other servers' queues remain intact. Use `cj outbox list` without authentication to inspect request IDs, servers, methods and paths; `--body` includes redacted bodies and `--server URL` filters them. `cj outbox drop REQUEST_ID` permanently discards one pending write. It never removes hook events. Transport errors, 5xx responses, deferred writes and invalid JSON success responses keep the same durable ID for replay, including project-bootstrap retries. Rejected writes explicitly say they were not queued. Offline doc moves require cached source details and destination project details; fetch them online first. Use `--to @global`, not `global`, for tenant-global docs. The session-start hook surfaces unavailable authentication before the first write. The session hook records activity and notifications without prompt text or file contents.

New queued plans and docs return a reserved resource UUID, distinct from the request ID. Use that resource UUID for dependent commands. `cj plan show ID --offline` projects ordered pending edits over the queued creation or cached current revision, with `local_pending=true` and `synced=false`; queued request bodies and IDs remain unchanged. `cj plan step` uses that projected revision. A remote revision conflict stays pending for explicit review. Historical revision reads require confirmed server data.

The repository's `AGENTS.md` and the user's instructions take precedence over this skill.

Watch startup succeeds only after the local command actually starts. A queued start exits nonzero with `queued=true`, `started=false`, `watch_id` and `request_id`; recover it with `cj sync` under the original account on the original host. Synchronization consumes unused local authorization once and never reruns an already started command. `starting` means awaiting the runner, `running` means it renews a lease, and `lost` means contact expired and the outcome is unknown. Startup failures identify a private runner log. Cancel by full UUID or unique prefix with `cj watch cancel ID`; cancellation uses direct retrieval and works without local PID metadata or a complete history download.

## Shared documentation

Global docs belong to the current tenant and are available from every project with cross-project access. They are agent-facing documentation, not project rules. The full brief lists current global docs separately; compact briefs include only docs with refs matching current Git changes. Open relevant docs with `cj doc show ID`.

```sh
cj doc create --global --title "Shared runbook" --body-file RUNBOOK.md --ref path:src
cj doc list --global
cj doc list --grep "runbook"
cj doc list --local --grep "runbook"
cj doc list --all-projects --grep "runbook"
cj doc move ID --to @global
cj doc move ID --to PROJECT_SLUG
```

`--global` creation conflicts with explicit `--project`. Default doc creation stays local to the project. Default doc listing and search include the current project's docs and the tenant's global docs. Use `--local` to exclude shared docs, `--global` for shared docs only, or `--all-projects` for every project plus global docs. Shared docs are labelled `GLOBAL`. Moves retain the ID and history; plans cannot move to global scope. Offline writes and cached reads follow the normal journal queue behavior.

Use `cj entry move ID --to SLUG --dry-run` to preview a selected knowledge transfer, then omit `--dry-run` to apply it. `--path-prefix OLD --replace-prefix NEW` rebases repository paths, with an empty NEW removing the prefix. The entry keeps its ID, date, author, topics and usage history; a source-project provenance log retains the old refs. Both projects must be writable. Never recreate entries just to move them.

Topic similarity defaults to the current project and relevant visible global knowledge. `cj topics similar --all-projects` requests workspace-wide analysis explicitly. Garden JSON separates `topic_analysis_partial` from `review_queue_complete`; a finished review queue does not mean exhaustive topic analysis. Plain prose words do not identify project dependencies; cite a project slug, qualified repository/package, or exact backticked identifier.

Automatic commit linking requires the current checkout and ancestry of its HEAD, in addition to the session's captured commit. Use explicit commit refs for work done in another checkout. `cj --version` and `cj status` show the build revision and dirty marker while the release remains the API compatibility version.
