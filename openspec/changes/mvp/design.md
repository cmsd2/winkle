# Design

## Context

Greenfield Rust project; see proposal.md for motivation and the specs for required behaviour. What the host looks like today:

- **Desktop:** Ubuntu 26.04, GNOME Shell 50 on Wayland, arm64.
- **Browser:**
  - Chromium 153 as a snap (`/snap/bin/chromium`); it is also the default browser.
  - The snap can read and write non-hidden paths under `$HOME` (home interface), but not dotfiles or dot-directories. Its own writable per-user area is `~/snap/chromium/common/`.
- **Desktop tools available:** `zenity`, `notify-send`, `gio`, `gtk-launch`, `update-desktop-database`, `gsettings`.
- **Rust:** 1.98 via rustup; `~/.cargo/bin` is on the login-session PATH through `~/.profile`.
- **Precedent:** the Claude desktop app's desktop entry on this machine sets `StartupWMClass` to the Wayland app_id so the dock groups its windows. That is the same mechanism hermit depends on.

## Goals / Non-Goals

**Goals:**
- A single static-ish binary with no daemon; every command runs and exits.
- Installed apps keep working even if the `hermit` binary is later removed. Only Uninstall-from-menu depends on it.
- Everything testable without touching the real home directory.

**Non-Goals:**
- Supporting browsers other than the Chromium snap, or desktops other than GNOME, in this change. The code should keep the browser behind a small seam, but no second implementation.
- Parsing arbitrary desktop entries. hermit only reads the entries it wrote.

## Decisions

### 1. The desktop entry is the registry
Each app is `$XDG_DATA_HOME/applications/hermit-<id>.desktop`. Its hermit-specific data lives in `X-Hermit-*` keys:
- `X-Hermit-Id`
- `X-Hermit-Url`
- `X-Hermit-Profile` (`shared` or `isolated`)
- `X-Hermit-Version`, the entry's format version

`list` scans for files with `X-Hermit-Id`, and `remove` refuses files without it.

- *Why:* there's no second store to drift out of sync; hand deletion "just works" (see the `app-list` spec).
- *Alternative:* a TOML/JSON registry in `$XDG_DATA_HOME/hermit/`. Rejected for the MVP because of the duplicate source of truth. It can be revisited if we need data that doesn't fit in the entry.

### 2. `Exec` calls Chromium directly
Launch lines are `/snap/bin/chromium (--profile-directory=Default | --user-data-dir=…) --app=<url>` rather than going through `hermit launch <id>`.

- *Why:* apps survive the hermit binary being moved or uninstalled, and there's no extra process at launch.
- *Cost:* changing launch behaviour needs a rewrite of the entries. That's acceptable: `install --force` (and later `refresh`) regenerates them, and `X-Hermit-Version` lets a future version detect old entries.
- *Exception:* the Uninstall action's `Exec` uses the **absolute path** of the hermit binary at install time (from `std::env::current_exe`), so it doesn't depend on the PATH of GNOME's launcher.

### 3. Window matching: derive Chromium's app_id
GNOME links a window to an app when the window's Wayland app_id equals the desktop file ID or its `StartupWMClass`. The spike (`spikes/app-id/FINDINGS.md`) measured Chromium 153 on Wayland:
- `--class` is **ignored**, so we can't choose the app_id ourselves.
- The app_id is derived from the `--app` URL: `chrome-<host>_<path with every "/" replaced by "_">-<profile directory>`, with the query string dropped. For example, `https://github.com/notifications` gives `chrome-github.com__notifications-Default`.

So hermit computes this value from the launch URL and writes it to `StartupWMClass`.
- **Profile directory:** shared apps pass `--profile-directory=Default` explicitly, so the suffix doesn't depend on which profile Chromium last used. Isolated apps get a fresh user-data-dir whose only profile is `Default`.
- **Shortcuts:** each shortcut URL has its own path and so its own app_id. Shortcut windows therefore don't group under the app's icon. The spec accepts this for the MVP; a per-app redirect-page launcher was considered and deferred (see roadmap, milestone 2).
- **Not measured:** the case of a launch request forwarded to an already-running Chromium. The spike couldn't observe it because the running process doesn't have `WAYLAND_DEBUG` set. The manual grouping check (task 1.3) covers it by launching a shared-profile entry while the main browser is running.
- **Alternatives rejected:** `--class` (ignored on Wayland); a hidden extra `.desktop` file per shortcut (adds a second dock icon while open, and more files to manage).

### 4. Profiles
- **Shared:** no `--user-data-dir` (Chromium's own), plus `--profile-directory=Default` (see decision 3).
- **Isolated:** `--user-data-dir=$HOME/snap/chromium/common/hermit/<id>`. It must be under the snap's area because the snap can't use hidden directories such as `~/.local/share`.
- **"In use"** for the `--purge` guard means the `SingletonLock` symlink exists in that directory and points at a live PID on this host.

### 5. Metadata fetching
- **HTTP client:** blocking `reqwest` with rustls.
- **Limits:** 10 s timeout per request, up to 5 redirects, body caps (2 MB for HTML, 1 MB for manifests, 5 MB for icons).
- **User-Agent:** Chromium's desktop UA string, because some sites serve different markup, or no manifest, to unknown clients.
- **HTML parsing:** `scraper`, reading only `<head>`-relevant elements.
- **Manifest:** `serde_json` into a lenient struct in which every field is optional and unknown fields are ignored.
- **Icon ranking:** implements the order in the `site-metadata` spec. The `sizes` attribute is only a hint, so ranking uses real decoded dimensions.

### 6. Icon installation
- **Raster:** decoded with the `image` crate (PNG, ICO, JPEG, WebP), converted to RGBA, scaled with Lanczos3 to 256×256 (upscaled if smaller), and written to `$XDG_DATA_HOME/icons/hicolor/256x256/apps/hermit-<id>.png`. Non-square images are padded onto a transparent square rather than stretched.
- **SVG:** copied to `hicolor/scalable/apps/hermit-<id>.svg` and also rasterised to the 256 bucket via `resvg`.
- **Placeholder:** a coloured rounded square in the theme colour (or a neutral grey) with the app name's first letter, drawn with `tiny-skia` plus a bundled font. This is the single most visible "cheap vs native" signal, so it's worth a proper look.
- **Desktop entry:** references `Icon=hermit-<id>` (a theme name, not a path) so GNOME picks the best size.
- **Refresh:** after writing, touch the `hicolor` directory's mtime so GNOME Shell notices new icons without a re-login.

### 7. Desktop entry contents
- **Keys:**
  - `Type=Application`
  - `Name`
  - `Comment` ("Web app for <host>")
  - `Icon`
  - `Exec`
  - `StartupWMClass` (derived as in decision 3)
  - `StartupNotify=true`
  - `Categories=Network;WebBrowser;`, then trimmed to `Network;` in case `WebBrowser` makes GNOME offer it as a browser
  - `Keywords` (host plus name words)
  - `Actions` (shortcut ids plus `Uninstall`)
  - the `X-Hermit-*` keys
- **Writing:** hand-written with correct escaping for the desktop entry format (`\s \n \t \\` in values, plus `Exec` field-code quoting). Values come from the network, so every one goes through the escaper and `%` is doubled in `Exec`.
- **Atomic replace:** written to a temp file in the same directory, then `rename`d.
- **Registration:** `update-desktop-database` is run if present. It's only strictly needed for `MimeType`, which the MVP doesn't use, but it's harmless and prepares for milestone 3.

### 8. Uninstall action
`[Desktop Action Uninstall]` runs `Exec=<abs-hermit> remove <id> --interactive`. `--interactive` shows a `zenity --question` dialog (with `--extra-button` for "Uninstall and Delete Data" on isolated apps), then reports the outcome with `notify-send`. On failure it shows `zenity --error`.

- *Why zenity:* it's already installed and gives native GTK dialogs with no GTK dependency in our binary.
- *Alternative:* the `gtk4-rs` crate. Rejected as heavy for one dialog.

### 9. Dock favourites
On remove, hermit reads `org.gnome.shell favorite-apps` via `gsettings`, drops `hermit-<id>.desktop` if present, and writes the list back only if it changed. If `gsettings` is unavailable, this step is skipped with a warning.

### 10. Crate layout
A single binary crate with modules:
- `cli` (clap derive)
- `metadata` (fetch, manifest, html, icon ranking)
- `icons` (decode, normalise, placeholder)
- `entry` (desktop entry model, writer, reader)
- `browser` (the Chromium-snap command line and profile paths; the seam for other browsers later)
- `desktop` (favourites, notifications, dialogs, cache nudges)
- `paths` (XDG resolution)

Errors use `anyhow` at the edges and `thiserror` inside the library parts.

### 11. Testing
- **Unit tests:** metadata parsing and icon ranking, against HTML/manifest fixtures in `tests/fixtures/`.
- **HTTP tests:** a local server (`wiremock` or `httpmock`) serves fixture sites so redirects, 404 manifests and timeouts are covered.
- **Isolation:** integration tests run the binary with `XDG_DATA_HOME` and `HOME` pointed at a temp dir and `HERMIT_BROWSER` set to a stub script that records its arguments. Tests never touch the real desktop.
- **Manual acceptance:** the dock and grouping scenarios in `app-install` are checked on the real desktop, using a checklist in tasks.md.

## Risks / Trade-offs

- **[Chromium changes its app_id derivation in a snap update, or it differs for characters we haven't measured, such as ports, percent-encoding or trailing slashes]** → The derivation lives in one function with tests pinned to measured values; `spikes/app-id/probe.sh` re-measures quickly. `X-Hermit-Version` lets a later `refresh` or `doctor` rewrite entries, and the roadmap's `hermit doctor` will check that grouping still works.
- **[Sites block non-browser fetches (Cloudflare challenges, login walls)]** → Fall back to HTML meta, then host name and placeholder icon; `--name`/`--icon` overrides; the warning says what happened.
- **[Login-walled sites return the login page's metadata]** → Usually the same brand, so acceptable; `--force` with overrides fixes the rest.
- **[Shared-profile apps opening while Chromium is closed start a full browser process]** → That's Chromium's behaviour and harmless; note it in the README.
- **[The absolute hermit path in the Uninstall action goes stale if the binary moves]** → The action fails visibly (GNOME reports a launch error). `hermit install --force` or the roadmap's `doctor` repairs it.
- **[Content from the network ends up in desktop entry fields that GNOME executes]** → Strict escaping and `%` doubling in `Exec`. URLs are validated with the `url` crate and re-serialised before use. Ids are restricted to `[a-z0-9-]`.
- **[Placeholder icons look cheap]** → Worth the extra effort in decision 6; sites without any icon are rare.

## Migration Plan

Not applicable: first release, no existing installs. Users can undo everything with `hermit remove --purge` for each app, or by deleting `hermit-*` files under `~/.local/share/applications` and `~/.local/share/icons/hicolor/*/apps/`, plus `~/snap/chromium/common/hermit/`.

## Open Questions

- Should `Categories` include `WebBrowser` or not? Test whether GNOME then lists the app as a browser choice in Settings → Default Apps, and pick whichever doesn't. This doesn't affect the specs.
- Whether to set `SingleMainWindow=true` to suppress GNOME's "New Window" menu item. Decide after using the MVP for a bit.
