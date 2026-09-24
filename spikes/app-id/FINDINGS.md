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

## Dock icon appears ~10 s late for shared-profile apps

Symptom: launching a shared-profile app (GitHub, Spotify) opens a usable window
at once, but the busy cursor spins and the app's dock icon only appears ~10 s
later. Isolated apps (HEY) show their icon immediately.

Cause, from gnome-shell 50.1-0ubuntu1 source (`src/shell-app.c`):

- A launch with a start-up sequence puts the app in `SHELL_APP_STATE_STARTING`.
- `shell_app_sync_running_state()` does nothing while an app is STARTING, so a
  window arriving for it doesn't make it RUNNING. Only completion of the start-up
  sequence does (`_shell_app_handle_startup_sequence`): the launched program
  claiming the activation token, or the sequence timing out.
- An isolated app is a fresh Chromium process that claims the token, so its
  sequence completes at once. A shared-profile launch is handed to the running
  browser ("Opening in existing browser session"), which doesn't claim the token,
  so the app stays STARTING (spinner, no dock icon) until the timeout.

Matching itself is fine: `probe-forwarded.sh` shows the forwarded window's
first request after creation is `set_app_id("chrome-github.com__-Default")`, and
gnome-shell matches WM_CLASS against `StartupWMClass` first.

Ubuntu patch (LP:2007652) in `get_app_from_window_wmclass`: for windows from the
Chromium snap, an entry matched by `StartupWMClass` is only accepted if its Exec
executable is `/snap/bin/chromium` or `/snap/bin/chromium_*`. **winkle must never
wrap the launch command** (e.g. a `winkle launch` shim); the window would fall
back to Chromium's icon.

Test (fresh desktop IDs, direct `/snap/bin/chromium` Exec, main Chromium running):

| Entry | StartupNotify | Result |
|---|---|---|
| Spike C | true | spinner, icon after ~10 s |
| Spike D | false | no spinner, icon immediately |

**Fix:** `StartupNotify=false` in winkle's entries.

Testing gotcha: editing an already-installed entry in place gave inconsistent
results (gnome-shell kept launching a stale copy). Use fresh desktop IDs for A/B tests.

## Why editing an installed entry sometimes has no effect

gnome-shell 50.1 `src/shell-app-system.c`: when desktop entries change,
`ShellAppCache` reloads them after a 5 s rate limit (`DEFAULT_TIMEOUT_SECONDS`) and
emits `changed`. `installed_changed()` then drops apps that `app_is_stale()`
reports as stale. `app_is_stale()` compares only these fields:
- should_show, filename, executable, commandline
- name, description, display name, icon

It does **not** compare `StartupNotify`, `StartupWMClass`, `Actions`, `Keywords`
and so on. An edit that changes only those leaves the old `GDesktopAppInfo` on the
`ShellApp`, and launches keep using it until the user logs out and back in. (The
`StartupWMClass` → app lookup is rebuilt from the cache, so window matching does
see new classes; launching does not.)

Observed: after `winkle install --force` changed only `StartupNotify`, GitHub and
Spotify kept the ~10 s delay. HEY, whose command line also changed
(`--no-first-run`), picked up the new entry. Reinstalling GitHub under a new desktop
ID got a fresh `ShellApp`.

Consequence: rewriting an entry in place isn't enough when only those fields
change. Either the old entry must disappear from the cache first (delete, wait more
than 5 s, rewrite), or a compared field must change.
