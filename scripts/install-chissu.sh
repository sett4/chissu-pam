#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname "$0")" && pwd -P)

# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/install_common.sh"

# Chissu installer for Ubuntu/Debian, Rocky Linux, and Arch Linux.
# Deploys chissu-cli, libpam_chissu.so, config, and dlib model assets.

ARTIFACT_DIR=${ARTIFACT_DIR:-target/release}
MODEL_DIR=${MODEL_DIR:-$INSTALL_COMMON_MODEL_DIR}
STORE_DIR=${STORE_DIR:-$INSTALL_COMMON_EMBED_DIR}
CONFIG_PATH=${CONFIG_PATH:-$INSTALL_COMMON_CONFIG_PATH}
OS_RELEASE=${OS_RELEASE:-/etc/os-release}
DRY_RUN=${DRY_RUN:-0}
FORCE=${FORCE:-0}
SKIP_DOWNLOAD=${SKIP_DOWNLOAD:-0}
UNINSTALL=${UNINSTALL:-0}
STATE_DIR=${STATE_DIR:-/var/lib/chissu-pam/install}

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
  --uninstall             Remove PAM wiring only (restores previous distro-specific state when possible)
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
      --uninstall) UNINSTALL=1; shift ;;
      --force) FORCE=1; shift ;;
      --skip-model-download) SKIP_DOWNLOAD=1; shift ;;
      -h|--help) usage; exit 0 ;;
      *) err "Unknown option: $1" ;;
    esac
  done
}

timestamp() { date +%Y%m%d%H%M%S; }

require_root() {
  if [[ $DRY_RUN -eq 1 ]]; then
    warn "Running in dry-run without root; commands will not execute"
    return
  fi
  if [[ $(id -u) -ne 0 ]]; then
    err "Run as root (sudo) to install packages and write to system paths"
  fi
}

ensure_state_dir() {
  if [[ $DRY_RUN -eq 1 ]]; then
    log "[dry-run] mkdir -p $STATE_DIR"
  else
    mkdir -p "$STATE_DIR"
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
  run apt-get install -y "${DEBIAN_BUILD_PREREQS[@]}"
}

install_prereqs_rocky() {
  log "Installing dependencies via dnf (enabling EPEL/CRB if needed)..."
  run dnf install -y epel-release
  run dnf config-manager --set-enabled crb || run dnf config-manager --set-enabled powertools || true
  run dnf groupinstall -y "Development Tools"
  run dnf install -y "${ROCKY_BUILD_PREREQS[@]}"
}

install_prereqs_arch() {
  log "Installing dependencies via pacman..."
  run pacman -Sy --noconfirm
  run pacman -S --needed --noconfirm "${ARCH_BUILD_PREREQS[@]}"
  if [[ -n "${SUDO_USER:-}" ]]; then
    run sudo -u "$SUDO_USER" yay -S --noconfirm dlib
  else
    warn "SUDO_USER not set; skipping yay dlib install"
  fi
}

ensure_dirs() {
  for dir in /etc/chissu-pam /usr/local/etc/chissu-pam "$MODEL_DIR" "$STORE_DIR"; do
    if [[ ! -d "$dir" ]]; then
      log "Creating directory $dir"
      run mkdir -p "$dir"
    fi
    run chmod 0755 "$dir"
    run chown root:root "$dir"
  done
}

backup_if_needed() {
  local target=$1
  if [[ -e "$target" && $FORCE -eq 1 ]]; then
    local stamp
    stamp=$(timestamp)
    local backup="${target}.bak-${stamp}"
    log "Backing up $target -> $backup"
    run cp -p "$target" "$backup"
  elif [[ -e "$target" ]]; then
    warn "$target exists; skip (use --force to overwrite)"
    return 1
  fi
  return 0
}

backup_to_state() {
  local target=$1
  [[ -f "$target" ]] || return 0
  local dest
  dest="$STATE_DIR/$(basename "$target").$(timestamp).bak"
  if [[ $DRY_RUN -eq 1 ]]; then
    log "[dry-run] backup $target -> $dest"
    return 0
  fi
  cp -p "$target" "$dest"
}

install_config() {
  if backup_if_needed "$CONFIG_PATH"; then
    log "Writing default config to $CONFIG_PATH"
    if [[ $DRY_RUN -eq 1 ]]; then
      render_default_config "$STORE_DIR" "$MODEL_DIR" | sed 's/^/[dry-run config] /'
    else
      render_default_config "$STORE_DIR" "$MODEL_DIR" > "$CONFIG_PATH"
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
  fetch_model "$INSTALL_COMMON_LANDMARK_FILE.bz2" "$INSTALL_COMMON_LANDMARK_URL"
  fetch_model "$INSTALL_COMMON_ENCODER_FILE.bz2" "$INSTALL_COMMON_ENCODER_URL"
}

insert_before_first_match() {
  local file=$1 pattern=$2 newline=$3
  if grep -Fq "$newline" "$file"; then
    log "Line already present in $file: $newline"
    return 0
  fi
  local tmp
  tmp=$(mktemp)
  awk -v pat="$pattern" -v ins="$newline" 'found==0 && $0 ~ pat {print ins; found=1} {print} END {if(found==0) print ins}' "$file" > "$tmp"
  if [[ $DRY_RUN -eq 1 ]]; then
    log "[dry-run] would insert before /$pattern/ in $file: $newline"
    rm -f "$tmp"
  else
    mv "$tmp" "$file"
  fi
}

ensure_file_with_content() {
  local dest=$1 src=$2 mode=$3
  if [[ $DRY_RUN -eq 1 ]]; then
    log "[dry-run] install -m $mode $src $dest"
    return
  fi
  install -m "$mode" "$src" "$dest"
}

pam_auth_line="auth    sufficient    libpam_chissu.so"

pam_uninstall_debian() {
  local snippet=/usr/share/pam-configs/chissu
  if [[ -f "$snippet" ]]; then
    backup_to_state "$snippet"
  fi
  log "Removing pam-auth-update entry (Debian/Ubuntu)"
  if [[ $DRY_RUN -eq 1 ]]; then
    log "[dry-run] pam-auth-update --package --remove chissu"
    log "[dry-run] rm -f $snippet"
    return
  fi
  if command -v pam-auth-update >/dev/null 2>&1; then
    pam-auth-update --package --remove chissu || warn "pam-auth-update remove failed"
  fi
  rm -f "$snippet"
}

pam_install_debian() {
  command -v pam-auth-update >/dev/null 2>&1 || err "pam-auth-update missing; cannot wire PAM"
  local snippet=/usr/share/pam-configs/chissu
  backup_to_state "$snippet"
  log "Writing pam-auth-update snippet to $snippet"
  ensure_file_with_content "$snippet" "$SCRIPT_DIR/pam/chissu-debian.conf" 0644
  log "Running pam-auth-update --package --enable chissu"
  if [[ $DRY_RUN -eq 1 ]]; then
    log "[dry-run] pam-auth-update --package --enable chissu"
  else
    pam-auth-update --package --enable chissu
  fi
}

authselect_profile_name=custom/chissu
authselect_prev_file="$STATE_DIR/authselect.previous"

authselect_current_profile() {
  authselect current 2>/dev/null | awk -F: '/^Profile ID:/ {gsub(/^ */,"",$2); print $2}'
}

pam_uninstall_rhel() {
  command -v authselect >/dev/null 2>&1 || { warn "authselect missing; nothing to uninstall"; return; }
  local previous="sssd"
  if [[ -f "$authselect_prev_file" ]]; then
    previous=$(cat "$authselect_prev_file")
  fi
  log "Restoring authselect profile $previous"
  if [[ $DRY_RUN -eq 1 ]]; then
    log "[dry-run] authselect select $previous"
    log "[dry-run] authselect apply-changes"
    log "[dry-run] rm -rf /etc/authselect/custom/chissu"
    return
  fi
  authselect select "$previous" --force
  authselect apply-changes
  rm -rf /etc/authselect/custom/chissu
}

pam_install_rhel() {
  command -v authselect >/dev/null 2>&1 || err "authselect missing; cannot wire PAM"
  if ! authselect check; then
    err "authselect reports unsynced state; run 'authselect apply-changes' and retry"
  fi
  local current
  current=$(authselect_current_profile || true)
  ensure_state_dir
  if [[ -n "$current" ]]; then
    if [[ $DRY_RUN -eq 1 ]]; then
      log "[dry-run] saving current authselect profile $current to $authselect_prev_file"
    else
      printf '%s' "$current" > "$authselect_prev_file"
    fi
  fi

  log "Creating authselect custom profile chissu from sssd"
  if [[ $DRY_RUN -eq 1 ]]; then
    log "[dry-run] authselect create-profile chissu -b sssd"
  else
    authselect create-profile chissu -b sssd || true
  fi

  local profile_dir=/etc/authselect/custom/chissu
  local system_auth="$profile_dir/system-auth"
  local password_auth="$profile_dir/password-auth"

  for f in "$system_auth" "$password_auth"; do
    [[ -f "$f" ]] || err "Expected authselect template missing: $f"
    backup_to_state "$f"
    insert_before_first_match "$f" "pam_unix.so" "$pam_auth_line"
  done

  log "Selecting authselect profile $authselect_profile_name"
  if [[ $DRY_RUN -eq 1 ]]; then
    log "[dry-run] authselect select $authselect_profile_name"
    log "[dry-run] authselect apply-changes"
  else
    authselect select "$authselect_profile_name" --force
    authselect apply-changes
  fi
}

arch_target_file=""

pick_arch_target() {
  if [[ -f /etc/pam.d/system-local-login ]]; then
    arch_target_file=/etc/pam.d/system-local-login
  elif [[ -f /etc/pam.d/login ]]; then
    arch_target_file=/etc/pam.d/login
  else
    err "Neither /etc/pam.d/system-local-login nor /etc/pam.d/login found"
  fi
}

pam_uninstall_arch() {
  local snippet=/etc/pam.d/chissu
  pick_arch_target
  local include_line="auth    include   chissu"
  if [[ -f "$arch_target_file" ]]; then
    backup_to_state "$arch_target_file"
    if [[ $DRY_RUN -eq 1 ]]; then
      log "[dry-run] removing include from $arch_target_file"
    else
      grep -v "^$include_line" "$arch_target_file" > "$arch_target_file.tmp" && mv "$arch_target_file.tmp" "$arch_target_file"
    fi
  fi
  if [[ -f "$snippet" ]]; then
    backup_to_state "$snippet"
    if [[ $DRY_RUN -eq 1 ]]; then
      log "[dry-run] rm -f $snippet"
    else
      rm -f "$snippet"
    fi
  fi
}

pam_install_arch() {
  pick_arch_target
  local snippet=/etc/pam.d/chissu
  backup_to_state "$arch_target_file"
  backup_to_state "$snippet"
  log "Writing PAM snippet $snippet"
  ensure_file_with_content "$snippet" "$SCRIPT_DIR/pam/chissu-arch.conf" 0644
  local include_line="auth    include   chissu"
  insert_before_first_match "$arch_target_file" "pam_unix.so" "$include_line"
}

wire_pam() {
  case "$os_flavor" in
    debian) pam_install_debian ;;
    rocky) pam_install_rhel ;;
    arch) pam_install_arch ;;
    *) err "Unsupported distro for PAM wiring" ;;
  esac
}

unwire_pam() {
  case "$os_flavor" in
    debian) pam_uninstall_debian ;;
    rocky) pam_uninstall_rhel ;;
    arch) pam_uninstall_arch ;;
    *) err "Unsupported distro for PAM uninstall" ;;
  esac
}

main() {
  parse_args "$@"
  require_root
  detect_os
  log "Detected OS flavor: $os_flavor"

  ensure_state_dir

  if [[ $UNINSTALL -eq 1 ]]; then
    unwire_pam
    log "PAM uninstall complete. Binaries/config/models were left untouched."
    return
  fi

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

  wire_pam

  log "Installation complete. PAM wiring applied for $os_flavor with libpam_chissu.so placed before pam_unix.so."
}

main "$@"
