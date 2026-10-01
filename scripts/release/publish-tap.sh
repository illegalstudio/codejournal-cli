#!/usr/bin/env bash
set -euo pipefail
: "${TAP_TOKEN:?An installation token scoped to homebrew-tap is required}"
formula="$(realpath "${1:?Pass the generated formula path}")"
version="${2:?Pass the release version}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || exit 1
work="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/cj-tap.XXXXXX")"
trap 'rm -rf "$work"' EXIT
# Keep credentials out of remote URLs, Git config files and credential helpers.
authorization="$(printf 'x-access-token:%s' "$TAP_TOKEN" | base64 | tr -d '\n')"
export GIT_CONFIG_COUNT=2
export GIT_CONFIG_KEY_0=credential.helper GIT_CONFIG_VALUE_0=''
export GIT_CONFIG_KEY_1=http.https://github.com/.extraheader
export GIT_CONFIG_VALUE_1="AUTHORIZATION: basic $authorization"
export GIT_TERMINAL_PROMPT=0
remote=https://github.com/illegalstudio/homebrew-tap.git
for attempt in 1 2 3; do
  if [[ "$attempt" == 1 ]]; then
    git clone --depth 1 "$remote" "$work/tap"
  else
    git -C "$work/tap" fetch origin main
    git -C "$work/tap" reset --hard origin/main
  fi
  cp "$formula" "$work/tap/Formula/codejournal-cli.rb"
  python3 - "$work/tap/README.md" <<'PY'
import pathlib
import sys
path = pathlib.Path(sys.argv[1])
body = path.read_text()
if '| `codejournal-cli` |' not in body:
    body = body.rstrip() + '\n| `codejournal-cli` | Project memory and work history for developers and coding agents |\n'
    path.write_text(body)
PY
  git -C "$work/tap" add Formula/codejournal-cli.rb README.md
  if git -C "$work/tap" diff --cached --quiet; then exit 0; fi
  git -C "$work/tap" -c user.name='illegal-studio[bot]' \
    -c user.email='327907755+illegal-studio[bot]@users.noreply.github.com' \
    commit -m "Update codejournal-cli to $version"
  if git -C "$work/tap" push origin HEAD:main; then exit 0; fi
done
printf '%s\n' 'The tap changed concurrently; publication failed without force-pushing.' >&2
exit 1
