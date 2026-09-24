# winkle

Make websites feel like native desktop apps on Ubuntu (GNOME, Wayland) across the
whole lifecycle: discovery, install, use, uninstall.

## Conventions

- Rust for the real implementation. Spikes (throwaway experiments to answer a
  question) may use whatever is quickest (shell, Python); keep them under `spikes/`.
- Before committing or pushing, all of these must pass with zero errors and zero warnings:
  `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- Tests must never touch the real desktop: integration tests run the binary with a temp
  `HOME`/`XDG_DATA_HOME` and stub `gsettings`, `zenity`, `notify-send` and browser on `PATH`.

## Target environment

- Ubuntu 26.04, GNOME Shell 50 on Wayland, arm64 (Snapdragon X Elite).
- Chromium is the snap (`/snap/bin/chromium`): it cannot read hidden dirs in `$HOME`,
  so per-app profiles must live under `~/snap/chromium/common/`.

## Design outline

- Each web app = a `.desktop` file in `~/.local/share/applications` + hi-res icons in
  `~/.local/share/icons/hicolor`, launching `chromium --app=URL`.
- `StartupWMClass` must match the Wayland app_id Chromium actually assigns, or the
  dock won't group windows with the icon. Verify empirically (first spike).
- `Exec` must start `/snap/bin/chromium` directly, with no wrapper: Ubuntu's gnome-shell
  only matches Chromium-snap windows to entries whose executable is `/snap/bin/chromium`
  (LP:2007652). Entries use `StartupNotify=false`, or launches handed to a running
  Chromium leave the dock icon missing for ~10 s. See `spikes/app-id/FINDINGS.md`.
- Metadata from the site's web app manifest (name, icons, theme colour, start_url,
  scope, shortcuts → desktop Actions), falling back to apple-touch-icon / favicon.
- Profile per app is a choice: shared main profile (already logged in) or isolated.
- Uninstall via an "Uninstall" desktop Action on each app's own `.desktop` file;
  removes desktop file, icons, handler registrations, optionally the profile.
- Later: GNOME search provider for discovery, a default-browser shim that routes
  links by domain to installed apps, activate-or-launch for hotkeys.
