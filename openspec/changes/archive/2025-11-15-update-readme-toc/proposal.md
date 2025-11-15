# Proposal: Update README Table of Contents

## Why
- The README has grown to cover capture, enrollment, PAM configuration, and testing topics but lacks a table of contents, so operators must scroll through hundreds of lines to find the right section.
- Getting-started guidance is fragmented: prerequisites list packages but not actual install commands or links to the dlib models we require.
- Installation, usage, and configuration details for `chissu-cli enroll` and the PAM stack are buried later in the file instead of being discoverable from a conventional "Why / Getting Started / Usage / Configuration" structure.
- Security differentiators (Secret Service-backed descriptor encryption and the ability to enroll without `root`) are nowhere near the top of the README even though they answer common "Why this project?" questions.

## What Changes
- Introduce an OSS-style table of contents that links to Overview, Why This Project, Getting Started (Prerequisites + Installation), Usage, Configuration, Testing, and License sections.
- Add a "Why This Project" section that highlights descriptor encryption through Secret Service and the fact that admin privileges are needed only for PAM wiring, not daily enrollment.
- Rework "Getting Started" to include concrete package installation commands (Debian-based example) plus a subsection on downloading/storing the dlib landmark and encoder models.
- Document installation tasks for placing binaries, config files, dlib weights, and `/etc/pam.d` service snippets so readers see exactly how to deploy the CLI + PAM module.
- Expand the Usage section with `chissu-cli enroll` walkthroughs, including how to run as the invoking user and an explicit `sudo` example when targeting another account.
- Add a Configuration section that explains `chissu-pam`'s TOML file(s), the precedence rules, and the keys relevant to capture/enrollment.

## Impact
- Maintainers, operators, and reviewers can jump straight to the information they need without skimming the entire README.
- Security posture and operating assumptions are immediately visible to new contributors evaluating the project.
- Onboarding becomes smoother because installation commands, PAM snippets, and enrollment instructions live under predictable anchors referenced by docs/specs.
