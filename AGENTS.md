# Code Journal CLI

This directory is a standalone Rust project and the source of the public `illegalstudio/codejournal-cli` repository. In the development monorepo, edit it here and let CI publish the subtree split. Public `main` is generated; maintainers integrate public contributions through the monorepo.

Read [README.md](README.md) and [CONTRIBUTING.md](CONTRIBUTING.md). Keep files, assets, the embedded skill, and ordinary tests independent of parent directories. Backend and Python-reference parity tests live outside this directory in the monorepo.

Keep handwritten implementation modules under 150 logical lines with one responsibility. Preserve hosted API compatibility, tenant boundaries, secret redaction, and offline behavior. Use synthetic fixtures, and never commit credentials or journal data.

Run `bash scripts/test.sh` before committing CLI changes. It includes `cargo fmt --check`, `cargo test --locked`, and the Python black-box suite. When changing integration behavior in the monorepo, also run its relevant server and parity checks.

Use concise commit messages without `Co-Authored-By`. Do not use em dashes in generated content. Follow the available Code Journal protocol for project memory and work logs.
