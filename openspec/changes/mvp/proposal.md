# Proposal

## Why

Running a site as `chromium --app=URL` is easy, but GNOME doesn't treat the result as a real app: no proper icon, windows not grouped under their dock icon, invisible to search, and no way to uninstall. hermit's first release should make a website installable, launchable and removable so that it looks and behaves like any other app on this desktop (Ubuntu 26.04, GNOME 50 on Wayland, Chromium snap, arm64).

## What Changes

- New `hermit` command-line tool, written in Rust.
- `hermit install <url>` reads the site's name, icons, start URL and shortcuts, preferring its web app manifest and falling back to HTML metadata. It then writes a GNOME desktop entry and hi-res icon so the app appears in the app grid, search and dock.
- Installed apps launch in a Chromium app window (no browser UI). Their windows group under their own dock icon, and the app can be pinned and reached with Super+1–9.
- Per-app choice of browser profile: **shared** (the main Chromium profile, already logged in; the default) or **isolated** (a separate profile just for this app).
- Manifest shortcuts become right-click actions on the app icon.
- `hermit list` shows installed apps.
- `hermit remove <id>` uninstalls an app. Each app also gets an **Uninstall** right-click action that confirms with a native dialog, so no terminal is needed.
- A first spike that measures which window ID (Wayland app_id) Chromium assigns to app windows, since dock grouping depends on it.

Out of scope for the MVP: GNOME search-provider discovery, a curated catalogue, routing links to installed apps, activate-or-launch hotkeys, unread badges, refreshing metadata after install, `mailto:`/protocol handlers, and any GUI beyond the uninstall confirmation.

## Capabilities

### New Capabilities
- `site-metadata`: working out an app's identity from a URL: name, icons, start URL, scope, theme colour and shortcuts, from the web app manifest with HTML fallbacks.
- `app-install`: turning a URL into an installed app: desktop entry, icons, launch command, window grouping, profile mode and shortcut actions.
- `app-list`: listing installed apps and their key properties.
- `app-removal`: uninstalling an app from the CLI or from its own right-click menu, including what data is and isn't deleted.

### Modified Capabilities
<!-- none: greenfield project -->

## Roadmap

This change is Milestone 1 of the roadmap in [docs/roadmap.md](../../../docs/roadmap.md). Later milestones will be separate changes.

## Impact

- New Rust crate at the repo root (binary `hermit`). Expected dependencies: an argument parser, an HTTP client, an HTML parser, JSON, URL handling and image processing.
- Writes only inside the user's home directory: `~/.local/share/applications`, `~/.local/share/icons/hicolor`, and, for isolated profiles, `~/snap/chromium/common/hermit/`.
- Runtime dependencies on the host: the Chromium snap, `zenity` for the uninstall dialog, and optionally `update-desktop-database`.
- Nothing system-wide, and no root access needed.
