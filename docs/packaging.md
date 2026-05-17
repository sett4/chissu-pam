# Packaging Guide

This guide collects maintainer-facing package build details for chissu-pam.
End-user installation instructions stay in the README.

## Debian/Ubuntu packages

Build Debian-family packages from the repository root:

```bash
build/package-deb.sh --distro debian
build/package-deb.sh --distro ubuntu
```

The script expects Debian packaging tools such as `debhelper`, `dpkg-dev`, and
`curl`. Pass `--version` to override the workspace version or `--arch` for
non-`amd64` builds.

On a native Debian/Ubuntu host, the artifact name auto-detects the release and
lands in `dist/chissu-pam_<version>_<distro>-<release>_amd64.deb`, for example:

```text
dist/chissu-pam_0.6.0-rc3_ubuntu-25.10_amd64.deb
```

When cross-building inside a container, pass `--suite <codename>` plus
`--artifact-label <distro-release>` explicitly:

```bash
build/package-deb.sh --distro ubuntu --suite noble --artifact-label ubuntu-24.04
```

The package installs the CLI, PAM module, default configuration, PAM snippet,
and doc snippets under standard system paths. During installation, `postinst`
downloads the dlib model files into `/var/lib/chissu-pam/dlib-models` unless
`CHISSU_PAM_SKIP_MODEL_DOWNLOAD=1` is set before running `dpkg -i`.

Set `CHISSU_PAM_PURGE_MODELS=1` before purging the package if the downloaded
model files should be removed as well.

## RPM packages

Build RPM-family packages from the repository root:

```bash
build/package-rpm.sh --distro fedora
```

The script expects RPM packaging tools and native libraries such as
`rpm-build`, `createrepo-c`, `clang`, `pam-devel`, `dlib-devel`,
`openblas-devel`, `lapack-devel`, `gtk3-devel`, `systemd-devel`, and
`libudev-devel`.

Pass `--version <semver>` to override the workspace version or `--skip-build`
when release binaries were produced by another step. Artifacts land in:

```text
dist/chissu-pam_<version>_<distro>_x86_64.rpm
```

The RPM `%post` hook downloads the dlib model files into
`/var/lib/chissu-pam/dlib-models` unless `CHISSU_PAM_SKIP_MODEL_DOWNLOAD=1` is
set before installation. Set `CHISSU_PAM_PURGE_MODELS=1` before uninstalling to
remove those files.

RPM packages use `authselect` to create or refresh a `custom/chissu` profile and
insert `libpam_chissu.so` before `pam_unix.so` in `system-auth` and
`password-auth`. On erase, `%postun` restores the previously selected profile.

## Build RPMs via Docker

Use the Fedora builder image when `rpmbuild` or required headers are missing
locally:

```bash
docker build -t chissu-rpm -f build/package/rpm/Dockerfile .
docker run --rm -it \
  -v "$PWD":/workspace \
  -w /workspace \
  chissu-rpm \
  ./build/package-rpm.sh --distro fedora --version 0.3.0
```

The container writes artifacts back into `dist/`. Pass `--skip-build` if release
binaries already exist before invoking the container.

## Source install helper

For local source builds, produce release artifacts first:

```bash
CARGO_HOME="$(pwd)/.cargo-home" cargo build --release -p chissu-cli -p pam-chissu
```

Then run:

```bash
sudo scripts/install-chissu.sh \
  --artifact-dir target/release \
  --model-dir /var/lib/chissu-pam/dlib-models \
  --store-dir /var/lib/chissu-pam/embeddings
```

The installer detects Debian/Ubuntu, Fedora, Rocky Linux, and Arch Linux. It
does not install missing packages automatically; it prints the appropriate
package-manager command before continuing. It supports `--dry-run`, custom
paths via `--artifact-dir`, `--model-dir`, `--store-dir`, and `--config-path`,
and rollback of only the PAM wiring with:

```bash
sudo scripts/install-chissu.sh --uninstall
```
