1. [x] Add a Markdown table of contents near the top of README.md linking to Overview, Why This Project, Getting Started, Usage, Configuration, Testing, and License anchors.
2. [x] Write a "Why This Project" section that explains descriptor encryption via Secret Service and clarifies when root privileges are or are not required.
3. [x] Revise Getting Started → Prerequisites with concrete package installation commands and add a subsection describing how to download/store the required dlib models.
4. [x] Flesh out Getting Started → Installation to cover placing the CLI binary, PAM module, config files, dlib weights, and `/etc/pam.d` snippets.
5. [x] Expand the Usage section with `chissu-cli enroll` walkthroughs, including a standard invocation and an explicit `sudo` example targeting another user.
6. [x] Add a Configuration section that documents the `chissu-pam` TOML file(s), precedence rules, and key settings the CLI/PAM respect.
7. [x] Run `openspec validate update-readme-toc --strict` (and any README link checker if available) to confirm the documentation/spec alignment.
