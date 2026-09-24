# Proposal

## Why

Using winkle day to day and dry-running the first release turned up four things to fix before 0.1.0 goes out on crates.io, where every published version is permanent:

- **Dock icon delay:** shared-profile apps show a busy cursor and no dock icon for about 10 seconds after launch (root cause in `spikes/app-id/FINDINGS.md`).
- **Terms screen:** isolated apps open to Chromium's first-run terms screen, which in this unbranded build shows placeholder text.
- **Undocumented constraint:** a constraint found while debugging (Ubuntu's window-matching patch) isn't written down, so a future change could break dock grouping without noticing.
- **Packaging gaps:** the published crate would lack the changelog, and its README tells people to build from source and points at files that aren't in the package.

## What Changes

- Desktop entries winkle writes use `StartupNotify=false`. For a launch handed to an already-running Chromium, GNOME otherwise keeps the app "starting" until a ~10 s timeout, and meanwhile shows no dock icon.
- Isolated apps launch with `--no-first-run`, so a new profile opens straight to the site.
- The design records that an entry's `Exec` must start `/snap/bin/chromium` directly. Ubuntu's gnome-shell only matches Chromium-snap windows to entries whose executable is `/snap/bin/chromium` (LP:2007652), so wrapper commands or a `winkle launch` shim are ruled out.
- Packaging and README:
  - `CHANGELOG.md` is added to the crate's `include` list.
  - The README's install section leads with `cargo install winkle`.
  - References to `spikes/` and `docs/` become links to GitHub, since those folders aren't in the package.
- Every entry's command line carries `--winkle-entry=<hash>`, a hash of the rest of the entry, which Chromium ignores. gnome-shell only notices an edited entry if certain fields change, and the command line is one of them. So `winkle install --force` takes effect without logging out, whatever changed (see `spikes/app-id/FINDINGS.md`, "Why editing an installed entry sometimes has no effect").
- Already-installed apps keep their old entries until reinstalled with `winkle install --force`. The README's known limitations say so for anyone who installed before 0.1.0 (in practice, only the developer's machine).

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `app-install`: three requirements change.
  - "Windows group under the app's own icon" now requires the icon to appear as soon as the window does, with no lingering busy cursor, including when the launch is handed to a running Chromium.
  - "Profile modes" now requires isolated apps to open directly to the site without Chromium's first-run screens.
  - "No silent overwrite" now requires a `--force` replacement to take effect in GNOME without logging out.

## Impact

- **Code:** `src/entry.rs` (the `StartupNotify` value and the entry hash), `src/browser.rs` (the launch arguments), and their unit and integration tests.
- **Packaging and docs:** `Cargo.toml` (`include`), `README.md`.
- **Users:** existing entries need `winkle install --force` to pick up both fixes. No data changes, and isolated profiles are kept.
- **Dependencies:** none.
