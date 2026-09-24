# app-list Specification

## Purpose

Shows which websites are installed as hermit apps, with enough detail to manage them, in both human-readable and machine-readable form.

## Requirements

### Requirement: List installed apps
`hermit list` SHALL print one line per installed hermit app, sorted by id, showing id, name, launch URL and profile mode (`shared` or `isolated`). Apps not installed by hermit SHALL NOT be listed.

#### Scenario: Two apps installed
- **WHEN** `github-com` (shared) and `app-hey-com` (isolated) are installed
- **THEN** `hermit list` prints `app-hey-com` then `github-com`, each with its name, URL and profile mode

#### Scenario: Other apps are ignored
- **WHEN** Chromium's own installed web apps or other desktop entries exist
- **THEN** they do not appear in `hermit list`

### Requirement: Empty state
When no apps are installed, `hermit list` SHALL print a short message saying so and exit with status 0.

#### Scenario: Nothing installed
- **WHEN** no hermit apps exist
- **THEN** a message says no apps are installed, and the exit status is 0

### Requirement: Machine-readable output
`hermit list --json` SHALL print a JSON array of objects with at least `id`, `name`, `url`, `profile` and `shortcuts` (an array of `{name, url}`). With no apps it SHALL print `[]`.

#### Scenario: JSON output
- **WHEN** one app is installed and the user runs `hermit list --json`
- **THEN** the output is a valid JSON array containing one object with those fields

### Requirement: Reflects hand edits
The list SHALL reflect what is actually on disk. An app whose desktop entry the user deleted by hand SHALL NOT be listed.

#### Scenario: Desktop entry deleted manually
- **WHEN** the user deletes an app's desktop entry with `rm`
- **THEN** that app no longer appears in `hermit list`
