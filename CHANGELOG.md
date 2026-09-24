# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-09-24

First release. winkle turns websites into apps that feel native on GNOME. It needs
Ubuntu (GNOME on Wayland) with the Chromium snap.

### Added

- `winkle install <url>` installs a website as an app. It gets its own entry in the app
  grid and Activities search, its own dock icon, and a Chromium app window with no
  browser toolbar.
  - The name, icon, start page and right-click shortcuts come from the site's web app
    manifest, falling back to the page's own metadata. Sites without a usable icon get
    a generated letter icon.
  - By default an app shares your main Chromium profile, so you're already logged in.
    `--isolated` gives it a profile of its own instead.
  - `--name`, `--icon` and `--id` override what the site declares. `--dry-run` shows
    what would be installed without changing anything, and `--force` replaces an
    installed app while keeping its isolated profile.
- `winkle list` shows installed apps as a table, or as JSON with `--json`.
- `winkle remove <id>` uninstalls an app and unpins it from the dock. `--purge` also
  deletes an isolated app's logins and site data.
- Every app has an **Uninstall** action in its right-click menu, which asks for
  confirmation first.

[0.1.0]: https://github.com/cmsd2/winkle/releases/tag/v0.1.0
