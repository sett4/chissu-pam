# Proposal: Rename CLI To chissu-pam

## Why
- The project graduated from its experimental "study-rust-v4l2" phase and now targets production usage as **chissu-pam**.
- Specs, docs, and code currently reference the former name, creating confusion for operators and packaging work.
- Default filesystem paths and environment variables also embed the deprecated name, so we need a cohesive rebrand before the next release.

## What Changes
- Rename the primary CLI/binary crate, `cargo` package, and Clap command metadata to `chissu-pam`.
- Update default descriptor storage paths and related environment variables to `/var/lib/chissu-pam/models` and `CHISSU_PAM_STORE_DIR`.
- Refresh documentation (README, docs/pam-auth.md, AGENTS constitution) to use the new branding.
- Modify the `face-features`, `infrared-capture`, and `pam-face-auth` specs so their command examples and defaults match the new name.

## Impact
- Breaking change for anyone invoking the old `study-rust-v4l2` binary or relying on the old env var; we will call this out in release notes.
- PAM module defaults stay aligned with the CLI defaults, reducing operator surprises.
- No behavioral changes to capture or face-processing logic—only naming, paths, and metadata.
