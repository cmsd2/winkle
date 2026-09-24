# Spec Delta

## MODIFIED Requirements

### Requirement: List installed apps
`winkle list` SHALL print one line per installed winkle app, sorted by id, showing id, name, launch URL and profile mode (`shared` or `isolated`). Apps not installed by winkle SHALL NOT be listed.

#### Scenario: Two apps installed
- **WHEN** `github-com` (shared) and `app-hey-com` (isolated) are installed
- **THEN** `winkle list` prints `app-hey-com` then `github-com`, each with its name, URL and profile mode

#### Scenario: Other apps are ignored
- **WHEN** Chromium's own installed web apps or other desktop entries exist
- **THEN** they do not appear in `winkle list`

#### Scenario: Apps from the old hermit name are ignored
- **WHEN** a `hermit-github-com.desktop` entry written by the tool's previous name exists
- **THEN** it does not appear in `winkle list`

### Requirement: Empty state
When no apps are installed, `winkle list` SHALL print a short message saying so and exit with status 0.

#### Scenario: Nothing installed
- **WHEN** no winkle apps exist
- **THEN** a message says no apps are installed, and the exit status is 0

### Requirement: Machine-readable output
`winkle list --json` SHALL print a JSON array of objects with at least `id`, `name`, `url`, `profile` and `shortcuts` (an array of `{name, url}`). With no apps it SHALL print `[]`.

#### Scenario: JSON output
- **WHEN** one app is installed and the user runs `winkle list --json`
- **THEN** the output is a valid JSON array containing one object with those fields

### Requirement: Reflects hand edits
The list SHALL reflect what is actually on disk. An app whose desktop entry the user deleted by hand SHALL NOT be listed.

#### Scenario: Desktop entry deleted manually
- **WHEN** the user deletes an app's desktop entry with `rm`
- **THEN** that app no longer appears in `winkle list`
