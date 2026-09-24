# Tasks

## 1. Spike: Chromium window app_id

- [x] 1.1 Write `spikes/app-id/probe.sh`. It launches `/snap/bin/chromium --app=<url>` with a throwaway `--user-data-dir` under `~/snap/chromium/common/hermit-spike/` and `WAYLAND_DEBUG=client`, captures stderr, extracts the `xdg_toplevel.set_app_id(...)` value, then closes the window and deletes the throwaway profile. Verify: running it prints one app_id for `https://github.com/`.
- [x] 1.2 Use the probe to measure the app_id for each of these cases:
  - (a) plain `--app`
  - (b) `--app` plus `--class=hermit-test`
  - (c) two different URLs on the same host
  - (d) a shortcut URL
  - (e) shared profile while a normal Chromium window is already open (the request is forwarded to the running process)

  Verify: `spikes/app-id/FINDINGS.md` has a table of case → app_id and says which strategy (A `--class` or B derived) works for shared and isolated profiles.
- [x] 1.3 Confirm grouping by hand, which also covers case (e). While the main Chromium is running, write a throwaway desktop entry with `--profile-directory=Default --app=https://github.com/` and `StartupWMClass=chrome-github.com__-Default`. Launch it and check the dock shows the entry's own icon, separate from Chromium's. Then remove the entry. Verify: result recorded in FINDINGS.md.
- [x] 1.4 Update design.md decision 3 with the chosen strategy and delete the rejected one. Verify: design.md no longer lists both as candidates.

## 2. Project scaffolding

- [x] 2.1 `cargo init --name hermit` at the repo root, with a `.gitignore` for `target/`. Add dependencies: clap (derive), reqwest (blocking, rustls-tls, no default features), scraper, serde, serde_json, url, image, resvg, tiny-skia, anyhow, thiserror. Add dev-dependencies: tempfile, httpmock, assert_cmd, predicates. Verify: `cargo build` succeeds on arm64.
- [x] 2.2 Create the module skeleton from design decision 10 (`cli`, `metadata`, `icons`, `entry`, `browser`, `desktop`, `paths`), with clap subcommands `install`, `list` and `remove` and all their flags stubbed. Verify: `cargo run -- --help` and `cargo run -- install --help` list every option in the specs.
- [x] 2.3 Implement `paths`: honour `XDG_DATA_HOME` (default `~/.local/share`), the snap profile root `~/snap/chromium/common/hermit/`, and a `HERMIT_BROWSER` override for tests. Verify: unit tests cover the default and override paths.

## 3. Site metadata

- [x] 3.1 URL intake: accept a bare host (default to https), reject non-http(s) schemes, normalise. Verify: unit tests for `hey.com`, `https://x/`, `file:///…` and `ftp://…`.
- [x] 3.2 Fetcher with the design's timeouts, redirect limit, body caps and Chromium desktop User-Agent, returning the final URL after redirects. Verify: httpmock tests for redirect chains, a timeout and an oversized body.
- [x] 3.3 HTML extraction: manifest link, `application-name`, `og:site_name`, `<title>`, and apple-touch-icon and icon links with `sizes`, all resolved against the final URL. Verify: fixture tests for GitHub-like, HEY-like and bare pages.
- [x] 3.4 Manifest parsing: lenient struct; name/short_name, icons (src, sizes, type, purpose), start_url, theme_color, shortcuts. Invalid JSON or a 404 gives a warning, not an error. Verify: fixture tests, including a broken manifest.
- [x] 3.5 Launch URL rule: the entered URL, not the post-redirect one; a deep link wins; the root uses a `start_url` same-origin with the fetched page; a cross-origin `start_url` is ignored. Shortcut filter: named, same-origin, at most 10. Verify: unit tests for each scenario in the `site-metadata` spec.
- [x] 3.6 Icon ranking and fetching per the spec order, using decoded dimensions and preferring SVG, then the largest square. Verify: tests for the "several sizes", "maskable only as fallback" and "no usable icon" scenarios.
- [x] 3.7 Overrides: `--name` and `--icon` (local path or URL). If the fetch fails without `--name`, error with a hint. Verify: tests for the two "site unreachable" scenarios.

## 4. Icons

- [ ] 4.1 Raster normalisation: decode PNG/ICO/JPEG/WebP, pad non-square images to square, and Lanczos-scale to 256×256 PNG. Verify: unit tests with a 16×16 ICO, a 512×512 PNG and a 300×200 JPEG check the output is 256×256 and not stretched.
- [ ] 4.2 SVG handling: copy the SVG to scalable and rasterise to 256 via resvg. Verify: a unit test checks both files exist and the PNG decodes.
- [ ] 4.3 Placeholder generator: rounded square in the theme colour or neutral grey, with a centred first letter from a bundled font. Verify: a snapshot test renders "G" on #24292f, and the PNG is committed under `tests/snapshots/` for eyeballing.

## 5. Desktop entry

- [ ] 5.1 Entry model and writer: all keys from design decision 7, `X-Hermit-*` keys, value escaping, `Exec` quoting with `%` doubling, and atomic write via temp file plus rename. Verify: unit tests for escaping hostile names (newlines, `%`, quotes, `;`), and `desktop-file-validate` passes on generated output (install `desktop-file-utils` if missing).
- [ ] 5.2 `browser` module: build the Chromium command line for shared (`--profile-directory=Default`) and isolated profiles and for shortcut URLs, and derive `StartupWMClass` per design decision 3. Verify: unit tests assert exact argument vectors, and assert the derived app_id equals every value measured in FINDINGS.md.
- [ ] 5.3 Entry reader for hermit's own files (reads the `X-Hermit-*` keys, ignores files without `X-Hermit-Id`). Verify: round-trip test write → read gives the same model.

## 6. Install command

- [ ] 6.1 Id derivation from the launch URL's host (strip `www.`, dots → hyphens), `--id` validation to `[a-z0-9-]`, duplicate detection, `--force` replacement that keeps any isolated profile, and refusal to touch non-hermit files with the same name. Verify: integration tests (temp `XDG_DATA_HOME`, stub browser) for each `app-install` id and overwrite scenario.
- [ ] 6.2 Wire up install: metadata → icons → entry → icon-cache nudge → `update-desktop-database` if present; print id and name. Verify: an integration test against an httpmock site produces the entry and icon files with expected contents.
- [ ] 6.3 `--dry-run` prints the resolved metadata and planned files without writing. Verify: an integration test asserts the output and that the temp data dir stays empty.
- [ ] 6.4 Uninstall action in the generated entry points at the absolute `current_exe()` path. Verify: an integration test checks the `[Desktop Action Uninstall]` `Exec` line.

## 7. List command

- [ ] 7.1 `hermit list`, sorted by id, showing id, name, URL and profile, with the empty-state message. Verify: integration tests for two apps, no apps, and a non-hermit entry being ignored.
- [ ] 7.2 `hermit list --json`, including shortcuts. Verify: an integration test parses the output with serde_json and checks the fields; with no apps it outputs `[]`.

## 8. Remove command

- [ ] 8.1 `hermit remove <id>`: delete the entry and icon files, remove from `favorite-apps` via gsettings (skip with a warning if unavailable), and error on an unknown or non-hermit id. Verify: integration tests using a stub `gsettings` on PATH that records calls.
- [ ] 8.2 `--purge`: delete the isolated profile. On a shared app, print a "no app-specific data" note and don't touch the Chromium profile. Refuse if `SingletonLock` points at a live PID, before any change. Verify: integration tests for each scenario in the `app-removal` spec, faking the lock with a symlink to the test's own PID.
- [ ] 8.3 `--interactive`: zenity question dialog (with the extra delete-data button for isolated apps), `notify-send` on success, `zenity --error` on failure; Cancel changes nothing. Verify: integration tests with stub `zenity` and `notify-send` scripts returning each exit code.

## 9. Docs and end-to-end checks

- [ ] 9.1 README.md: install hermit (`cargo install --path .`), usage for install/list/remove, profile modes, what data remove keeps, and the known limitations from design.md. Verify: every command in the README runs as written against a real site.
- [ ] 9.2 Manual acceptance on the real desktop, recorded as a checklist in `docs/acceptance/mvp.md`:
  - install `github.com` (shared) and `app.hey.com` (isolated)
  - both appear in the grid and in search by name and host
  - both launch without browser UI
  - dock grouping stays separate from Chromium
  - pin, then Super+number focuses the existing window
  - a shortcut action works
  - isolated login persists
  - Uninstall from right-click, both the keep-data and delete-data paths

  Verify: every item ticked, with notes on anything surprising.
- [ ] 9.3 `cargo clippy -- -D warnings` and `cargo test` pass. Verify: both commands exit 0.
