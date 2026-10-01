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
mise use -g github:illegalstudio/codejournal-cli@latest
cj --version
```

To pin a version, replace `latest` with the release version, for example `0.1.0`.

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

Agent setup supports Codex, Claude Code, Cursor, Grok, Kimi Code, and Pi. It installs the embedded [Code Journal skill](skill/SKILL.md), preserves unrelated settings, and backs up settings it changes. Review and trust newly installed Codex hooks with `/hooks`. Grok, Kimi Code, and Pi receive a managed instruction block that loads the skill; setup preserves the rest of your instructions.

To select an agent or remove the managed integration:

```bash
cj setup agents --agent kimi
cj setup agents --agent kimi --uninstall
```

## Update

Use the same package manager that installed the CLI:

```bash
brew upgrade illegalstudio/tap/codejournal-cli
# Or, for mise:
mise upgrade github:illegalstudio/codejournal-cli
cj setup agents --refresh
```

For a direct installation, `cj update` downloads the latest stable release, verifies its checksum, replaces the binary, and refreshes your installed agent skills. `cj update --check` checks for a release without changing anything. Package-managed binaries are protected from direct replacement.

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

## Offline work

Reads can use cached responses; writes and hook events queue locally when delivery is deferred. Inspect and replay queued work with:

```bash
cj status
cj outbox list
cj sync
```

Use `--offline` to request queued writes and cached reads explicitly. Avoid storing secrets in journal content.

Licensed under [MIT](LICENSE).
