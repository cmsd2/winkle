# Spec Delta

## MODIFIED Requirements

### Requirement: Remove command
`winkle remove <id>` SHALL delete the app's desktop entry and icons, and remove the app from the dock favourites if pinned. The app SHALL disappear from the app grid and search without logging out.

#### Scenario: Remove an installed app
- **WHEN** the user runs `winkle remove github-com`
- **THEN** the app no longer appears in the app grid, search, dock favourites or `winkle list`

#### Scenario: Unknown id
- **WHEN** the user runs `winkle remove does-not-exist`
- **THEN** the command exits with a non-zero status and an error saying no such app is installed

### Requirement: Refuse to purge a profile in use
If `--purge` is given while the app's isolated profile is in use by a running Chromium process, the command SHALL fail before changing anything and ask the user to close the app first.

#### Scenario: Purge while running
- **WHEN** the isolated app is open and the user runs `winkle remove app-hey-com --purge`
- **THEN** nothing is removed and the error asks the user to close the app

### Requirement: Uninstall from the app's menu
Each installed app SHALL have an "Uninstall" action in its right-click menu. Choosing it SHALL show a confirmation dialog naming the app with these choices:
- Cancel
- Uninstall
- Uninstall and Delete Data (isolated apps only)

Confirming SHALL behave like `winkle remove` (with `--purge` for the delete-data choice) and SHALL show a desktop notification with the result. Cancelling SHALL change nothing.

#### Scenario: Uninstall via right-click
- **WHEN** the user right-clicks the app, chooses Uninstall, then confirms
- **THEN** the app is removed and a notification says it was uninstalled

#### Scenario: Cancel
- **WHEN** the user chooses Uninstall, then Cancel
- **THEN** the app remains installed and unchanged

#### Scenario: Failure is reported
- **WHEN** Uninstall and Delete Data is chosen while the app is still open
- **THEN** a notification or dialog explains that the app must be closed first, and nothing is removed

## ADDED Requirements

### Requirement: Only winkle's own apps
Removal SHALL only act on apps winkle installed, and SHALL only delete files winkle created.

#### Scenario: Non-winkle entry with a matching name
- **WHEN** the id matches a desktop entry that winkle did not create
- **THEN** the command reports no such winkle app and deletes nothing

#### Scenario: App installed under the old hermit name
- **WHEN** the user runs `winkle remove github-com` and only `hermit-github-com.desktop` exists
- **THEN** the command reports that no winkle app `github-com` is installed and deletes nothing

## REMOVED Requirements

### Requirement: Only hermit's own apps
**Reason**: The tool is renamed to winkle; the rule now protects everything winkle did not create, including apps installed under the old hermit name.
**Migration**: Replaced by "Only winkle's own apps". Remove apps installed under the old name with the old `hermit` binary before uninstalling it.
