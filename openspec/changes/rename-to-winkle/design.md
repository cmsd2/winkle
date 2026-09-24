# Design

## Context

See proposal.md for why. The name `hermit` appears in four kinds of place:

1. **Identity:** the crate and binary name, clap's `name`, and `notify-send --app-name`.
2. **On-disk contract** (what installed apps and their data depend on):
   - desktop file prefix `hermit-<id>.desktop`
   - icon name `hermit-<id>`
   - entry keys `X-Hermit-Id`, `X-Hermit-Url`, `X-Hermit-Profile` and `X-Hermit-Version`
   - profile root `~/snap/chromium/common/hermit/`
   - the Uninstall action's `Exec` (absolute path to `~/.cargo/bin/hermit`)
3. **Test and dev plumbing:** the `HERMIT_BROWSER` override, the integration-test stubs, and the spike's `hermit-spike` scratch dir.
4. **Docs and hosting:** README, roadmap, CLAUDE.md, spike notes, the GitHub repo `cmsd2/hermit`, and the local folder `~/Development/hermit`.

Three apps are installed under the old name: GitHub, HEY (isolated) and Spotify.

## Goals / Non-Goals

**Goals:**
- After the change, no user-visible or on-disk `hermit` name remains in anything winkle writes or prints.
- A clean switch-over path for existing apps, documented in the README.
- The crate is ready for crates.io (`cargo publish --dry-run` succeeds).

**Non-Goals:**
- Migrating existing apps; the user decided against it.
- Recognising `hermit-*` entries in any way (listing, warning, removing).
- Publishing to crates.io.
- Rewriting history: the archived `mvp` change and past commit messages keep the old name.

## Decisions

### 1. One name constant, not scattered literals
Introduce `paths::APP_NAME = "winkle"` and derive every other name from it:
- the desktop file prefix
- the icon name
- the profile root folder
- the notification app name
- the `X-<Name>-*` key prefix, via a `KEY_PREFIX = "X-Winkle-"` constant in `entry`
- the browser override variable (`WINKLE_BROWSER`)

**Why:** the rename becomes one edit plus a sweep, and tests can assert against the constant. **Alternative:** plain search-and-replace, which is simpler now but leaves the next rename just as scattered.

### 2. No migration; old apps are simply invisible to winkle
winkle only reads `winkle-*.desktop` files carrying `X-Winkle-Id`, so `hermit-*` entries fall outside both the file-prefix filter and the key check. No extra code is needed; the new spec scenarios pin the behaviour with tests. The README switch-over section documents:
1. remove each old app with the **old** binary (`hermit remove <id>`), while it still exists
2. for isolated apps, optionally keep logins with `mv ~/snap/chromium/common/hermit/<id> ~/snap/chromium/common/winkle/<id>` before reinstalling
3. reinstall with `winkle install … [--isolated]`
4. `cargo uninstall hermit`

The order matters: each old app's Uninstall action runs `~/.cargo/bin/hermit`, so uninstalling the binary first leaves those actions broken.

### 3. Crate metadata for crates.io
- Add `readme`, `keywords` (`gnome`, `pwa`, `web-app`, `desktop`, `chromium`) and `categories` (`command-line-utilities`).
- Add an `include`/`exclude` list so the published crate carries the source, README, LICENSE and test fixtures, but not `openspec/`, `spikes/`, `.claude/` or the docs.
- Verify with `cargo publish --dry-run` and `cargo package --list`.

### 4. Hosting renames last, after the code is green
1. Rename the GitHub repo with `gh repo rename winkle` and update `origin`.
2. Move the local folder to `~/Development/winkle`.

The folder move changes the session's working directory, so it is the final task, done after the last commit and push.

## Risks / Trade-offs

- **[The user forgets to remove old apps before uninstalling `hermit`]** → The orphaned entries still launch fine; only their Uninstall action fails. Recovery: delete `~/.local/share/applications/hermit-*.desktop` and the `hermit-*` icons by hand. The README says so.
- **[The GitHub rename breaks links]** → GitHub redirects the old repo URL, and git remotes keep working through the redirect. `origin` is still updated explicitly.
- **[The crates.io name is taken before publishing]** → Out of scope here, but the dry run confirms nothing else blocks publishing. The name was free on 2026-09-24.

## Migration Plan

For the developer (this machine), after the change lands:
1. `hermit remove` for each of GitHub, HEY and Spotify.
2. `mv` HEY's profile to keep its logins.
3. `cargo install --path .`, then reinstall the three apps with `winkle`.
4. `cargo uninstall hermit`.

The tasks walk through this together with the user.

**Rollback:** reinstall `hermit` from the pre-rename commit; the two names never share files.
