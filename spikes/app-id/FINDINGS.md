# Spike: Chromium window app_id

Chromium snap 153.0.8010.36, GNOME 50 on Wayland, arm64. Measured with
`probe.sh` (`WAYLAND_DEBUG=client`, throwaway `--user-data-dir`), reading
`xdg_toplevel.set_app_id`. Chromium runs as a native Wayland client
(`--ozone-platform=wayland`).

| Case | Command | app_id |
|---|---|---|
| a. plain `--app` | `--app=https://github.com/` | `chrome-github.com__-Default` |
| b. `--class` | `--app=https://github.com/ --class=winkle-test` | `chrome-github.com__-Default` (`--class` ignored) |
| b2. `--class`, deep link | `--app=https://github.com/notifications --class=winkle-test` | `chrome-github.com__notifications-Default` |
| c1. deep link | `--app=https://github.com/notifications` | `chrome-github.com__notifications-Default` |
| c2. other path, same host | `--app=https://github.com/pulls` | `chrome-github.com__pulls-Default` |
| d. shortcut-style URL with query | `--app=https://github.com/issues/new?x=1` | `chrome-github.com__issues_new-Default` |
| e. forwarded to running Chromium | | not observable with the probe; see task 1.3 |

## So far

- **Strategy A (`--class`) does not work on Wayland**: Chromium ignores it.
- **Strategy B (derived) works but is per-URL**: the app_id is
  `chrome-<host>_<path with / → _>-<profile dir>`, and the query string is dropped. Every
  distinct launch path is a different app_id, so shortcut windows (other paths)
  will **not** group under the app's icon via `StartupWMClass`, which is single-valued.
- The trailing `-Default` is the profile *directory* name inside the user-data-dir,
  so shared and isolated apps both end in `-Default` (unless the user's main
  profile is another directory).

## Side finding: winkle cannot kill snap Chromium

`kill`/`pkill` against the snap's processes fails with EPERM, even outside the
Claude Code sandbox; it looks like the snap's AppArmor confinement. `chrome://quit`
passed on the command line doesn't quit it either. Consequences:
- Tooling can't close app windows. The probe now waits for the user to close them.
- winkle must never rely on signalling Chromium, e.g. for `remove --purge` of a
  running app. The design already only *reads* `SingletonLock`, which is fine.

## Decision

- Use strategy B: derive `StartupWMClass` from the launch URL. Shared apps pass
  `--profile-directory=Default` explicitly. The main profile here is `Default`
  (the only profile in `~/snap/chromium/common/chromium/Local State`).
- Shortcut windows won't group under the app's icon; dropped from the MVP spec.
- Case (e), a request forwarded to a running Chromium, wasn't observable with the
  probe; it's checked by hand in task 1.3.

## Manual grouping check (task 1.3)

With the main Chromium already running (so the launch was handed to it, i.e.
case e), a throwaway desktop entry with
`Exec=/snap/bin/chromium --profile-directory=Default --app=https://github.com/`
and `StartupWMClass=chrome-github.com__-Default` was launched with `gtk-launch`.
The window appeared under the entry's own (star) icon in the dock, separate from
Chromium's. **Grouping works for shared-profile apps, including forwarded launches.**
