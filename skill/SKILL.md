---
name: code-journal
description: Shared project memory and work tracking through the cj command. Read the project brief at session start, search before investigating known behavior, and record durable discoveries and completed work.
---

# Code Journal

Code Journal is shared across agents working on the same Git repository. The `cj` executable connects to the user's hosted journal. It detects the project from the Git remote and can queue writes while offline. Never include secrets or private source content in journal entries.

## Where it applies

Code Journal records belong to physical projects: a Git repository, or a plain folder the user registered with `cj project init`, including its subfolders. Anywhere else, such as a browser chat workspace, a temporary directory, or the home directory, there is no journal: the session hook says so, and `cj` refuses project commands. Do not use Code Journal in that session. Run `cj project init` in a plain folder only when the user asks to register it, and never for temporary or scratch directories; you may ask the user when the folder is clearly a real project.

## Start

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

Use `--global` only for facts that apply beyond this repository. Correct stale knowledge with `cj supersede OLD --by NEW`, `cj obsolete ID`, or `cj entry flag ID --wrong --note "..."`. Answer open questions with `cj answer ID`.

## Log work

At the end of a turn that changed anything durable, record one log with the goal, result, and remaining work:

```sh
cj log add --title "Implemented the device flow" --status done \
  --plan PLAN_ID --body "What changed, what passed, what remains."
```

The client links session commits where possible. Use `--no-auto-commits` when the log must not cite them.

## Track tasks and plans

Start a matching open task before doing it, then close it with a useful note:

```sh
cj task start ID
cj task done ID --note "What changed"
cj task add --title "Follow-up" --body "Why it matters"
cj plan show ID
cj plan update ID --body-file PLAN.md
```

Use `cj task add --to PROJECT` to forward work to another project. Keep longer plans and subsystem documentation in Code Journal and in repository files when the repository requires them.

## Notifications and offline work

Use `cj notify --kind needs_input --title "..."` when the user may not be watching and their input is required. Use `cj watch start --title "..." -- COMMAND` when asked to report when a long command ends. `cj status` shows queued writes; `cj sync` replays them. The session hook records activity and notifications without prompt text or file contents.

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
