## ADDED Requirements
### Requirement: Face Feature Comparison Command
The CLI MUST provide a `faces compare` subcommand that consumes descriptor JSON files produced by `faces extract` and reports cosine similarity scores between an input file and one or more comparison targets.

#### Scenario: Scores each comparison target
- **GIVEN** an input descriptor file containing at least one face
- **AND** two comparison descriptor files
- **WHEN** the operator runs `study-rust-v4l2 faces compare --input <file> --compare-target <target1> --compare-target <target2>`
- **THEN** the command computes cosine similarity for every face pair between the input file and each comparison file
- **AND** the command reports the highest similarity per comparison target in descending order
- **AND** the process exits with status code 0 after listing all scores

#### Scenario: JSON comparison run
- **WHEN** the operator runs the command with `--json`
- **THEN** stdout emits a single JSON array where each element contains the comparison path and the reported similarity score
- **AND** informational logs are suppressed from stdout while stderr still carries errors

#### Scenario: Missing comparison file aborts
- **WHEN** any specified input or comparison descriptor file is unreadable or missing
- **THEN** the command emits an error describing the filesystem issue to stderr
- **AND** no similarity scores are emitted
- **AND** the process exits with status code 2
