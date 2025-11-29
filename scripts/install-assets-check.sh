#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname "$0")" && pwd -P)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd -P)
TMP_ROOT=$(mktemp -d)
# shellcheck disable=SC2317  # invoked via trap
cleanup() { rm -rf "$TMP_ROOT"; }
trap cleanup EXIT

# Regenerate into temp and diff to detect drift without mutating the working tree.
DEST="$TMP_ROOT/assets"
mkdir -p "$DEST"
# shellcheck disable=SC1090,SC1091
source "$SCRIPT_DIR/lib/install_common.sh"
write_assets "$DEST"

status=0
for rel in etc/chissu-pam/config.toml usr/share/chissu-pam/install-common.sh; do
  if ! diff -u "$DEST/$rel" "$REPO_ROOT/build/package/assets/$rel"; then
    status=1
  fi
done

if [[ $status -eq 0 ]]; then
  echo "Assets in sync"
fi
exit $status
