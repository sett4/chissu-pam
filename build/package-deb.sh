#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: build/package-deb.sh --distro <debian|ubuntu> [--suite <suite>] [--artifact-label <label>] [--version <semver>] [--arch <arch>] [--revision <n>] [--skip-build]

Build the chissu-pam Debian/Ubuntu package using dpkg-buildpackage.

Options:
  --distro <name>   Target distribution slug (debian or ubuntu).
  --suite <name>    Target Debian/Ubuntu suite or codename for changelog metadata.
  --artifact-label <label>
                    Release label embedded in the output filename (for example ubuntu-24.04).
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
LIB_PATH="$REPO_ROOT/scripts/lib/install_common.sh"
PKG_NAME="chissu-pam"
PAM_DEST_REL="usr/lib/x86_64-linux-gnu/security"
DISTRO=""
SUITE=""
ARTIFACT_LABEL=""
VERSION=""
ARCH="amd64"
REVISION="1"
SKIP_BUILD=0
if [[ ! -f "$LIB_PATH" ]]; then
  echo "Missing shared installer library at $LIB_PATH" >&2
  exit 1
fi

# shellcheck disable=SC1090
source "$LIB_PATH"

BUILD_DEPS=("${DEBIAN_BUILD_PREREQS[@]}")

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
    --suite)
      SUITE="$2"
      shift 2
      ;;
    --artifact-label)
      ARTIFACT_LABEL="$2"
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

detect_host_release_label() {
  local expected_distro="$1"
  local os_id="" version_id=""

  if [[ ! -r /etc/os-release ]]; then
    return 1
  fi

  # shellcheck disable=SC1091
  source /etc/os-release
  os_id="${ID:-}"
  version_id="${VERSION_ID:-}"

  if [[ "$os_id" != "$expected_distro" || -z "$version_id" ]]; then
    return 1
  fi

  printf '%s-%s\n' "$expected_distro" "$version_id"
}

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

if [[ -z "$SUITE" ]]; then
  SUITE="$DISTRO"
fi

if [[ -z "$ARTIFACT_LABEL" ]]; then
  if ! ARTIFACT_LABEL="$(detect_host_release_label "$DISTRO")"; then
    if [[ "$SUITE" == "$DISTRO" ]]; then
      ARTIFACT_LABEL="$DISTRO"
    else
      ARTIFACT_LABEL="${DISTRO}-${SUITE}"
    fi
  fi
fi

MAINTAINER="${CHISSU_PAM_MAINTAINER:-Chissu Maintainers <ops@chissu.dev>}"
DATERFC=$(date -R)
WORK_ROOT="$REPO_ROOT/build/package/work/$ARTIFACT_LABEL"
SRC_DIR="$WORK_ROOT/${PKG_NAME}-${VERSION}"
DEBIAN_TEMPLATE_DIR="$REPO_ROOT/build/package/debian"
DEBIAN_DIR="$SRC_DIR/debian"
ARTIFACT_DIR="$SRC_DIR/artifacts"
DIST_DIR="$REPO_ROOT/dist"
SOURCE_FORMAT_FILE="$DEBIAN_TEMPLATE_DIR/source/format"

log() {
  echo "[package-deb] $*"
}

normalize_deb_upstream_version() {
  if [[ "$VERSION" == *-* ]]; then
    printf '%s~%s\n' "${VERSION%%-*}" "${VERSION#*-}"
    return 0
  fi

  printf '%s\n' "$VERSION"
}

resolve_deb_version() {
  local source_format=""
  local upstream_version=""

  upstream_version="$(normalize_deb_upstream_version)"

  if [[ -r "$SOURCE_FORMAT_FILE" ]]; then
    source_format="$(<"$SOURCE_FORMAT_FILE")"
  fi

  if [[ "$source_format" == *"(native)"* ]]; then
    printf '%s\n' "$upstream_version"
    return 0
  fi

  printf '%s-%s\n' "$upstream_version" "$REVISION"
}

command -v dpkg-buildpackage >/dev/null || { echo "dpkg-buildpackage not found" >&2; exit 1; }
command -v dh >/dev/null || { echo "debhelper (dh) not found" >&2; exit 1; }

require_build_deps() {
  missing=()
  for pkg in "${BUILD_DEPS[@]}"; do
    if ! dpkg -s "$pkg" >/dev/null 2>&1; then
      missing+=("$pkg")
    fi
  done
  if [[ ${#missing[@]} -gt 0 ]]; then
    log "Missing build prerequisites: ${missing[*]}"
    log "Install them with: apt-get install -y ${missing[*]}"
    exit 1
  fi
}

if [[ $SKIP_BUILD -eq 0 ]]; then
  require_build_deps
  log "Building release artifacts"
  pushd "$REPO_ROOT" >/dev/null
  CARGO_HOME="$REPO_ROOT/.cargo-home" \
    cargo build --release -p chissu-cli -p pam-chissu
  popd >/dev/null
else
  log "Skipping cargo build (per --skip-build)"
fi

"$REPO_ROOT/scripts/render-install-assets.sh"

DEB_VERSION="$(resolve_deb_version)"

rm -rf "$WORK_ROOT"
mkdir -p "$DEBIAN_DIR" "$ARTIFACT_DIR" "$DIST_DIR"

escape_sed() {
  printf '%s' "$1" | sed -e 's/[\\/&]/\\&/g'
}

copy_template() {
  local template="$1"
  local dest="$2"
  local maint_escaped date_escaped debver_escaped version_escaped arch_escaped suite_escaped
  maint_escaped=$(escape_sed "$MAINTAINER")
  date_escaped=$(escape_sed "$DATERFC")
  debver_escaped=$(escape_sed "$DEB_VERSION")
  version_escaped=$(escape_sed "$VERSION")
  arch_escaped=$(escape_sed "$ARCH")
  suite_escaped=$(escape_sed "$SUITE")
  sed \
    -e "s/__DISTRO__/$suite_escaped/g" \
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
         "$ARTIFACT_DIR/usr/share/doc/chissu-pam" \
         "$ARTIFACT_DIR/usr/share/pam-configs" \
         "$ARTIFACT_DIR/usr/share/chissu-pam"

cp "$BIN_SRC" "$ARTIFACT_DIR/usr/bin/chissu-cli"
cp "$PAM_SRC" "$ARTIFACT_DIR/$PAM_DEST_REL/libpam_chissu.so"
cp "$REPO_ROOT/build/package/assets/etc/chissu-pam/config.toml" "$ARTIFACT_DIR/etc/chissu-pam/config.toml"
cp "$REPO_ROOT/build/package/assets/usr/share/chissu-pam/install-common.sh" "$ARTIFACT_DIR/usr/share/chissu-pam/install-common.sh"
cp "$REPO_ROOT/build/package/assets/usr/share/doc/chissu-pam/README.Debian" "$ARTIFACT_DIR/usr/share/doc/chissu-pam/README.Debian"
cp "$REPO_ROOT/build/package/assets/usr/share/pam-configs/chissu" "$ARTIFACT_DIR/usr/share/pam-configs/chissu"

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

FINAL_NAME="$DIST_DIR/${PKG_NAME}_${VERSION}_${ARTIFACT_LABEL}_${ARCH}.deb"
log "Copying $(basename "$DEB_OUTPUT") -> $(basename "$FINAL_NAME")"
cp "$DEB_OUTPUT" "$FINAL_NAME"
log "Done: $FINAL_NAME"
