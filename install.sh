#!/bin/sh
set -eu
repo=illegalstudio/codejournal-cli
fail() { printf '%s\n' "$*" >&2; exit 1; }
case "$(uname -s)" in
  Linux) platform=linux ;;
  Darwin) platform=macos ;;
  *) fail 'Use the Windows archive from GitHub Releases on this operating system.' ;;
esac
case "$(uname -m)" in
  x86_64|amd64) architecture=x64 ;;
  arm64|aarch64) architecture=arm64 ;;
  *) fail 'Unsupported processor: release binaries support x64 and arm64.' ;;
esac
version="${CJ_VERSION:-}"
if [ -z "$version" ]; then
  version="$(curl -fsSL --proto '=https' --tlsv1.2 "https://api.github.com/repos/$repo/releases/latest" |
    sed -n 's/.*"tag_name": *"v\([0-9][0-9.]*\)".*/\1/p')"
fi
printf '%s\n' "$version" | LC_ALL=C grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || fail 'Cannot determine a stable release version.'
work="$(mktemp -d "${TMPDIR:-/tmp}/codejournal-install.XXXXXX")"
staged=
cleanup() {
  [ -z "$staged" ] || rm -f "$staged"
  rm -rf "$work"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
asset="codejournal-cli-v$version-$platform-$architecture.tar.gz"
base="https://github.com/$repo/releases/download/v$version"
curl -fsSL --proto '=https' --tlsv1.2 "$base/$asset" -o "$work/$asset"
curl -fsSL --proto '=https' --tlsv1.2 "$base/SHA256SUMS" -o "$work/SHA256SUMS"
expected="$(awk -v name="$asset" '$2 == name {print $1}' "$work/SHA256SUMS")"
printf '%s\n' "$expected" | LC_ALL=C grep -Eq '^[a-f0-9]{64}$' || fail 'Missing or invalid release checksum.'
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$work/$asset" | awk '{print $1}')"
else
  actual="$(shasum -a 256 "$work/$asset" | awk '{print $1}')"
fi
[ "$actual" = "$expected" ] || fail 'Checksum mismatch; installation aborted.'
tar -xzf "$work/$asset" -C "$work" cj
chmod 755 "$work/cj"
[ "$("$work/cj" --version)" = "cj $version" ] || fail 'Downloaded binary version does not match the release.'
destination="${CJ_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$destination"
[ ! -d "$destination/cj" ] || fail 'The destination cj is a directory.'
staged="$(mktemp "$destination/.cj-install.XXXXXX")"
cp "$work/cj" "$staged"
chmod 755 "$staged"
mv -f "$staged" "$destination/cj"
staged=
printf 'Installed cj %s in %s/cj\n' "$version" "$destination"
printf 'Add %s to your PATH if needed.\n' "$destination"
