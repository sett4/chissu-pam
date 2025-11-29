#!/usr/bin/env bash
set -euo pipefail

# Shared defaults and helpers for installer and packaging scripts.
# Keep POSIX-friendly bits in render_runtime_helper(); other functions may use bash arrays.

# Canonical defaults
# shellcheck disable=SC2034  # exported for consumers
INSTALL_COMMON_VIDEO_DEVICE="/dev/video2"
INSTALL_COMMON_PIXEL_FORMAT="GREY"
INSTALL_COMMON_WARMUP_FRAMES="4"
INSTALL_COMMON_JITTERS="1"
INSTALL_COMMON_EMBED_DIR="/var/lib/chissu-pam/embeddings"
INSTALL_COMMON_MODEL_DIR="/var/lib/chissu-pam/dlib-models"
# shellcheck disable=SC2034  # exported for consumers
INSTALL_COMMON_CONFIG_PATH="/etc/chissu-pam/config.toml"

INSTALL_COMMON_LANDMARK_FILE="shape_predictor_68_face_landmarks.dat"
INSTALL_COMMON_ENCODER_FILE="dlib_face_recognition_resnet_model_v1.dat"
# shellcheck disable=SC2034  # exported for consumers
INSTALL_COMMON_LANDMARK_URL="https://dlib.net/files/${INSTALL_COMMON_LANDMARK_FILE}.bz2"
# shellcheck disable=SC2034  # exported for consumers
INSTALL_COMMON_ENCODER_URL="https://dlib.net/files/${INSTALL_COMMON_ENCODER_FILE}.bz2"

# Build-time prerequisite packages per distro (used by installer preflight and packaging helpers)
# shellcheck disable=SC2034  # exported for consumers
DEBIAN_BUILD_PREREQS=(build-essential pkg-config libdlib-dev libopenblas-dev liblapack-dev libudev-dev curl bzip2 rustc cargo)
# shellcheck disable=SC2034  # exported for consumers
ROCKY_BUILD_PREREQS=(dnf-plugins-core dlib dlib-devel openblas-devel lapack-devel gtk3-devel systemd-devel curl bzip2 gcc gcc-c++ make)
# shellcheck disable=SC2034  # exported for consumers
ARCH_BUILD_PREREQS=(base-devel pkgconf openblas lapack gtk3 systemd curl bzip2 dlib)

render_default_config() {
  local store_dir=${1:-$INSTALL_COMMON_EMBED_DIR}
  local model_dir=${2:-$INSTALL_COMMON_MODEL_DIR}
  cat <<EOF_CFG
# chissu-pam default configuration
similarity_threshold = 0.9
capture_timeout_secs = 5
frame_interval_millis = 500
video_device = "$INSTALL_COMMON_VIDEO_DEVICE"
pixel_format = "$INSTALL_COMMON_PIXEL_FORMAT"
warmup_frames = $INSTALL_COMMON_WARMUP_FRAMES
jitters = $INSTALL_COMMON_JITTERS
embedding_store_dir = "$store_dir"
landmark_model = "$model_dir/$INSTALL_COMMON_LANDMARK_FILE"
encoder_model = "$model_dir/$INSTALL_COMMON_ENCODER_FILE"
require_secret_service = true
EOF_CFG
}

# Emit a POSIX shell helper for runtime hooks (postinst/%post) to download models.
render_runtime_helper() {
  cat <<'EOF_RT'
#!/bin/sh
set -e

INSTALL_COMMON_MODEL_DIR=${INSTALL_COMMON_MODEL_DIR:-/var/lib/chissu-pam/dlib-models}
INSTALL_COMMON_EMBED_DIR=${INSTALL_COMMON_EMBED_DIR:-/var/lib/chissu-pam/embeddings}
LANDMARK_FILE=shape_predictor_68_face_landmarks.dat
ENCODER_FILE=dlib_face_recognition_resnet_model_v1.dat
LANDMARK_URL=https://dlib.net/files/shape_predictor_68_face_landmarks.dat.bz2
ENCODER_URL=https://dlib.net/files/dlib_face_recognition_resnet_model_v1.dat.bz2
SKIP_DOWNLOAD=${CHISSU_PAM_SKIP_MODEL_DOWNLOAD:-0}

log() {
    echo "chissu-pam: $*"
}

maybe_download() {
    name="$1" url="$2" dest="$3"
    if [ -f "$dest" ]; then
        log "$name already present at $dest; skipping"
        return 0
    fi
    if [ "$SKIP_DOWNLOAD" = "1" ]; then
        log "skipping $name download (CHISSU_PAM_SKIP_MODEL_DOWNLOAD=1)"
        return 0
    fi
    tmp=$(mktemp -d)
    archive="$tmp/archive.bz2"
    log "downloading $name"
    if ! curl -fsSL "$url" -o "$archive"; then
        log "failed to download $url" >&2
        rm -rf "$tmp"
        exit 1
    fi
    if ! bzip2 -d -c "$archive" > "$dest"; then
        log "failed to decompress $archive" >&2
        rm -rf "$tmp"
        exit 1
    fi
    chmod 0644 "$dest"
    rm -rf "$tmp"
}

maybe_download_models() {
    install -d -m 0755 "$INSTALL_COMMON_MODEL_DIR" "$INSTALL_COMMON_EMBED_DIR"
    maybe_download "$LANDMARK_FILE" "$LANDMARK_URL" "$INSTALL_COMMON_MODEL_DIR/$LANDMARK_FILE"
    maybe_download "$ENCODER_FILE" "$ENCODER_URL" "$INSTALL_COMMON_MODEL_DIR/$ENCODER_FILE"
}
EOF_RT
}

# Convenience helper to write assets to a destination root (e.g., build/package/assets)
write_assets() {
  local dest_root=$1
  mkdir -p "$dest_root/etc/chissu-pam" "$dest_root/usr/share/chissu-pam"
  render_default_config "$INSTALL_COMMON_EMBED_DIR" "$INSTALL_COMMON_MODEL_DIR" > "$dest_root/etc/chissu-pam/config.toml"
  render_runtime_helper > "$dest_root/usr/share/chissu-pam/install-common.sh"
  chmod 0644 "$dest_root/usr/share/chissu-pam/install-common.sh"
}
