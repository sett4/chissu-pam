#!/usr/bin/env bash
set -euo pipefail

PACKAGE_NAME=${PACKAGE_NAME:-chissu-pam}
ARCH=${ARCH:-amd64}
OUTPUT_DIR=${OUTPUT_DIR:-dist}
ARTIFACT_DIR=${ARTIFACT_DIR:-target/release}
VERSION=""
SKIP_BUILD=0
DEPENDS="libgtk-3-0, libudev1, libopenblas0, liblapack3, libdlib-dev"
MAINTAINER="chissu-pam maintainers <maintainers@example.com>"
DESCRIPTION="Face recognition CLI and PAM module for Ubuntu"

usage() {
  cat <<'USAGE'
Usage: build-deb.sh [options]

Options:
  --version VERSION     Override package version (defaults to Cargo workspace version)
  --arch ARCH           Debian architecture string (default: amd64)
  --artifact-dir DIR    Directory containing release artifacts (default: target/release)
  --output-dir DIR      Directory to place resulting .deb (default: dist)
  --skip-build          Do not run cargo build (expects artifacts to exist)
  --depends STRING      Override comma-separated dependency list
  --maintainer STRING   Maintainer field for control file
  --description STRING  Package description
  -h, --help            Show this help text

Environment overrides: PACKAGE_NAME, ARCH, OUTPUT_DIR, ARTIFACT_DIR
USAGE
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --version) VERSION=$2; shift 2 ;;
      --arch) ARCH=$2; shift 2 ;;
      --artifact-dir) ARTIFACT_DIR=$2; shift 2 ;;
      --output-dir) OUTPUT_DIR=$2; shift 2 ;;
      --skip-build) SKIP_BUILD=1; shift ;;
      --depends) DEPENDS=$2; shift 2 ;;
      --maintainer) MAINTAINER=$2; shift 2 ;;
      --description) DESCRIPTION=$2; shift 2 ;;
      -h|--help) usage; exit 0 ;;
      *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
    esac
  done
}

resolve_version() {
  if [[ -n "$VERSION" ]]; then
    return
  fi
  if [[ ! -f Cargo.toml ]]; then
    echo "Cargo.toml not found; run from repo root or pass --version" >&2
    exit 1
  fi
  VERSION=$(python3 - <<'PY'
import tomllib
from pathlib import Path
root = Path('Cargo.toml')
data = tomllib.loads(root.read_text())
print(data['workspace']['package']['version'])
PY
)
}

run_build() {
  if [[ $SKIP_BUILD -eq 1 ]]; then
    return
  fi
  echo "Building release artifacts..."
  CARGO_HOME="$(pwd)/.cargo-home" cargo build --release -p chissu-cli -p pam-chissu
}

copy_artifacts() {
  local staging=$1
  local cli_src="$ARTIFACT_DIR/chissu-cli"
  local pam_src="$ARTIFACT_DIR/libpam_chissu.so"
  [[ -f "$cli_src" ]] || { echo "Missing CLI artifact: $cli_src" >&2; exit 1; }
  [[ -f "$pam_src" ]] || { echo "Missing PAM artifact: $pam_src" >&2; exit 1; }
  install -Dm0755 "$cli_src" "$staging/usr/local/bin/chissu-cli"
  install -Dm0644 "$pam_src" "$staging/lib/security/libpam_chissu.so"
}

write_default_config() {
  cat <<'CFG'
# chissu-pam default configuration
similarity_threshold = 0.9
capture_timeout_secs = 5
frame_interval_millis = 500
video_device = "/dev/video2"
pixel_format = "GREY"
warmup_frames = 4
jitters = 1
embedding_store_dir = "/var/lib/chissu-pam/models"
landmark_model = "/var/lib/chissu-pam/dlib-models/shape_predictor_68_face_landmarks.dat"
encoder_model = "/var/lib/chissu-pam/dlib-models/dlib_face_recognition_resnet_model_v1.dat"
require_secret_service = true
CFG
}

stage_docs() {
  local staging=$1
  install -Dm0644 README.md "$staging/usr/share/doc/$PACKAGE_NAME/README.md"
  write_default_config | install -Dm0644 /dev/stdin \
    "$staging/usr/share/doc/$PACKAGE_NAME/examples/config.toml"
}

write_control() {
  local staging=$1
  local control_dir="$staging/DEBIAN"
  mkdir -p "$control_dir"
  local installed_size
  installed_size=$(du -sk "$staging" | cut -f1)
  cat > "$control_dir/control" <<CONTROL
Package: $PACKAGE_NAME
Version: $VERSION
Section: misc
Priority: optional
Architecture: $ARCH
Depends: $DEPENDS
Maintainer: $MAINTAINER
Installed-Size: $installed_size
Description: $DESCRIPTION
CONTROL
}

build_package() {
  local staging="build/deb/${PACKAGE_NAME}_${VERSION}_${ARCH}"
  rm -rf "$staging"
  mkdir -p "$staging"
  copy_artifacts "$staging"
  stage_docs "$staging"
  write_control "$staging"
  mkdir -p "$OUTPUT_DIR"
  local deb_path="$OUTPUT_DIR/${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"
  dpkg-deb --build "$staging" "$deb_path"
  echo "Package created: $deb_path"
}

main() {
  parse_args "$@"
  resolve_version
  run_build
  build_package
}

main "$@"
