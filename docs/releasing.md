# Release Process

This guide covers maintainer-facing release automation for chissu-pam.

## Tag format

Push a semver tag to start the release workflow:

```bash
git tag v0.3.0
git push origin v0.3.0
```

Prerelease tags are also supported:

```bash
git tag v0.3.0-rc1
git push origin v0.3.0-rc1
```

Tags must match either `v<MAJOR>.<MINOR>.<PATCH>` or
`v<MAJOR>.<MINOR>.<PATCH>-<prerelease>`.

## GitHub Actions workflow

The `Release Packages` workflow builds release-specific packages inside
container images and publishes them to GitHub Releases for the same tag.

Expected release assets include:

```text
chissu-pam_<version>_debian-12_amd64.deb
chissu-pam_<version>_ubuntu-24.04_amd64.deb
chissu-pam_<version>_ubuntu-25.10_amd64.deb
chissu-pam_<version>_<distro>_x86_64.rpm
```

Release notes are generated automatically. Edit the GitHub Release manually
after the workflow finishes if user-facing notes need more detail.

## RPM prerelease handling

RPM does not accept every semver prerelease string directly as a package
version. For prerelease tags, the generated artifact keeps the original semver
in its filename, while the RPM spec normalizes the internal fields to:

```text
Version=<core>
Release=0.<release>.<prerelease>
```

This keeps RC packages installable and preserves upgrade ordering.

## Failed workflow recovery

If the release workflow fails, fix the underlying issue and re-run the failed
jobs for the tag in GitHub Actions. Existing release assets are replaced when
uploads succeed.
