<p align="center">
  <img src="assets/logo.png" alt="Code Journal logo" width="130">
</p>

<h1 align="center">Code Journal CLI</h1>

<p align="center">
  <em>Project memory, right where you work.</em>
</p>

<p align="center">
  <a href="https://github.com/illegalstudio/codejournal-cli/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/illegalstudio/codejournal-cli/ci.yml?branch=main&amp;style=flat-square&amp;label=tests&amp;color=2563EB" alt="Tests"></a>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/CLI-Rust-2563EB?style=flat-square&amp;logo=rust&amp;logoColor=white" alt="Built with Rust"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-2563EB?style=flat-square" alt="MIT license"></a>
</p>

<p align="center">
  <strong>Knowledge and work history &middot; Agent integrations &middot; Offline queue &middot; Git context</strong>
</p>

<p align="center">
  The open-source <code>cj</code> client connects developers and AI coding agents to Code Journal.
  Capture discoveries, maintain plans and tasks, and carry useful context from one session to the next.
  The hosted service owns journal storage and business rules; this client handles terminal commands,
  local Git observations, agent hooks, durable delegations, and offline delivery.
</p>

<p align="center">
  <a href="https://codejournal.online"><strong>codejournal.online</strong></a>
</p>

---

## Install

### Homebrew

On macOS or Linux:

```bash
brew install illegalstudio/tap/codejournal-cli
cj --version
```

### mise

Install the published binary with mise:

```bash
mise use -g github:illegalstudio/codejournal-cli@0.1.0
cj --version
```

Use `@latest` to follow stable releases. mise waits 24 hours before selecting a newly published release with `latest`; an explicit version installs it immediately. See [mise release-age settings](https://mise.jdx.dev/configuration/settings.html#minimum_release_age).

### Direct download

Download an archive for your operating system and processor from [Releases](https://github.com/illegalstudio/codejournal-cli/releases), verify it against the release's `SHA256SUMS`, and put `cj` on your `PATH`.

For a user-local installation on macOS or Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/illegalstudio/codejournal-cli/main/install.sh | sh
```

The installer verifies the published checksum before installing into `~/.local/bin`. Add that directory to your `PATH`. Set `CJ_VERSION=0.1.0` to choose a release or `CJ_INSTALL_DIR` to choose the installation directory.

On Windows, download the ZIP archive or use Scoop:

```powershell
scoop install https://github.com/illegalstudio/codejournal-cli/releases/latest/download/codejournal-cli.json
```

## Connect your account

Create and verify a Code Journal account on the service you use. Pass its URL explicitly:

```bash
cj --server https://codejournal.online login
cj whoami
cj setup agents --dry-run
cj setup agents
```

Login opens browser approval for the device and saves the selected service and workspace. Tokens use the system keyring where available, with a user-only file fallback. `cj logout` revokes the device token.
The public service is being prepared. If you use another Code Journal service, pass its URL explicitly to `cj login`.

Code Journal supports Codex, Claude Code, Cursor, Grok, Kimi Code, Pi, and [OpenCode](#opencode).

`cj setup agents` manages all seven agents. It installs the embedded [Code Journal skill](skill/SKILL.md), preserves unrelated settings, and backs up settings it changes. Review and trust newly installed Codex hooks with `/hooks`. Grok, Kimi Code, and Pi receive a managed instruction block that loads the skill; setup preserves the rest of your instructions. OpenCode receives a native plugin and reuses a discoverable skill when available.

Codex rules automatically approve journal reads and `cj add --body ...` or `cj notify --body ...` with explicit inline content. Commands that read body files or change local notification delivery settings use ordinary Codex command review. Refresh the managed setup to replace an older broad rule.

To select an agent or remove the managed integration:

```bash
cj setup agents --agent kimi
cj setup agents --agent kimi --uninstall
```

Watch workers execute only a command authorized by a local `cj watch start`. The private authorization records the original command, directory, timeout and account scope, and can be consumed once. Changing the remote watch definition prevents execution.

### OpenCode

Install the skill and native plugin, then restart OpenCode:

```bash
cj setup agents --agent opencode --dry-run
cj setup agents --agent opencode
cj setup agents --agent opencode --status
```

Setup uses `~/.config/opencode`, respecting `XDG_CONFIG_HOME` and `OPENCODE_CONFIG_DIR`. It reuses a skill discovered in the standard global OpenCode, Claude-compatible or shared-agent directories. Otherwise it installs `skills/code-journal/SKILL.md` in the OpenCode config directory. Shared skills are never overwritten or removed by the OpenCode target. Conflicting copies stop installation with their paths; identical existing copies are reported without adding another. Project-local skills and custom `skills.paths` remain user-managed.

The plugin loads the brief on startup/resume and after compaction, tracks tool activity and commit attribution, warns before concurrent edits, and records waiting and idle events. It reminds the agent to log changes through tool results and shows an idle reminder when needed. It does not automatically start another model turn. Session deletion records session end; closing the app has no equivalent OpenCode session-end event. Prompts, file contents and tool output are not sent to the journal.

```bash
cj setup agents --agent opencode --refresh
cj hooks status --agent opencode
cj setup agents --agent opencode --uninstall
```

`cj hooks install` and `cj hooks uninstall` also accept `--agent opencode` to manage only the plugin. Refresh updates CJ-owned files; customized files are preserved and reported. Keep `cj` authenticated and on your PATH. The integration was validated with OpenCode 1.18.31. OpenCode's `--pure` mode disables external plugins; `CODE_JOURNAL_HOOKS=off` disables CJ hook activity. Native [skill permissions](https://opencode.ai/docs/skills/) and existing tool permissions still apply.

## Delegate to another agent

The delegation broker, supervisor, provider adapters and MCP server are built into `cj`. Install and authenticate the provider CLI you want to use: Claude Code (`claude`), Codex (`codex`), Cursor (`agent` or `cursor-agent`), Grok (`grok`), Kimi Code (`kimi`) or Pi (`pi`). Delegation uses that provider's account and subscription. It does not require Python or any separate broker installation.

```sh
cj delegation start codex --cwd "$PWD" --mode read-only --prompt 'Review the login flow and report findings'
cj delegation wait ID --timeout 30
cj delegation answer ID --reply-to SEQ --body 'Use the documented behavior'
cj delegation result ID --timeout 30
cj delegation status ID
```

The default is read-only. Durable starts preserve messages and checkpoints across CLI exits. A guest completes only by publishing an explicit result; stdout alone is diagnostic. `recover ID` keeps the session history and fences older attempts; `cancel ID` stops the confirmed owned run. Live guests block duplicate recovery.

Temporary local database contention is retried without rerunning the task; a busy supervisor keeps its guest alive. If a previous version left a session stalled, `recover ID` restarts that session after its recorded processes have exited.

Grok delegations stream tool activity and partial responses to keep long reviews observable. Provider output counts as activity; supervisor heartbeats alone do not. A silent guest still reaches its `--stall-timeout`, and completion still requires an explicit result.

`--mode write` requires a clean Git checkout and retains its writer lock through host review, a host commit and `cj delegation integrate ID`. The guest must publish a diff checkpoint and receive host approval before finishing. It cannot commit or push. Use `cj delegation models TARGET` to inspect model options, and `run TARGET --dry-run --prompt TEXT` to inspect arguments without launching a guest.

`cj setup agents --refresh` installs the embedded `code-journal-delegation` and `code-journal-guest` instructions for configured agents. They are also available through `cj delegation guide` and `guide --guest`. Local delegation works without a journal login; hosted lifecycle events use the account captured at creation and queue when unavailable. Explicit `--offline` keeps delivery local.

To import existing local delegation history, finish any active legacy workers, stop their idle broker and run `cj delegation migrate --from /path/to/delegations.sqlite3`. The importer preserves IDs and history and never changes the original database. It requires an empty native store unless you add `--merge`: this leaves existing native sessions unchanged and translates imported event cursors, replies and acknowledgements together. Conflicting session IDs are rejected. `cj delegation --help` lists messaging, MCP and cleanup commands.

## Update

Use the same package manager that installed the CLI:

```bash
brew update && brew upgrade illegalstudio/tap/codejournal-cli
# Or, for mise:
mise upgrade github:illegalstudio/codejournal-cli
cj setup agents --refresh
cj --version
cj sync
```

For a direct installation, `cj update` downloads the latest stable release, verifies its checksum, replaces the binary, and refreshes your installed agent skills. `cj update --check` checks for a release without changing anything. Package-managed binaries are protected from direct replacement.

The server can require a minimum CLI version. If your version is no longer supported, `cj` shows the installed and required releases with update instructions. Agent session hooks pass that notice to your coding agent too. With mise, select the required release explicitly using `mise use -g github:illegalstudio/codejournal-cli@VERSION` if `latest` has not picked it up yet. After updating, verify `cj --version`, refresh agent instructions with `cj setup agents --refresh`, and run `cj sync`.

While an update is required, cached reads are not fresh and queued writes have not reached the server. Keep the outbox so synchronization can retry safely after the upgrade. Every backend request identifies the installed release, and the service records it with the data written by the CLI.

## Start a project journal

Run these commands in your Git repository:

```bash
cj project init
cj brief
cj add --kind discovery --title "Cache invalidation" --body "Record what future sessions should know."
cj search cache
cj search liveness --literal
cj log add --title "Completed the cache fix" --status done --body "Describe the change and validation."
cj open
```

Use `cj --help` and `cj <command> --help` for the full interface. The [embedded skill](skill/SKILL.md) explains plans, tasks, docs, refs, maintenance, and the agent workflow.

`cj topics similar` and `cj garden` analyze the selected project and relevant global knowledge. Use `cj topics similar --all-projects` for an explicit workspace-wide comparison. They may report that topic analysis is partial for large or highly similar topic collections. Review the returned suggestions without assuming they include every duplicate. You can still merge other known duplicates with `cj topics merge SOURCES --into TARGET`. JSON output identifies incomplete analysis.

### Review journal maintenance

`cj garden` shows five pending findings, with priority and progress counts. Read the target and verify the cited code before recording an outcome. Safe automatic repairs and running a scan do not acknowledge manual reviews.

```bash
cj garden
cj garden review FINDING_ID --outcome verified --note "Checked the cited code."
cj garden --after FINDING_UUID
cj garden --all
cj garden --dry-run
```

Use `cj garden --pending` for the stored queue without a new scan or maintenance. Full scans print page progress on stderr while JSON stays on stdout. Use `--limit N` for 1 to 25 findings. `--after` continues the stored queue without rescanning; `--all` requests the complete pending output. `--dry-run --all` previews everything without writes. A review needs an evidence note and supports `verified`, `corrected`, `superseded`, `obsolete`, `dismissed` or `deferred`; deferral also requires `--until` with a future timestamp. Correct the underlying content with its normal command first. Reviewed evidence stays hidden until the relevant knowledge or files change, and pending counts remain in the brief. Older servers provide a bounded report and explicitly identify unavailable persistent reviews.

Garden only suggests duplicate entries when titles have the same normalized content terms, allowing reordered words and inflections. A shared sentence template with a different package, host, version or other title term is insufficient. Review candidates before superseding any entry.

### Archive a finished project

```bash
cj project archive --project my-project
cj projects --archived
cj projects --all
cj project restore --project my-project
```

Archiving is reversible and keeps the project's history. Archived projects disappear from normal discovery and agent briefs. New writes are rejected until you restore the project, including writes from older clients. You can still read records by ID or explicitly select the project, for example `cj --project my-project log list`. Finish or cancel running watches before archiving.

Archive and restore require an online connection. Once this client has observed an archive, offline briefs also show only the archive notice. Already queued writes remain queued and may need an explicit restore before synchronization. Exports include archived history and archive dates; importing into a new project retains that state after its history is loaded.

### Discover active work

Discovery returns concise summaries in both text and JSON. Defaults are current docs, active plans, open or in-progress tasks, open feedback, active knowledge and watches, and unread notifications. Recent work logs remain visible, including completed work. Project inventories default to active projects; use `cj projects --archived` or `--all` for archives; topic lists count active knowledge.

```bash
cj doc list
cj doc list --status outdated
cj doc list --all
cj task list --status done
cj task list --grep release
cj feedback show FEEDBACK_ID
cj feedback show FEEDBACK_ID --json
cj plan list --all --verbose
cj search "cache" --all-statuses
cj notifications list --read
cj notifications list --all
cj brief --all
cj brief --verbose --json
```

`--all` includes inactive records; `--status` selects one state where supported. `--verbose` independently requests bodies and full metadata. Scope flags such as `--global` and `--all-projects` retain the active defaults. Read a chosen record by ID for its content; plan/doc history still requires `--revision`, `--history` or `--all-revisions`. Legacy `--status open` on docs explicitly selects draft, current and outdated records.

`cj task list --grep TEXT` matches a literal substring of the title or body, ignoring case, while retaining the selected lifecycle and project filters. Checklist steps require a fresh online revision and a valid Markdown checkbox; a failed read does not silently use cached content for a write. Use `--offline` explicitly to work from cached or queued content.

After moving a repository, use `cj checkouts remove /old/absolute/path --host HOST` to forget its exact registration and Live observation. The host defaults to this machine. This does not change files or resolve symlinks; `cj project path-remove` uses the same behavior. `cj refs move path:OLD path:NEW` accepts the same path prefix shown by stored references.

The brief and session hooks keep complete project rules, active coordination and recent work. Default JSON omits record bodies and duplicate project rules. Cached reads follow the same visibility and detail policy; explicit all-record or verbose caches cannot replace default context. Exports and backups retain every record.

`cj feedback show ID` reads one complete report, including its resolution and attribution, without fetching the feedback list. Use a full ID or a unique prefix of at least eight hexadecimal characters. Closed or dismissed reports and reports from archived projects remain readable by ID; no repository checkout is required.

Search results include a short matching excerpt. Use `cj search "liveness" --literal` when you need a case-insensitive phrase without stemming, including identifiers such as `worker::probe` or paths. `%` and `_` remain literal. This mode requires a query and cannot combine with `--any` or `--prefix`.

Use `cj search "repl" --whole-word` to match `REPL` without matching `replacement`, `replies` or `replay`. It matches a literal word or phrase without stemming; adjacent letters, digits and underscores prevent a match. It can combine with `--literal`, but not `--any` or `--prefix`, and requires an updated server. Literal substring matching remains available through `--literal` alone.

Full-text and literal search results rank exact query-term coverage ahead of stemming-only matches, then prefer title coverage. For broad `--any` queries this keeps subsystem-specific results ahead of records matching only a common term. Noisy single-word results suggest `--whole-word` and `--topic`.

Normal full-text searches require every term by default. When a multi-term query finds nothing, text output and JSON `notices` suggest `--any` to match at least one term while preserving the other filters.

Empty knowledge queries also suggest a document search using `cj doc list --grep QUERY`. The suggested command keeps the selected project local, or retains an explicit `--all-projects` scope. It searches current documents without downloading their bodies automatically.

Task descriptions can be edited without replacing the task. Read `cj task show ID` for its `body_version`, then use `cj task edit ID --body-file details.md --base BODY_VERSION` (or `--body TEXT`). A stale body version fails without changing the task or its history. An explicit empty body clears the description.

### Amend a work log

```bash
cj log show LOG_ID
cj log update LOG_ID --base 1 --body-file revised-log.md --status done
```

Use the current `revision` reported by `log show` for `--base`. Body and status edits require it; a stale base fails and preserves the existing record. Status accepts `done`, `in_progress` or `blocked`. An explicit empty body clears it. ID, creation time, title, attribution and plan link are retained; omitted fields, including refs, are retained. `--ref` replaces refs and `--clear-refs` clears them. Existing refs-only commands still work without `--base`; supplying it also protects ref changes.

Amendments require the updated server. Offline amendments queue their original base and content; a conflict remains pending for you to review and is never rebased automatically. Work logs stored in activity archives, and logs belonging to archived projects, remain read-only.

A confirmed content amendment of a log created in the same session satisfies its Stop reminder. Updating an unrelated log or only its refs does not. Commit tracking attributes captured commits only; an unknown HEAD change is reported as a notice and does not prevent recording the work. Use an explicit commit ref when the uncaptured commit belongs to this session.

### Plan and document size

Content reads and mutation confirmations show body size in Unicode characters, the 100,000-character limit and the remaining space. JSON exposes `body_size` with `characters`, `limit`, `remaining` and `warning_at`. At 90,000 characters, current content includes a warning to compact its current state before appending more progress. Existing numbered revisions remain accessible through `show --revision N` or `show --history`.

Cached and pending reads measure the displayed content, including local edits. Historical reads show their size without suggesting that you edit the old snapshot. `show --body` keeps its plain Markdown output.

### Move work history

```bash
cj log move LOG_ID --to destination --dry-run
cj log move LOG_ID --to destination
cj plan move PLAN_ID --to destination --with-logs
```

Moving a log keeps its ID, original date, content, attribution, refs and plan link. `--with-logs` moves a plan and all its linked live logs in one transaction, including logs left in earlier projects. Without it, only the plan moves. All affected projects must be writable; stored activity archives remain read-only. A log preview requires an online connection and never queues; actual moves support the offline outbox.

### Read a repository audit

For a focused inspection, including scripts that inspect several repositories:

```bash
cj brief --audit --json
cj search "queue worker"
cj doc show DOC_ID --current-only
```

The audit contains complete project rules, recent active sessions, entry counts, and project-local task, plan and document metadata. It omits document bodies, search vectors, shared knowledge, global documents and maintenance suggestions. It does not refresh detected project packages or inspect Git references. Its cache is separate from the full brief cache, and cached output is marked offline. Older servers remain compatible because the client also filters their full response; the updated backend avoids reading unnecessary bodies in the first place.

`--compact` shortens the normal text brief further. Normal JSON uses summaries; request `--verbose` for full content or `--audit` for the narrower repository inspection. The normal brief accepts `--limit 0`, `--pinned-limit 0` and `--log-limit 0` to omit recent entries, pinned entries and recent logs respectively; these zero limits require the updated backend. Audit rules are never truncated, and `--audit` cannot be combined with `--compact` or `--max-chars`.

`cj garden` remains the broader maintenance workflow, including shared topic names and relevant global knowledge. A read-only repository audit does not require running maintenance.

### Watch a command

```bash
cj watch start --title "Wait for CI" --timeout 3h -- ./wait-for-ci.sh
```

Timeouts accept integer seconds or the suffixes `s`, `m`, `h` and `d`, from one second to 24 hours. For example, `10800` and `3h` are equivalent. Commands preserve whitespace and accept empty argument values, with a nonempty executable, at most 100 items and at most 2,000 Unicode characters per item. The CLI checks these limits before authentication or starting a process. Put longer inline code in a script file instead of splitting or truncating it silently.

`watch start` succeeds only after the detached runner has authenticated and spawned the command. A queued start exits unsuccessfully and reports `queued=true`, `started=false`, the reserved `watch_id` and the original `request_id` in JSON. Run `cj sync` with the original account and host to deliver the creation and recover its unused local authorization. Repeated synchronization never reruns an already consumed command.

Watches begin as `starting`; an authenticated runner changes them to `running` and renews its lease every 30 seconds. If renewal stops, the server reports `lost` after the 120-second lease expires and its scheduled check runs. This means the command outcome is unknown. A late result can supply the actual outcome. `cj sync` also identifies older watches on this host whose local runner is absent, without restarting their commands.

Use `cj watch list` for active watches, `cj watch list --all` for recent history, and `cj watch cancel ID` with a full UUID or unique prefix of at least eight hexadecimal characters. Cancellation works beyond the recent-history limit and without a local PID file. It signals only a verified matching runner. Cancelling a queued start retains its creation and cancellation requests for synchronization and prevents command execution.

Use `cj watch output ID --lines 40` to read the latest captured stdout and stderr. The default is 40 lines, with a maximum of 200 lines, 16 KB read from a local file and 10,000 output characters. On the original host and credentials it reads a verified local runner's output; otherwise it uses the stored server tail when available. Completed watches retain only their final server tail.

Text and JSON identify the output source, verified runner PID, local file update time and truncation. The lease reports runner contact, not command progress; a quiet command may still be working. `--offline` requires a previously cached lookup for the same ID or prefix and credentials. JSON `cached` describes watch metadata, while `output.source=local` can still contain a current local snapshot. Inspecting output never restarts a command.

If startup fails, the error identifies a private `WATCH_ID.runner.log` file beside the local watch state. Inspect that file for authentication, transport or process errors, then use `cj sync` after correcting the cause. Changing the remote command, directory or timeout invalidates the original execution authorization. Mismatch diagnostics identify fields and argument indexes without printing values.

## Offline work and recovery

Online commands and agent hooks automatically resume pending delivery in the background. Transient failures retry with increasing waits, honoring the server's retry delay, until delivery succeeds. After restarting your computer, the next online command or hook resumes the queue. `cj status` shows write and hook-event counts, queue age, the next retry and any error that needs attention. New writes keep their account/workspace scope and join earlier requests in order.

`--offline` does not start background delivery for that command. Set `CODE_JOURNAL_AUTO_SYNC=off` to prevent automatic workers from starting; `cj sync` remains an immediate manual retry. Previously started workers run independently. Older requests without a verified credential scope require one `cj sync` under their original account. Authentication, validation, required-update or expired-retry errors preserve the queue for you to inspect. Background delivery does not start watched commands; queued watch startup still uses explicit `cj sync` on its original host and account.

Reads can use cached responses; writes and hook events queue locally when delivery is deferred. Use `--offline` to request queued writes and cached reads explicitly. Cached briefs retain their known rules, but listed document bodies may not have been downloaded. A server failure with no cached response is reported separately from an empty result. Hook events retain their originating server, workspace and credential scope. Switching accounts or workspaces leaves those events queued for the original credentials; legacy events without that scope are retained without automatic delivery. Avoid storing secrets in journal content.

Writes can queue offline even when the system keyring cannot be reached. Without access to the credential, credential-scoped cached reads are unavailable. Platform keyring failures are retried twice, after 100 ms and 250 ms; a missing credential is not retried. A keyring access error does not mean the saved token is missing. Sandbox access can differ between invocations, so retry once in the same permitted execution context. Persistent access denial needs a user-managed permission change; repeated retries cannot grant access. Use `--offline` for queued writes while access is unavailable.

A queued `request_id` identifies the synchronization request. Plan and document creation additionally reserves a distinct `resource_id`, usable immediately for `--offline` updates and checklist progress with the updated server. Current reads show projected local content, marked as pending and unsynchronized. Each update remains a separate ordered request; revisions and conflicts are checked during delivery. Entry, task and log creation still require a receipt after synchronization:

```bash
cj status
cj outbox list
cj outbox hooks list
cj sync
cj outbox receipt REQUEST_ID
```

`cj sync` can recover older dependencies through stored request receipts. It keeps requests pending when recovery cannot be verified. `cj status` reports a changed HEAD that commit hooks have not captured; use explicit `--ref commit:SHA` references for recovery.

Hook events saved without configuration or credential access remain unbound. `cj status` explains these separately from another account's events, and `cj sync` preserves them. Inspect their project and date with `cj outbox hooks list` (add `--body` for redacted event bodies). After verifying the original account, use `cj outbox hooks bind EVENT_ID --to-current-account`, then `cj sync`. Binding preserves the event ID and payload and refuses already-bound events. To permanently discard a single hook instead, use `cj outbox hooks drop EVENT_ID`. Listing and dropping hooks require no login; both mutations accept a unique ID prefix of at least eight hexadecimal characters and refuse to race an active delivery.

Checkout activity skips identical confirmed state for 15 minutes and sends real changes immediately. When earlier snapshots are waiting for delivery, later changes queue in order. Failed or uncertain requests retain their original IDs until acknowledged. Ordinary queued writes have a safe retry window of 90 days; older files remain pending so you can check existing records before resubmitting them. The server can release older saved responses to free space while retaining protection against duplicate writes. If a retry reports that its response was released, the request stays queued for verification; use `cj outbox receipt REQUEST_ID` for supported resource IDs before deciding how to reconcile it.

Use `--section "Heading"` to read one Markdown section and its subsections, or `--max-lines N` to bound the body to 1-500 lines. Selection excludes history, defaults to 200 lines and caps content at 16,000 characters. JSON reports source line ranges, full body size and truncation; selected text never claims to be the whole document. These options require the updated server. Plan/doc writes trim outer whitespace and remove duplicate leading title headings; compare the returned canonical body when verifying an update.

`cj plan show ID` and `cj doc show ID` return only the current version by default, including `--json`. Output reports the available revision count and current revision number without downloading old bodies. `--current-only` remains an explicit spelling of this default.

```bash
cj doc show DOC_ID --section "Current state" --max-lines 80
cj doc show DOC_ID                 # Current version and revision count
cj doc show DOC_ID --json          # Same lightweight content as JSON
cj doc show DOC_ID --revision 59   # Only revision 59, without history
cj doc show DOC_ID --history       # An index of 20 revisions, without old bodies
cj doc show DOC_ID --all-revisions # All revision content, explicitly requested
```

The same flags work for plans. `--history` prints a `--before-revision N` command for the next page. `--revision N --body` prints just that version's body. `--all-revisions` includes all saved revision contents within the server's complete-response budget (2,000 combined current/history rows and 8 MiB of body/ref/note data). Larger histories use the paged index and individual revision reads, or the account data download. Counts include the current version, even if an imported journal has gaps in its historical snapshots.

Documents and plans accept bodies of up to 100,000 characters and 30 unique references. Split large documents or group references by subsystem. `cj rules show` prints only the rules on standard output, including an empty value when none are set; use `--json` for structured output. An unknown project remains an error.

Licensed under [MIT](LICENSE).

Imports pack at most 100 records and 512 KiB per client chunk. The service accepts up to 64 MiB, 50,000 records and 1,000 project records per staged batch, with bodies limited to 100,000 characters. A single record must fit a chunk; oversized records fail before upload.

`cj --version` and `cj status` identify the build revision and whether its CLI source was dirty at compilation; the release number continues to control API compatibility and package updates.

Move selected knowledge without recreating it:

```bash
cj entry move ENTRY_ID --to destination-project --dry-run
cj entry move ENTRY_ID --to destination-project --path-prefix apps/tool --replace-prefix ""
```

The move retains the entry ID, creation time, author, topics and usage history, and records original refs and project provenance. It affects one selected entry; repeat it for other explicitly selected entries.
