#!/usr/bin/env bash
set -euo pipefail

# Chissu installer for Ubuntu/Debian, Rocky Linux, and Arch Linux.
# Deploys chissu-cli, libpam_chissu.so, config, and dlib model assets.

ARTIFACT_DIR=${ARTIFACT_DIR:-target/release}
MODEL_DIR=${MODEL_DIR:-/var/lib/chissu-pam/dlib-models}
STORE_DIR=${STORE_DIR:-/var/lib/chissu-pam/models}
CONFIG_PATH=${CONFIG_PATH:-/etc/chissu-pam/config.toml}
OS_RELEASE=${OS_RELEASE:-/etc/os-release}
DRY_RUN=${DRY_RUN:-0}
FORCE=${FORCE:-0}
SKIP_DOWNLOAD=${SKIP_DOWNLOAD:-0}

log() { printf '%s\n' "$*"; }
warn() { printf 'WARN: %s\n' "$*" >&2; }
err() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
run() {
  if [[ $DRY_RUN -eq 1 ]]; then
    log "[dry-run] $*"
  else
    "$@"
  fi
}

usage() {
  cat <<'USAGE'
Usage: install-chissu.sh [options]

Options:
  --artifact-dir DIR      Directory containing chissu-cli and libpam_chissu.so (default: target/release)
  --model-dir DIR         Destination for dlib model .dat files (default: /var/lib/chissu-pam/dlib-models)
  --store-dir DIR         Destination for embedding store (default: /var/lib/chissu-pam/models)
  --config-path PATH      Destination for chissu-pam config (default: /etc/chissu-pam/config.toml)
  --dry-run               Print actions without making changes
  --force                 Overwrite existing binaries/configs (with backup) when present
  --skip-model-download   Skip downloading dlib model archives
  -h, --help              Show this help

Environment overrides: ARTIFACT_DIR, MODEL_DIR, STORE_DIR, CONFIG_PATH, DRY_RUN, FORCE, SKIP_DOWNLOAD
USAGE
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --artifact-dir) ARTIFACT_DIR=$2; shift 2 ;;
      --model-dir) MODEL_DIR=$2; shift 2 ;;
      --store-dir) STORE_DIR=$2; shift 2 ;;
      --config-path) CONFIG_PATH=$2; shift 2 ;;
      --dry-run) DRY_RUN=1; shift ;;
      --force) FORCE=1; shift ;;
      --skip-model-download) SKIP_DOWNLOAD=1; shift ;;
      -h|--help) usage; exit 0 ;;
      *) err "Unknown option: $1" ;;
    esac
  done
}

require_root() {
  if [[ $DRY_RUN -eq 1 ]]; then
    warn "Running in dry-run without root; commands will not execute"
    return
  fi
  if [[ $(id -u) -ne 0 ]]; then
    err "Run as root (sudo) to install packages and write to system paths"
  fi
}

os_flavor=""
detect_os() {
  if [[ ! -f "$OS_RELEASE" ]]; then
    err "$OS_RELEASE missing; cannot detect OS"
  fi
  # shellcheck source=/dev/null
  source "$OS_RELEASE"
  case "${ID:-}" in
    ubuntu|debian) os_flavor="debian" ;;
    rocky) os_flavor="rocky" ;;
    arch) os_flavor="arch" ;;
    *) ;;
  esac
  if [[ -z "$os_flavor" && "${ID_LIKE:-}" =~ debian ]]; then
    os_flavor="debian"
  elif [[ -z "$os_flavor" && "${ID_LIKE:-}" =~ rhel ]]; then
    os_flavor="rocky"
  elif [[ -z "$os_flavor" && "${ID_LIKE:-}" =~ arch ]]; then
    os_flavor="arch"
  fi
  [[ -n "$os_flavor" ]] || err "Unsupported distro (ID=${ID:-unknown})"
}

install_prereqs_debian() {
  log "Installing dependencies via apt..."
  run apt-get update
  run apt-get install -y build-essential pkg-config libdlib-dev libopenblas-dev liblapack-dev libudev-dev curl bzip2
}

install_prereqs_rocky() {
  log "Installing dependencies via dnf (enabling EPEL/CRB if needed)..."
  run dnf install -y epel-release
  run dnf config-manager --set-enabled crb || run dnf config-manager --set-enabled powertools || true
  run dnf groupinstall -y "Development Tools"
  run dnf install -y dlib dlib-devel openblas-devel lapack-devel gtk3-devel systemd-devel pkgconfig curl bzip2
}

install_prereqs_arch() {
  log "Installing dependencies via pacman..."
  run pacman -Sy --noconfirm
  run pacman -S --needed --noconfirm base-devel pkgconf openblas lapack gtk3 systemd curl bzip2
  run sudo -u $SUDO_USER yay -S --noconfirm dlib
}

ensure_dirs() {
  for dir in /etc/chissu-pam /usr/local/etc/chissu-pam "$MODEL_DIR" "$STORE_DIR"; do
    if [[ ! -d "$dir" ]]; then
      log "Creating directory $dir"
      run mkdir -p "$dir"
    fi
    if [[ "$dir" == "$STORE_DIR" ]]; then
      run chmod 0777 "$dir"
    else
      run chmod 0755 "$dir"
    fi
    run chown root:root "$dir"
  done
}

default_config() {
  cat <<EOF
# chissu-pam default configuration
similarity_threshold = 0.9
capture_timeout_secs = 5
frame_interval_millis = 500
video_device = "/dev/video2"
pixel_format = "GREY"
warmup_frames = 4
jitters = 1
embedding_store_dir = "$STORE_DIR"
landmark_model = "$MODEL_DIR/shape_predictor_68_face_landmarks.dat"
encoder_model = "$MODEL_DIR/dlib_face_recognition_resnet_model_v1.dat"
require_secret_service = true
EOF
}

backup_if_needed() {
  local target=$1
  if [[ -e "$target" && $FORCE -eq 1 ]]; then
    local stamp
    stamp=$(date +%Y%m%d%H%M%S)
    local backup="${target}.bak-${stamp}"
    log "Backing up $target -> $backup"
    run cp -p "$target" "$backup"
  elif [[ -e "$target" ]]; then
    warn "$target exists; skip (use --force to overwrite)"
    return 1
  fi
  return 0
}

install_config() {
  if backup_if_needed "$CONFIG_PATH"; then
    log "Writing default config to $CONFIG_PATH"
    if [[ $DRY_RUN -eq 1 ]]; then
      default_config | sed 's/^/[dry-run config] /'
    else
      default_config > "$CONFIG_PATH"
      run chmod 0644 "$CONFIG_PATH"
      run chown root:root "$CONFIG_PATH"
    fi
  fi
}

copy_artifact() {
  local src=$1 dest=$2 mode=$3
  [[ -f "$src" ]] || err "Missing artifact: $src"
  if backup_if_needed "$dest"; then
    log "Installing $(basename "$src") to $dest"
    run install -m "$mode" "$src" "$dest"
  fi
}

install_binaries() {
  local cli_src="$ARTIFACT_DIR/chissu-cli"
  local pam_src="$ARTIFACT_DIR/libpam_chissu.so"
  local pam_dest
  if [[ "$os_flavor" == "rocky" ]]; then
    pam_dest="/usr/lib64/security/libpam_chissu.so"
  else
    pam_dest="/lib/security/libpam_chissu.so"
  fi
  copy_artifact "$cli_src" /usr/local/bin/chissu-cli 0755
  copy_artifact "$pam_src" "$pam_dest" 0644
  if [[ "$os_flavor" == "rocky" && $DRY_RUN -eq 0 && -x /sbin/restorecon ]]; then
    log "Applying SELinux context to $pam_dest"
    run /sbin/restorecon "$pam_dest"
  fi
}

fetch_model() {
  local name=$1 url=$2
  local target="$MODEL_DIR/${name%.bz2}"
  if [[ -f "$target" ]]; then
    log "Model present: $target (skip download)"
    return
  fi
  [[ $SKIP_DOWNLOAD -eq 0 ]] || { warn "Skipping download for $name"; return; }
  local tmp_archive="$MODEL_DIR/$name"
  log "Downloading $name"
  run curl -L "$url" -o "$tmp_archive"
  log "Unpacking $name"
  if [[ $DRY_RUN -eq 1 ]]; then
    log "[dry-run] bunzip2 $tmp_archive"
  else
    bunzip2 -f "$tmp_archive"
  fi
}

provision_models() {
  fetch_model "shape_predictor_68_face_landmarks.dat.bz2" "https://dlib.net/files/shape_predictor_68_face_landmarks.dat.bz2"
  fetch_model "dlib_face_recognition_resnet_model_v1.dat.bz2" "https://dlib.net/files/dlib_face_recognition_resnet_model_v1.dat.bz2"
}

main() {
  parse_args "$@"
  require_root
  detect_os
  log "Detected OS flavor: $os_flavor"

  case "$os_flavor" in
    debian) install_prereqs_debian ;;
    rocky) install_prereqs_rocky ;;
    arch) install_prereqs_arch ;;
    *) err "Unsupported distro" ;;
  esac

  ensure_dirs
  install_config
  install_binaries
  provision_models

  log "Installation complete. Configure /etc/pam.d/<service> to load libpam_chissu.so."
}

main "$@"
OS_RELEASE=${OS_RELEASE:-/etc/os-release}
