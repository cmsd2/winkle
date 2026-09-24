# Tasks

## 1. Dock icon delay

- [x] 1.1 Write `StartupNotify=false` in generated entries (`src/entry.rs`), with a one-line comment pointing at `spikes/app-id/FINDINGS.md`. Update the unit tests, and make the integration test `install_writes_a_valid_entry_and_icon` assert `StartupNotify=false`. Verify: `cargo test` passes, and `desktop-file-validate` still passes on generated entries.

## 2. First-run screen for isolated apps

- [x] 2.1 Add `--no-first-run` to `launch_argv` for `Profile::Isolated`, before `--app=`. Update the unit tests `isolated_argv` and `shortcut_argv_uses_the_app_profile`, and the integration test `isolated_install_gets_its_own_profile`. Verify: `cargo test` passes; shared-profile argv is unchanged.

## 3. Exec constraint

- [x] 3.1 Add a doc comment on `launch_argv` explaining the Ubuntu LP:2007652 matching rule, and a unit test asserting argv[0] is the browser path for both profile modes. Add a line to CLAUDE.md's design outline: "`Exec` must start `/snap/bin/chromium` directly; no wrapper". Verify: the test passes and CLAUDE.md contains the rule.

- [x] 3.2 Add `--winkle-entry=<8 hex>` to every launch command line (main and shortcuts), after the profile arguments and before `--app=`. The value is an FNV-1a 64-bit hash of the entry as written with that argument omitted. Tests:
  - the hash changes when any single field changes (StartupNotify value, name, URL, a shortcut)
  - it's identical for identical entries
  - `argv[0]` is still the browser and the last argument is still `--app=…`
  - the generated file passes `desktop-file-validate`
  Verify: `cargo test` passes.

## 4. Packaging and README

- [x] 4.1 Add `/CHANGELOG.md` to `include` in Cargo.toml. Verify: `cargo package --list --allow-dirty` still succeeds, and lists CHANGELOG.md when a temporary one is present (then removed).
- [x] 4.2 README:
  - the install section leads with `cargo install winkle`, keeping `cargo install --path .` for building from source
  - the `spikes/…` and `docs/…` mentions become links to `https://github.com/cmsd2/winkle/blob/main/…`
  - a known-limitations note says apps installed before 0.1.0 should be reinstalled with `--force`

  Verify: every GitHub link in the README returns HTTP 200 after push, and `grep -nE '`(spikes|docs)/' README.md` finds no unlinked paths.

- [x] 4.3 Check the README's pre-0.1.0 note still holds with the cache-busting argument: `--force` alone refreshes apps, with no logout. Keep the note if so; otherwise fix it. Verify: the note matches the behaviour confirmed in task 6.4.

## 5. Checks, commit, push

- [x] 5.1 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass with zero warnings; commit and push, and CI passes on both runners. Verify: the commands exit 0 and `gh run view` for the pushed commit reports success on `ubuntu-latest` and `ubuntu-24.04-arm`.

## 6. Refresh installed apps and confirm on the desktop (with the user)

- [x] 6.1 `cargo install --path .`, then `winkle install --force` for GitHub, Spotify and HEY (`--isolated`). Verify: each entry has `StartupNotify=false`, and HEY's `Exec` includes `--no-first-run`.
- [x] 6.2 The user launches GitHub and Spotify from the app grid while Chromium is running, and confirms the dock icon appears with the window and there's no lingering spinner. Verify: recorded in `docs/acceptance/prerelease-fixes.md`.
- [x] 6.3 Install a new isolated app with a fresh profile (e.g. `winkle install example.com --isolated --id first-run-check`). The user confirms it opens straight to the site; then remove it with `--purge`. Verify: recorded in the same acceptance file, and the test app is gone from `winkle list`.
- [x] 6.4 With the new build: `winkle install --force` for Spotify, and move GitHub back from the temporary `github` ID to `github-com` (`winkle remove github`, then `winkle install github.com`). Then change only a GNOME-uncompared field on one app (e.g. temporarily install Spotify again with `--force` after toggling a test build's `StartupNotify`), or, simpler, confirm with the user that Spotify's icon now appears immediately without logging out. Verify: recorded in `docs/acceptance/prerelease-fixes.md`.
