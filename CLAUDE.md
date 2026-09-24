# hermit

Make websites feel like native desktop apps on Ubuntu (GNOME, Wayland) across the
whole lifecycle: discovery, install, use, uninstall.

## Conventions

- Rust for the real implementation. Spikes (throwaway experiments to answer a
  question) may use whatever is quickest (shell, Python); keep them under `spikes/`.

## Target environment

- Ubuntu 26.04, GNOME Shell 50 on Wayland, arm64 (Snapdragon X Elite).
- Chromium is the snap (`/snap/bin/chromium`): it cannot read hidden dirs in `$HOME`,
  so per-app profiles must live under `~/snap/chromium/common/`.

## Design outline

- Each web app = a `.desktop` file in `~/.local/share/applications` + hi-res icons in
  `~/.local/share/icons/hicolor`, launching `chromium --app=URL`.
- `StartupWMClass` must match the Wayland app_id Chromium actually assigns, or the
  dock won't group windows with the icon. Verify empirically (first spike).
- Metadata from the site's web app manifest (name, icons, theme colour, start_url,
  scope, shortcuts → desktop Actions), falling back to apple-touch-icon / favicon.
- Profile per app is a choice: shared main profile (already logged in) or isolated.
- Uninstall via an "Uninstall" desktop Action on each app's own `.desktop` file;
  removes desktop file, icons, handler registrations, optionally the profile.
- Later: GNOME search provider for discovery, a default-browser shim that routes
  links by domain to installed apps, activate-or-launch for hotkeys.
