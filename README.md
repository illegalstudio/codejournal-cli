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
  local Git observations, agent hooks, and offline delivery.
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
cj log add --title "Completed the cache fix" --status done --body "Describe the change and validation."
cj open
```

Use `cj --help` and `cj <command> --help` for the full interface. The [embedded skill](skill/SKILL.md) explains plans, tasks, docs, refs, maintenance, and the agent workflow.

`cj topics similar` and `cj garden` may report that topic analysis is partial for large or highly similar topic collections. Review the returned suggestions without assuming they include every duplicate. You can still merge other known duplicates with `cj topics merge SOURCES --into TARGET`. JSON output preserves the server's `topic_analysis` details.

### Read a repository audit

For a focused inspection, including scripts that inspect several repositories:

```bash
cj brief --audit --json
cj search "queue worker"
cj doc show DOC_ID --current-only
```

The audit contains complete project rules, recent active sessions, entry counts, and project-local task, plan and document metadata. It omits document bodies, search vectors, shared knowledge, global documents and maintenance suggestions. It does not refresh detected project packages or inspect Git references. Its cache is separate from the full brief cache, and cached output is marked offline. Older servers remain compatible because the client also filters their full response; the updated backend avoids reading unnecessary bodies in the first place.

`--compact` shortens the normal text brief and keeps the full JSON contract. Use `--audit` for lean JSON. The normal brief accepts `--limit 0`, `--pinned-limit 0` and `--log-limit 0` to omit recent entries, pinned entries and recent logs respectively; these zero limits require the updated backend. Audit rules are never truncated, and `--audit` cannot be combined with `--compact` or `--max-chars`.

`cj garden` remains the broader maintenance workflow, including shared topic names and relevant global knowledge. A read-only repository audit does not require running maintenance.

### Watch a command

```bash
cj watch start --title "Wait for CI" --timeout 3h -- ./wait-for-ci.sh
```

Timeouts accept integer seconds or the suffixes `s`, `m`, `h` and `d`, from one second to 24 hours. For example, `10800` and `3h` are equivalent. Commands accept at most 100 items including the executable, with at most 2,000 Unicode characters per item. The CLI checks these limits before authentication or starting a process. Put longer inline code in a script file instead of splitting or truncating it silently.

## Offline work and recovery

Reads can use cached responses; writes and hook events queue locally when delivery is deferred. Use `--offline` to request queued writes and cached reads explicitly. Cached briefs retain their known rules, but listed document bodies may not have been downloaded. A server failure with no cached response is reported separately from an empty result. Hook events retain their originating server, workspace and credential scope. Switching accounts or workspaces leaves those events queued for the original credentials; legacy events without that scope are retained without automatic delivery. Avoid storing secrets in journal content.

Writes can queue offline even when the system keyring cannot be reached. Without access to the credential, credential-scoped cached reads are unavailable. Platform keyring failures are retried twice, after 100 ms and 250 ms; a missing credential is not retried. A keyring access error does not mean the saved token is missing. Sandbox access can differ between invocations, so retry once in the same permitted execution context. Persistent access denial needs a user-managed permission change; repeated retries cannot grant access. Use `--offline` for queued writes while access is unavailable.

A queued `request_id` identifies the synchronization request, **not** the created entry, plan, document, task or log. After synchronization, obtain the resource ID before making dependent updates:

```bash
cj status
cj outbox list
cj sync
cj outbox receipt REQUEST_ID
```

`cj sync` can recover older dependencies through stored request receipts. It keeps requests pending when recovery cannot be verified. `cj status` reports a changed HEAD that commit hooks have not captured; use explicit `--ref commit:SHA` references for recovery.

Documents and plans accept bodies of up to 100,000 characters and 30 unique references. Split large documents or group references by subsystem. `cj rules show` prints only the rules on standard output, including an empty value when none are set; use `--json` for structured output. An unknown project remains an error.

Licensed under [MIT](LICENSE).
