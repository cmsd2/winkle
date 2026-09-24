# Spec Delta

## MODIFIED Requirements

### Requirement: Install command
`winkle install <url>` SHALL install the site as an app using the metadata described in the `site-metadata` capability, and print the app's id and name. Options:
- `--id <id>`
- `--name <text>`
- `--icon <path-or-url>`
- `--isolated`
- `--force`
- `--dry-run`

#### Scenario: Successful install
- **WHEN** the user runs `winkle install github.com`
- **THEN** the command exits with status 0 and prints the new app's id and name

### Requirement: No silent overwrite
Installing an id that is already installed SHALL fail unless `--force` is given. With `--force`, the existing app SHALL be replaced and its isolated profile, if any, kept.

#### Scenario: Duplicate install
- **WHEN** `github-com` is installed and the user runs `winkle install github.com` again
- **THEN** the command fails, says the app exists, and suggests `--force`

#### Scenario: Forced reinstall keeps logins
- **WHEN** an isolated app is reinstalled with `--force`
- **THEN** its profile directory, and so its logins, are unchanged

### Requirement: Stays within the user's home
Install SHALL write only under the user's data directories: `XDG_DATA_HOME`, defaulting to `~/.local/share`, and, for isolated profiles, the Chromium snap's per-user data area. It SHALL NOT require root and SHALL NOT modify files that winkle did not create.

#### Scenario: Pre-existing unrelated desktop entry
- **WHEN** a non-winkle desktop entry exists with the file name winkle would use
- **THEN** install fails without modifying that file, even with `--force`

### Requirement: Dry run
`--dry-run` SHALL resolve metadata and print the app id, name, launch URL, chosen icon source, profile mode, shortcuts and the files it would create, without writing anything.

#### Scenario: Dry run writes nothing
- **WHEN** the user runs `winkle install github.com --dry-run`
- **THEN** the planned files and resolved metadata are printed and no files are created
