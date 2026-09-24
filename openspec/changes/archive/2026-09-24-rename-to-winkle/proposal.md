# Proposal

## Why

The name `hermit` is taken on crates.io, where it belongs to the Hermit unikernel, a well-known Rust project. The `hermit` command also clashes with Cash App's Hermit, a widely used tool that installs a binary of the same name. To publish on crates.io, and to avoid a command that collides with, or is confused with, other tools, the project needs a new name. **winkle** was chosen: it's short, and "to winkle something out" means to extract it, which is what this does to a site and the browser. It's free on crates.io and in Ubuntu's archive.

## What Changes

- **BREAKING:**
  - the crate, binary and command are renamed from `hermit` to `winkle`: `winkle install`, `winkle list`, `winkle remove`
  - the files and data the tool writes are renamed:
    - desktop entries: `winkle-<id>.desktop`
    - icons: `winkle-<id>`
    - entry keys: `X-Winkle-*`
    - isolated profiles: `~/snap/chromium/common/winkle/<id>/`
  - the test override variable `HERMIT_BROWSER` becomes `WINKLE_BROWSER`
- **BREAKING:**
  - apps installed under the old name are **not** migrated: `winkle` doesn't see or manage them
  - the README documents the switch-over instead:
    1. remove old apps with the old binary
    2. optionally move an isolated profile to keep its logins
    3. reinstall with `winkle`
    4. uninstall the old binary
- The GitHub repository is renamed `cmsd2/hermit` → `cmsd2/winkle` (GitHub redirects the old URL), and the local folder `~/Development/hermit` → `~/Development/winkle`.
- Crate metadata is readied for crates.io (keywords, categories, readme) and checked with `cargo publish --dry-run`. Publishing itself is not part of this change.
- Current docs are updated: README, roadmap, CLAUDE.md, spike notes and the probe script. The archived `mvp` change keeps the old name as history.

Behaviour is otherwise unchanged: same metadata resolution, window matching, profile modes and uninstall flow.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `app-install`: the `install` command and its messages use the `winkle` name. "Files hermit did not create" becomes "files winkle did not create". Dry-run output names winkle's files.
- `app-list`: lists apps installed by winkle (not hermit), under `winkle list`.
- `app-removal`: `winkle remove`, the Uninstall action runs `winkle`, and only winkle's own apps and files are touched. The requirement "Only hermit's own apps" is renamed.
- `site-metadata`: command examples use `winkle install`, and the explanation of why redirects are not followed refers to winkle.

## Impact

- **Code:** every module that builds names (`paths`, `entry`, `browser`, `desktop`, the commands and the CLI definition), plus integration tests and stubs.
- **User data:** existing `hermit-*` apps keep working, but are orphaned from the tool until removed with the old binary. The user currently has three: GitHub, HEY (isolated) and Spotify.
- **External:** the GitHub repo rename (redirects keep old links and clones working). No crates.io publish yet.
- **Dependencies:** unchanged.
