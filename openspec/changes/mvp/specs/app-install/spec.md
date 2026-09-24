# Spec Delta

## Purpose

Turns a website into an installed desktop app that GNOME treats like any other: it appears in the app grid and search, launches in its own window, groups under its own dock icon, and exposes shortcuts on right-click.

## ADDED Requirements

### Requirement: Install command
`hermit install <url>` SHALL install the site as an app using the metadata described in the `site-metadata` capability, and print the app's id and name. Options:
- `--id <id>`
- `--name <text>`
- `--icon <path-or-url>`
- `--isolated`
- `--force`
- `--dry-run`

#### Scenario: Successful install
- **WHEN** the user runs `hermit install github.com`
- **THEN** the command exits with status 0 and prints the new app's id and name

### Requirement: App identifier
Each app SHALL have an id of lowercase letters, digits and hyphens. By default it SHALL be derived from the launch URL's host (see `site-metadata`), with a leading `www.` removed and dots replaced by hyphens (`app.hey.com` → `app-hey-com`). `--id` SHALL override it, and invalid ids SHALL be rejected.

#### Scenario: Default id
- **WHEN** the user installs `https://www.github.com/`
- **THEN** the app id is `github-com`

#### Scenario: Invalid custom id
- **WHEN** the user passes `--id "My App"`
- **THEN** the command fails with an error describing the allowed characters and installs nothing

### Requirement: No silent overwrite
Installing an id that is already installed SHALL fail unless `--force` is given. With `--force`, the existing app SHALL be replaced and its isolated profile, if any, kept.

#### Scenario: Duplicate install
- **WHEN** `github-com` is installed and the user runs `hermit install github.com` again
- **THEN** the command fails, says the app exists, and suggests `--force`

#### Scenario: Forced reinstall keeps logins
- **WHEN** an isolated app is reinstalled with `--force`
- **THEN** its profile directory, and so its logins, are unchanged

### Requirement: Appears as a native app
After install, and without logging out, the app SHALL appear in the GNOME app grid and in Activities search under its name. Search SHALL also match the site's host name. The app SHALL show its chosen icon.

#### Scenario: Found by name and host
- **WHEN** the app "GitHub" for `github.com` has been installed
- **THEN** typing "GitHub" or "github.com" in Activities search shows it with its icon

### Requirement: Launches as an app window
Launching the app SHALL open the launch URL in a Chromium app window with no tabs, address bar or browser toolbar.

#### Scenario: Launch from the app grid
- **WHEN** the user clicks the app in the app grid
- **THEN** a window opens showing the launch URL with no browser UI

### Requirement: Windows group under the app's own icon
Windows opened from the app's main launcher entry SHALL be associated with that app, not with Chromium. The dock SHALL show the app's own icon for them, and the app SHALL be pinnable to the dock. Windows opened from shortcut actions are exempt (see Shortcut actions).

#### Scenario: Dock shows the app, not Chromium
- **WHEN** the app is launched while a normal Chromium window is also open
- **THEN** the dock shows separate icons for the app and for Chromium, and the app window sits under the app's icon

#### Scenario: Pinned app activates its window
- **WHEN** the app is pinned to the dock, running, and the user presses its Super+number shortcut
- **THEN** the existing app window is focused rather than a new one opened

### Requirement: Profile modes
By default an app SHALL use the user's main Chromium profile (shared cookies and logins). With `--isolated`, the app SHALL use its own profile, not shared with the main browser or other apps, which persists between launches.

#### Scenario: Shared profile reuses logins
- **WHEN** the user is logged in to GitHub in Chromium and installs `github.com` without `--isolated`
- **THEN** the app opens already logged in

#### Scenario: Isolated profile starts clean and persists
- **WHEN** the user installs with `--isolated`, logs in inside the app, and closes and relaunches it
- **THEN** the app is still logged in, and the main Chromium profile is unaffected

### Requirement: Shortcut actions
Each collected manifest shortcut SHALL appear as an action in the app icon's right-click menu. It SHALL open that shortcut's URL as a Chromium app window (no browser UI) using the same profile as the app. Shortcut windows are not required to group under the app's dock icon, and MAY appear as a separate dock entry.

#### Scenario: Right-click shortcut
- **WHEN** the manifest defines a "New issue" shortcut to `/issues/new`
- **THEN** right-clicking the app shows "New issue", which opens that URL in an app window with the app's profile

### Requirement: Stays within the user's home
Install SHALL write only under the user's data directories: `XDG_DATA_HOME`, defaulting to `~/.local/share`, and, for isolated profiles, the Chromium snap's per-user data area. It SHALL NOT require root and SHALL NOT modify files that hermit did not create.

#### Scenario: Pre-existing unrelated desktop entry
- **WHEN** a non-hermit desktop entry exists with the file name hermit would use
- **THEN** install fails without modifying that file, even with `--force`

### Requirement: Dry run
`--dry-run` SHALL resolve metadata and print the app id, name, launch URL, chosen icon source, profile mode, shortcuts and the files it would create, without writing anything.

#### Scenario: Dry run writes nothing
- **WHEN** the user runs `hermit install github.com --dry-run`
- **THEN** the planned files and resolved metadata are printed and no files are created
