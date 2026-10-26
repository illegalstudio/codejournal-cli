#!/usr/bin/env bash
set -euo pipefail
if [[ -z "${CLI_MACOS_P12:-}" ]]; then
  printf '%s\n' 'Apple signing credentials are not configured; GitHub provenance is provided separately.'
  exit 0
fi
: "${CLI_MACOS_P12_PASSWORD:?Set the signing certificate password}"
: "${CLI_MACOS_SIGNING_IDENTITY:?Set the Developer ID signing identity}"
work="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/cj-sign.XXXXXX")"
keychain="$work/signing.keychain-db"
cleanup() {
  security delete-keychain "$keychain" >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT
password="$(openssl rand -hex 32)"
python3 - "$work/certificate.p12" <<'PYTHON'
import base64
import os
import pathlib
import sys
pathlib.Path(sys.argv[1]).write_bytes(base64.b64decode(os.environ['CLI_MACOS_P12'], validate=True))
PYTHON
security create-keychain -p "$password" "$keychain"
security unlock-keychain -p "$password" "$keychain"
security import "$work/certificate.p12" -k "$keychain" -P "$CLI_MACOS_P12_PASSWORD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$password" "$keychain" >/dev/null
codesign --keychain "$keychain" --sign "$CLI_MACOS_SIGNING_IDENTITY" --options runtime --timestamp "${1:?Binary path required}"
codesign --verify --strict "${1}"
