#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
cargo fmt --check
cargo test --locked
node --test integrations/opencode/tests/*.test.mjs
CODE_JOURNAL_AUTO_SYNC=off PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_*.py'
