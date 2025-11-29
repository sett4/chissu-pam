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
