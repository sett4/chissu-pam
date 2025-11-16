## 1. Rescope capture capability
- [x] 1.1 Rename `capture-cli` spec folder to `chissu-cli-capture` and align Purpose/scope text.
- [x] 1.2 Remove `doctor` requirement from the capture spec and ensure remaining requirements reference the new capability name.

## 2. Add dedicated doctor capability
- [x] 2.1 Create `openspec/specs/chissu-cli-doctor/spec.md` with the existing doctor requirements and scenarios.

## 3. Update references and validate
- [x] 3.1 Update active specs that referenced `capture-cli` to use `chissu-cli-capture`.
- [x] 3.2 Run `openspec validate refactor-capture-doctor-specs --strict`.
