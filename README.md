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

## Install from source

Install a current stable Rust toolchain and Git, then run:

```bash
cargo install --git https://github.com/illegalstudio/codejournal-cli.git --locked
cj --version
```

Cargo installs `cj` into its binary directory, usually `~/.cargo/bin`. Ensure that directory is on your `PATH`.
Binary releases, package-manager installers, and self-update are planned. Source installation is the supported method at this stage.

## Connect your account

Create and verify a Code Journal account on the service you use. Pass its URL explicitly:

```bash
cj --server https://codejournal.online login
cj whoami
cj setup agents --dry-run
cj setup agents
```

Login opens browser approval for the device and saves the selected service and workspace. Tokens use the system keyring where available, with a user-only file fallback. `cj logout` revokes the device token.
The public service is being prepared; for local development, use `https://codejournal-saas.ddev.site` with the hosted product's DDEV environment.

Agent setup currently supports Codex, Claude Code, and Cursor. It installs the embedded [Code Journal skill](skill/SKILL.md), preserves unrelated settings, and backs up settings it changes. Review and trust newly installed Codex hooks with `/hooks`.

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

## Development

Clone this repository and run:

```bash
cargo build --locked
bash scripts/test.sh
```

The test script checks Rust formatting, runs Rust unit tests, and runs the Python black-box suite against local mock APIs and synthetic data. It requires Python 3 and Git; no running SaaS, personal journal, or credentials are needed. CI runs this suite on Linux. A Codex policy test runs only when the Codex CLI is installed.

## Contributions and source ownership

This public repository is generated from `apps/cli` in the Code Journal development monorepo. The monorepo remains the canonical source; its CI publishes a history-preserving subtree split to this repository's `main` branch.

Issues and pull requests are welcome here. Maintainers integrate accepted changes into the monorepo, then let the split publish them here. See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow.

Licensed under [MIT](LICENSE).
