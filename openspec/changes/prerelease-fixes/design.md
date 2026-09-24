# Design

## Context

See proposal.md for the why. The findings behind the two behaviour fixes are in `spikes/app-id/FINDINGS.md` ("Dock icon appears ~10 s late"). They come from reading gnome-shell 50.1-0ubuntu1 source and an A/B test on fresh desktop IDs.

Current code:
- `src/entry.rs` writes `StartupNotify=true`.
- `src/browser.rs::launch_argv` builds `[browser, --profile-directory=Default | --user-data-dir=…, --app=URL]`.

## Goals / Non-Goals

**Goals:**
- Dock icons appear immediately for every app.
- Isolated apps never show Chromium's first-run screens.
- The constraints behind these choices are written down where the next change will see them.
- The crate's README and contents make sense on crates.io.

**Non-Goals:**
- Getting Chromium to claim GNOME's activation token for launches handed to a running browser. That would keep the busy cursor as launch feedback, but it's Chromium's behaviour, not something winkle can change.
- Automatically refreshing entries that are already installed. `winkle install --force` does it, and the roadmap's `winkle refresh` or `doctor` will cover it later.

## Decisions

### 1. `StartupNotify=false` for all entries
Measured: with `true`, a launch handed to a running Chromium leaves gnome-shell's app in STARTING until the start-up sequence times out (~10 s). `shell_app_sync_running_state()` ignores windows while an app is STARTING. With `false` there's no sequence, and the window moves the app straight to RUNNING.

- **Isolated apps as well:** a fresh isolated launch does claim the token, but launching an isolated app that's already running is handed to its own browser process, which has the same problem. One rule for every entry is simpler and never wrong.
- **What's lost:** the busy cursor during a genuinely cold start. Windows appear within a second or two, so that's acceptable.
- **Alternative rejected:** setting `false` only for shared apps. It gives inconsistent behaviour, and isolated re-launches still hit the delay.

### 2. `--no-first-run` for isolated profiles only
- **Isolated:** each isolated app gets a brand-new `--user-data-dir`, so Chromium shows its first-run flow. In the unbranded snap build, that's a terms screen with placeholder text.
- **Shared:** the shared profile has long since finished its first run. Passing the flag there would be a no-op, and it would change the command line that's handed to the running browser for no benefit.

The flag goes before `--app=` in `launch_argv`, so `Exec` stays `/snap/bin/chromium …` (decision 3).

- **Alternative rejected:** also `--no-default-browser-check`. The first-run flow is what showed the terms screen, and that flag only suppresses an infobar in normal browser windows, which app windows don't have.

### 3. `Exec` must start `/snap/bin/chromium` directly
Ubuntu's gnome-shell patch (LP:2007652, in `get_app_from_window_wmclass`) only accepts a `StartupWMClass` match for a Chromium-snap window when the entry's executable is `/snap/bin/chromium` or `/snap/bin/chromium_*`. With any other executable, the window falls back to Chromium's own icon. Observed during the spike: a logging wrapper in `Exec` made GitHub's window join Chromium's dock icon.

This is recorded in three places so it isn't lost:
- a doc comment on `launch_argv`
- a unit test asserting `argv[0] == DEFAULT_BROWSER` for every profile mode
- CLAUDE.md's design outline

A `winkle launch` shim on the roadmap would need another approach.

### 4. Packaging and README
- **Changelog:** add `/CHANGELOG.md` to `include`. It doesn't exist until the first release creates it, but a missing include path is fine for `cargo package`, and `cargo package --list` verifies it once present.
- **Install section:** lead with `cargo install winkle`, and keep `cargo install --path .` as the from-source option.
- **Links:** turn the `spikes/…` and `docs/…` mentions into links to `https://github.com/cmsd2/winkle/blob/main/…`. The existing relative `[LICENSE](LICENSE)` link stays, since LICENSE ships in the crate.

## Risks / Trade-offs

- **[A future Chromium passes the activation token through, making the busy cursor useful again]** → Harmless: `false` still works. Revisit if cold-start feedback is missed.
- **[`--no-first-run` also skips first-run imports and prompts that someone might want]** → Isolated apps are meant to start clean, so nothing is lost.
- **[Existing installs keep the old behaviour until reinstalled]** → The README's known limitations note it. The only such installs are on the developer's machine, and the tasks refresh them.

## Migration Plan

After release, reinstall existing apps with `winkle install <url> --force [--isolated]`. Isolated profiles and logins are kept (existing `--force` behaviour).
