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

## Notifications and offline work

Use `cj notify --kind needs_input --title "..."` when the user may not be watching and their input is required. Use `cj watch start --title "..." -- COMMAND` when asked to report when a long command ends. `cj status` shows pending writes by server; `cj sync` replays only the configured server's writes. Other servers' queues remain intact. Use `cj outbox list` without authentication to inspect request IDs, servers, methods and paths; `--body` includes redacted bodies and `--server URL` filters them. `cj outbox drop REQUEST_ID` permanently discards one pending write. It never removes hook events. Transport errors, 5xx responses, deferred writes and invalid JSON success responses keep the same durable ID for replay, including project-bootstrap retries. Rejected writes explicitly say they were not queued. Offline doc moves require cached source details and destination project details; fetch them online first. Use `--to @global`, not `global`, for tenant-global docs. The session-start hook surfaces unavailable authentication before the first write. The session hook records activity and notifications without prompt text or file contents.

The repository's `AGENTS.md` and the user's instructions take precedence over this skill.

## Shared documentation

Global docs belong to the current tenant and are available from every project with cross-project access. They are agent-facing documentation, not project rules. The full brief lists current and draft global docs separately; compact briefs include only docs with refs matching current Git changes. Open relevant docs with `cj doc show ID`.

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
