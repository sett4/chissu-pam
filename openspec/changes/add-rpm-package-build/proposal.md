# Proposal: RPM Packaging Workflow

## Why
We currently only package chissu-pam as `.deb` artifacts and publish them to GitHub Releases. Fedora/RHEL/CentOS users still compile from source, which is error-prone and duplicates the same installation hurdles we just solved for Debian-based systems. Maintainers also asked to build proposals on the `t/add-rpm-package-build` branch, so this plan assumes that branch context.

## What Changes
- Introduce a standard RPM packaging script (mirroring the Debian helper) that stages binaries/config/docs, relies on distro tooling (e.g., `rpmbuild`), and leaves dlib model downloads to install-time scripts instead of bundling them.
- Define install-time post scripts that fetch dlib weights if missing, matching the Debian behaviour.
- Extend the GitHub Actions release workflow triggered by `v<MAJOR>.<MINOR>.<PATCH>` tags so it also builds RPM artifacts and uploads them to GitHub Releases next to the existing `.deb` files.
- Document the rpm-build instructions and automated release outputs so contributors know how to produce and verify the packages locally before tagging.
