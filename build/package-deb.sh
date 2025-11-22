#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: build/package-deb.sh --distro <debian|ubuntu> [--version <semver>] [--arch <arch>] [--revision <n>] [--skip-build]

Build the chissu-pam Debian/Ubuntu package using dpkg-buildpackage.

Options:
  --distro <name>   Target distribution slug (debian or ubuntu).
  --version <ver>   Override the package version (defaults to workspace version).
  --arch <arch>     Target architecture (default: amd64).
  --revision <n>    Debian revision suffix (default: 1).
  --skip-build      Reuse existing target/release artifacts instead of rebuilding.
  -h, --help        Show this help message.

Environment overrides:
  CHISSU_PAM_MAINTAINER  Set maintainer identity for changelog (default: "Chissu Maintainers <ops@chissu.dev>").
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PKG_NAME="chissu-pam"
PAM_DEST_REL="usr/lib/x86_64-linux-gnu/security"
DISTRO=""
VERSION=""
ARCH="amd64"
REVISION="1"
SKIP_BUILD=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --distro)
      DISTRO="${2,,}"
      shift 2
      ;;
    --version)
      VERSION="$2"
      shift 2
      ;;
    --arch)
      ARCH="$2"
      shift 2
      ;;
    --revision)
      REVISION="$2"
      shift 2
      ;;
    --skip-build)
      SKIP_BUILD=1
      shift 1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
  done

if [[ -z "$DISTRO" ]]; then
  echo "--distro is required" >&2
  usage >&2
  exit 1
fi

if [[ "$DISTRO" != "debian" && "$DISTRO" != "ubuntu" ]]; then
  echo "Unsupported distro: $DISTRO" >&2
  exit 1
fi

if [[ -z "$VERSION" ]]; then
  VERSION=$(awk -F'"' '
    /^\[workspace\.package\]/ { in_block=1; next }
    /^\[/ { in_block=0 }
    in_block && /version/ { print $2; exit }
  ' "$REPO_ROOT/Cargo.toml")
fi

if [[ -z "$VERSION" ]]; then
  echo "Failed to determine version" >&2
  exit 1
fi

MAINTAINER="${CHISSU_PAM_MAINTAINER:-Chissu Maintainers https://github/sett4/chissu-pam}"
DATERFC=$(date -R)
DEB_VERSION="${VERSION}-${REVISION}"
WORK_ROOT="$REPO_ROOT/build/package/work/$DISTRO"
SRC_DIR="$WORK_ROOT/${PKG_NAME}-${VERSION}"
DEBIAN_TEMPLATE_DIR="$REPO_ROOT/build/package/debian"
DEBIAN_DIR="$SRC_DIR/debian"
ARTIFACT_DIR="$SRC_DIR/artifacts"
DIST_DIR="$REPO_ROOT/dist"

log() {
  echo "[package-deb] $*"
}

command -v dpkg-buildpackage >/dev/null || { echo "dpkg-buildpackage not found" >&2; exit 1; }
command -v dh >/dev/null || { echo "debhelper (dh) not found" >&2; exit 1; }

if [[ $SKIP_BUILD -eq 0 ]]; then
  log "Building release artifacts"
  pushd "$REPO_ROOT" >/dev/null
  CARGO_HOME="$REPO_ROOT/.cargo-home" \
    cargo build --release -p chissu-cli -p pam-chissu
  popd >/dev/null
else
  log "Skipping cargo build (per --skip-build)"
fi

rm -rf "$WORK_ROOT"
mkdir -p "$DEBIAN_DIR" "$ARTIFACT_DIR" "$DIST_DIR"

escape_sed() {
  printf '%s' "$1" | sed -e 's/[\\/&]/\\&/g'
}

copy_template() {
  local template="$1"
  local dest="$2"
  local maint_escaped date_escaped debver_escaped version_escaped arch_escaped distro_escaped
  maint_escaped=$(escape_sed "$MAINTAINER")
  date_escaped=$(escape_sed "$DATERFC")
  debver_escaped=$(escape_sed "$DEB_VERSION")
  version_escaped=$(escape_sed "$VERSION")
  arch_escaped=$(escape_sed "$ARCH")
  distro_escaped=$(escape_sed "$DISTRO")
  sed \
    -e "s/__DISTRO__/$distro_escaped/g" \
    -e "s/__VERSION__/$version_escaped/g" \
    -e "s/__DEB_VERSION__/$debver_escaped/g" \
    -e "s/__ARCH__/$arch_escaped/g" \
    -e "s/__DATE__/$date_escaped/g" \
    -e "s/__MAINTAINER__/$maint_escaped/g" \
    "$template" > "$dest"
}

log "Rendering Debian metadata"
copy_template "$DEBIAN_TEMPLATE_DIR/changelog.in" "$DEBIAN_DIR/changelog"
copy_template "$DEBIAN_TEMPLATE_DIR/control.in" "$DEBIAN_DIR/control"
cp "$DEBIAN_TEMPLATE_DIR/rules" "$DEBIAN_DIR/rules"
cp "$DEBIAN_TEMPLATE_DIR/install" "$DEBIAN_DIR/install"
cp "$DEBIAN_TEMPLATE_DIR/dirs" "$DEBIAN_DIR/dirs"
cp "$DEBIAN_TEMPLATE_DIR/postinst" "$DEBIAN_DIR/postinst"
cp "$DEBIAN_TEMPLATE_DIR/prerm" "$DEBIAN_DIR/prerm"
cp "$DEBIAN_TEMPLATE_DIR/postrm" "$DEBIAN_DIR/postrm"
cp "$DEBIAN_TEMPLATE_DIR/copyright" "$DEBIAN_DIR/copyright"
mkdir -p "$DEBIAN_DIR/source"
cp "$DEBIAN_TEMPLATE_DIR/source/format" "$DEBIAN_DIR/source/format"
chmod 755 "$DEBIAN_DIR/rules" "$DEBIAN_DIR/postinst" "$DEBIAN_DIR/prerm" "$DEBIAN_DIR/postrm"

log "Staging artifacts"
BIN_SRC="$REPO_ROOT/target/release/chissu-cli"
PAM_SRC="$REPO_ROOT/target/release/libpam_chissu.so"
if [[ ! -f "$BIN_SRC" || ! -f "$PAM_SRC" ]]; then
  echo "Expected release binaries missing; run without --skip-build" >&2
  exit 1
fi

mkdir -p "$ARTIFACT_DIR/usr/bin" \
         "$ARTIFACT_DIR/$PAM_DEST_REL" \
         "$ARTIFACT_DIR/etc/chissu-pam" \
         "$ARTIFACT_DIR/usr/share/doc/chissu-pam"

cp "$BIN_SRC" "$ARTIFACT_DIR/usr/bin/chissu-cli"
cp "$PAM_SRC" "$ARTIFACT_DIR/$PAM_DEST_REL/libpam_chissu.so"
cp "$REPO_ROOT/build/package/assets/etc/chissu-pam/config.toml" "$ARTIFACT_DIR/etc/chissu-pam/config.toml"
cp "$REPO_ROOT/build/package/assets/usr/share/doc/chissu-pam/README.Debian" "$ARTIFACT_DIR/usr/share/doc/chissu-pam/README.Debian"

pushd "$SRC_DIR" >/dev/null
log "Running dpkg-buildpackage"
MAINT_NAME="${MAINTAINER%% <*}"
MAINT_EMAIL_PART="${MAINTAINER##*<}"
MAINT_EMAIL="${MAINT_EMAIL_PART%>}"
DEBFULLNAME="$MAINT_NAME" DEBEMAIL="$MAINT_EMAIL" dpkg-buildpackage -us -uc
popd >/dev/null

DEB_OUTPUT=$(find "$WORK_ROOT" -maxdepth 1 -type f -name "${PKG_NAME}_*.deb" | head -n1)
if [[ -z "$DEB_OUTPUT" ]]; then
  echo "dpkg-buildpackage did not produce a .deb" >&2
  exit 1
fi

FINAL_NAME="$DIST_DIR/${PKG_NAME}_${VERSION}_${DISTRO}_${ARCH}.deb"
log "Copying $(basename "$DEB_OUTPUT") -> $(basename "$FINAL_NAME")"
cp "$DEB_OUTPUT" "$FINAL_NAME"
log "Done: $FINAL_NAME"
