#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname "$0")" && pwd -P)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd -P)
LIB="$SCRIPT_DIR/lib/install_common.sh"
DEST_ROOT="$REPO_ROOT/build/package/assets"

if [[ ! -f "$LIB" ]]; then
  echo "install_common library missing at $LIB" >&2
  exit 1
fi

# shellcheck disable=SC1090
source "$LIB"

write_assets "$DEST_ROOT"

echo "Rendered assets to $DEST_ROOT"
