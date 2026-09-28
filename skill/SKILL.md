---
name: code-journal
description: Shared project memory and work tracking through the cj command. Read the project brief at session start, search before investigating known behavior, and record durable discoveries and completed work.
---

# Code Journal

Code Journal is shared across agents working on the same Git repository. The `cj` executable connects to the user's hosted journal. It detects the project from the Git remote and can queue writes while offline. Never include secrets or private source content in journal entries.

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
