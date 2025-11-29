#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: build/package-rpm.sh --distro <fedora|rhel> [--version <semver>] [--release <n>] [--arch <arch>] [--skip-build]

Build the chissu-pam RPM package via rpmbuild.

Options:
  --distro <name>   Target RPM-based distribution label (fedora, rhel, etc.).
  --version <ver>   Override the package version (defaults to workspace version).
  --release <rel>   RPM release number (default: 1).
  --arch <arch>     Target architecture (default: x86_64).
  --skip-build      Reuse existing target/release artifacts.
  -h, --help        Show this help message.
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}" )" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DISTRO=""
VERSION=""
RELEASE="1"
ARCH="x86_64"
SKIP_BUILD=0
BUILD_DEPS=(dbus-devel clang-libs gcc-c++ rust cargo rpm-build dlib-devel pam-devel)

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
    --release)
      RELEASE="$2"
      shift 2
      ;;
    --arch)
      ARCH="$2"
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

command -v rpmbuild >/dev/null || { echo "rpmbuild not found" >&2; exit 1; }

log() {
  echo "[package-rpm] $*"
}

require_build_deps() {
  missing=()
  for pkg in "${BUILD_DEPS[@]}"; do
    if ! rpm -q "$pkg" >/dev/null 2>&1; then
      missing+=("$pkg")
    fi
  done
  if [[ ${#missing[@]} -gt 0 ]]; then
    log "Missing build prerequisites: ${missing[*]}"
    if command -v dnf >/dev/null 2>&1; then
      log "Install them with: dnf install -y ${missing[*]}"
    elif command -v yum >/dev/null 2>&1; then
      log "Install them with: yum install -y ${missing[*]}"
    else
      log "Install the missing packages using your package manager."
    fi
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

BIN_SRC="$REPO_ROOT/target/release/chissu-cli"
PAM_SRC="$REPO_ROOT/target/release/libpam_chissu.so"
if [[ ! -f "$BIN_SRC" || ! -f "$PAM_SRC" ]]; then
  echo "Release binaries missing; run without --skip-build" >&2
  exit 1
fi

WORK_ROOT="$REPO_ROOT/build/package/rpm/work/$DISTRO"
STAGING_ROOT="$WORK_ROOT/chissu-pam-$VERSION"
ARTIFACT_DIR="$STAGING_ROOT/artifacts"
RPMS_DIR="$WORK_ROOT/rpmbuild"
SPEC_TEMPLATE="$REPO_ROOT/build/package/rpm/chissu-pam.spec.in"
SPEC_PATH="$RPMS_DIR/SPECS/chissu-pam.spec"
DIST_DIR="$REPO_ROOT/dist"

rm -rf "$WORK_ROOT"
mkdir -p "$STAGING_ROOT" "$ARTIFACT_DIR"
mkdir -p "$RPMS_DIR"/{BUILD,BUILDROOT,RPMS,SRPMS,SPECS,SOURCES}
mkdir -p "$DIST_DIR"

log "Staging artifacts"
mkdir -p "$ARTIFACT_DIR/usr/bin" \
         "$ARTIFACT_DIR/usr/lib64/security" \
         "$ARTIFACT_DIR/etc/chissu-pam" \
         "$ARTIFACT_DIR/usr/share/doc/chissu-pam" \
         "$ARTIFACT_DIR/var/lib/chissu-pam/dlib-models" \
         "$ARTIFACT_DIR/var/lib/chissu-pam/embeddings" \
         "$ARTIFACT_DIR/var/lib/chissu-pam/install"

cp "$BIN_SRC" "$ARTIFACT_DIR/usr/bin/chissu-cli"
cp "$PAM_SRC" "$ARTIFACT_DIR/usr/lib64/security/libpam_chissu.so"
cp "$REPO_ROOT/build/package/assets/etc/chissu-pam/config.toml" "$ARTIFACT_DIR/etc/chissu-pam/config.toml"
cp "$REPO_ROOT/build/package/assets/usr/share/doc/chissu-pam/README.RPM" "$ARTIFACT_DIR/usr/share/doc/chissu-pam/README.RPM"
touch "$ARTIFACT_DIR/var/lib/chissu-pam/dlib-models/.keep" "$ARTIFACT_DIR/var/lib/chissu-pam/embeddings/.keep" "$ARTIFACT_DIR/var/lib/chissu-pam/install/.keep"
cp "$REPO_ROOT/LICENSE" "$STAGING_ROOT/LICENSE"

log "Rendering spec"
sed \
  -e "s/__VERSION__/$VERSION/g" \
  -e "s/__RELEASE__/$RELEASE/g" \
  -e "s/__ARCH__/$ARCH/g" \
  "$SPEC_TEMPLATE" > "$SPEC_PATH"

log "Creating source tarball"
tar -C "$WORK_ROOT" -czf "$RPMS_DIR/SOURCES/chissu-pam-$VERSION.tar.gz" "chissu-pam-$VERSION"

log "Running rpmbuild"
rpmbuild --define "_topdir $RPMS_DIR" -bb "$SPEC_PATH"

RPM_OUTPUT=$(find "$RPMS_DIR/RPMS" -type f -name "*.rpm" | head -n1)
if [[ -z "$RPM_OUTPUT" ]]; then
  echo "rpmbuild did not produce an RPM" >&2
  exit 1
fi

FINAL_NAME="$DIST_DIR/chissu-pam_${VERSION}_${DISTRO}_${ARCH}.rpm"
log "Copying $(basename "$RPM_OUTPUT") -> $(basename "$FINAL_NAME")"
cp "$RPM_OUTPUT" "$FINAL_NAME"
log "Done: $FINAL_NAME"
