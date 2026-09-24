# Spec Delta

## MODIFIED Requirements

### Requirement: Windows group under the app's own icon
Windows opened from the app's main launcher entry SHALL be associated with that app, not with Chromium. The dock SHALL show the app's own icon for them as soon as the window appears, with no busy cursor lingering after it, including when the launch is handed to an already-running Chromium. The app SHALL be pinnable to the dock. Windows opened from shortcut actions are exempt (see Shortcut actions).

#### Scenario: Dock shows the app, not Chromium
- **WHEN** the app is launched while a normal Chromium window is also open
- **THEN** the dock shows separate icons for the app and for Chromium, and the app window sits under the app's icon

#### Scenario: Pinned app activates its window
- **WHEN** the app is pinned to the dock, running, and the user presses its Super+number shortcut
- **THEN** the existing app window is focused rather than a new one opened

#### Scenario: Icon appears immediately for a shared-profile app
- **WHEN** a shared-profile app is launched from the app grid while the main Chromium is already running
- **THEN** its dock icon appears together with its window, and no busy cursor remains once the window is shown

### Requirement: Profile modes
By default an app SHALL use the user's main Chromium profile (shared cookies and logins). With `--isolated`, the app SHALL use its own profile, not shared with the main browser or other apps, which persists between launches. An isolated app SHALL open directly to its site, without Chromium's first-run screens, even on the very first launch of a new profile.

#### Scenario: Shared profile reuses logins
- **WHEN** the user is logged in to GitHub in Chromium and installs `github.com` without `--isolated`
- **THEN** the app opens already logged in

#### Scenario: Isolated profile starts clean and persists
- **WHEN** the user installs with `--isolated`, logs in inside the app, and closes and relaunches it
- **THEN** the app is still logged in, and the main Chromium profile is unaffected

#### Scenario: First launch of an isolated app skips Chromium's first-run screens
- **WHEN** an app installed with `--isolated` is launched for the first time, with a brand-new profile
- **THEN** its window shows the site, not a Chromium welcome or terms-of-service screen

### Requirement: No silent overwrite
Installing an id that is already installed SHALL fail unless `--force` is given. With `--force`, the existing app SHALL be replaced and its isolated profile, if any, kept. Whichever fields of the entry changed, the replacement SHALL take effect without the user logging out for launches from Activities search. The app grid MAY keep launching the previous entry until the next login; that is a GNOME Shell limitation, which the README documents.

#### Scenario: Duplicate install
- **WHEN** `github-com` is installed and the user runs `winkle install github.com` again
- **THEN** the command fails, says the app exists, and suggests `--force`

#### Scenario: Forced reinstall keeps logins
- **WHEN** an isolated app is reinstalled with `--force`
- **THEN** its profile directory, and so its logins, are unchanged

#### Scenario: Forced reinstall takes effect in search without logging out
- **WHEN** an app is reinstalled with `--force` and only a launch setting that GNOME doesn't compare changes (such as `StartupNotify`)
- **THEN** launching the app from Activities search a few seconds later uses the new entry, without logging out
