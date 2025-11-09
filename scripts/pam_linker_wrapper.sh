#!/usr/bin/env bash
set -euo pipefail

REAL_LINKER=${PAM_REAL_LINKER:-cc}

out=""
next_is_output=0
for arg in "$@"; do
  if [[ ${next_is_output} -eq 1 ]]; then
    out="${arg}"
    next_is_output=0
  elif [[ "${arg}" == "-o" ]]; then
    next_is_output=1
  fi
done

"${REAL_LINKER}" "$@"

if [[ -n "${out}" && -f "${out}" && "${out}" == *"libpam_chissu.so" ]]; then
  dir=$(dirname "${out}")
  pam_artifact="${dir}/pam_chissu.so"
  cp -f "${out}" "${pam_artifact}"
  ln -sf "pam_chissu.so" "${dir}/libpam_chissuauth.so"

  if [[ $(basename "${dir}") == "deps" ]]; then
    parent=$(dirname "${dir}")
    cp -f "${out}" "${parent}/pam_chissu.so"
    ln -sf "pam_chissu.so" "${parent}/libpam_chissuauth.so"
  fi
fi
