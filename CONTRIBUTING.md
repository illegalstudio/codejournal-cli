# Contributing to Code Journal CLI

Open issues and pull requests in [illegalstudio/codejournal-cli](https://github.com/illegalstudio/codejournal-cli). Include a concrete reproduction for bugs and explain the user-facing behavior a change should provide.

## Source workflow

The public repository is an automated split of `apps/cli` in the development monorepo. Work on a branch based on public `main` and submit a pull request here. Maintainers port accepted changes into `apps/cli`, validate them with the hosted product when appropriate, and let CI publish the resulting split. Public `main` is maintained by that split; maintainers do not merge changes directly into it.

Keep this folder self-contained: runtime code, the embedded agent skill, docs, assets, and standalone tests must work when it is the repository root. Backend and Python-reference parity tests belong in the monorepo's `tests/cli-parity` harness.

## Checks

Use a current stable Rust toolchain, Python 3, and Git:

```bash
bash scripts/test.sh
```

Keep each handwritten implementation module focused and below 150 logical lines. Add regression coverage for behavior that could break; preserve API compatibility and offline delivery. Keep commit messages concise, without `Co-Authored-By` trailers.

Use synthetic test data. Do not commit credentials, account state, journal data, generated binaries, or agent settings. Report security concerns through the repository's private vulnerability reporting feature rather than a public issue.
