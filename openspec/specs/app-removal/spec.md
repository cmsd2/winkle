# app-removal Specification

## Purpose

Uninstalls a hermit app cleanly, from the command line or from the app's own right-click menu, with a clear and safe rule for what data is deleted.

## Requirements

### Requirement: Remove command
`hermit remove <id>` SHALL delete the app's desktop entry and icons, and remove the app from the dock favourites if pinned. The app SHALL disappear from the app grid and search without logging out.

#### Scenario: Remove an installed app
- **WHEN** the user runs `hermit remove github-com`
- **THEN** the app no longer appears in the app grid, search, dock favourites or `hermit list`

#### Scenario: Unknown id
- **WHEN** the user runs `hermit remove does-not-exist`
- **THEN** the command exits with a non-zero status and an error saying no such app is installed

### Requirement: Data retention
Removing an app SHALL keep its isolated profile unless `--purge` is given. With `--purge` the isolated profile SHALL be deleted. The shared main Chromium profile SHALL never be modified by removal, with or without `--purge`.

#### Scenario: Default remove keeps data
- **WHEN** an isolated app is removed without `--purge` and later reinstalled with the same id and `--isolated`
- **THEN** the reinstalled app is still logged in

#### Scenario: Purge deletes isolated data
- **WHEN** an isolated app is removed with `--purge`
- **THEN** its profile directory no longer exists

#### Scenario: Purge on a shared app
- **WHEN** a shared-profile app is removed with `--purge`
- **THEN** the app is removed, the main Chromium profile is untouched, and a note says there was no app-specific data to delete

### Requirement: Refuse to purge a profile in use
If `--purge` is given while the app's isolated profile is in use by a running Chromium process, the command SHALL fail before changing anything and ask the user to close the app first.

#### Scenario: Purge while running
- **WHEN** the isolated app is open and the user runs `hermit remove app-hey-com --purge`
- **THEN** nothing is removed and the error asks the user to close the app

### Requirement: Only hermit's own apps
Removal SHALL only act on apps hermit installed, and SHALL only delete files hermit created.

#### Scenario: Non-hermit entry with a matching name
- **WHEN** the id matches a desktop entry that hermit did not create
- **THEN** the command reports no such hermit app and deletes nothing

### Requirement: Uninstall from the app's menu
Each installed app SHALL have an "Uninstall" action in its right-click menu. Choosing it SHALL show a confirmation dialog naming the app with these choices:
- Cancel
- Uninstall
- Uninstall and Delete Data (isolated apps only)

Confirming SHALL behave like `hermit remove` (with `--purge` for the delete-data choice) and SHALL show a desktop notification with the result. Cancelling SHALL change nothing.

#### Scenario: Uninstall via right-click
- **WHEN** the user right-clicks the app, chooses Uninstall, then confirms
- **THEN** the app is removed and a notification says it was uninstalled

#### Scenario: Cancel
- **WHEN** the user chooses Uninstall, then Cancel
- **THEN** the app remains installed and unchanged

#### Scenario: Failure is reported
- **WHEN** Uninstall and Delete Data is chosen while the app is still open
- **THEN** a notification or dialog explains that the app must be closed first, and nothing is removed
