#!/usr/bin/env bash
set -euo pipefail

if [[ "${GITHUB_ACTIONS:-}" != true || "$(uname -s)" != Linux ]]; then
  echo "This setup is only for disposable GitHub Actions Linux runners." >&2
  exit 1
fi

sudo apt-get -o Acquire::Retries=3 update
sudo apt-get -o Acquire::Retries=3 install -y bubblewrap apparmor

restriction=/proc/sys/kernel/apparmor_restrict_unprivileged_userns
if [[ -f "$restriction" && "$(<"$restriction")" == 1 ]]; then
  sudo tee /etc/apparmor.d/bwrap >/dev/null <<'PROFILE'
abi <abi/4.0>,
include <tunables/global>

profile bwrap /usr/bin/bwrap flags=(unconfined) {
  userns,
}
PROFILE
  sudo apparmor_parser -r /etc/apparmor.d/bwrap
fi

/usr/bin/bwrap --unshare-user --unshare-pid --ro-bind / / \
  --dev /dev --proc /proc -- /bin/true
