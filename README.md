# hermit

Install websites as apps that feel native on GNOME. Each one gets its own icon in the app
grid, search and dock, opens in its own window with no browser toolbar, and can be
uninstalled from its right-click menu.

Built for Ubuntu 26.04 (GNOME on Wayland) with the Chromium snap.

## Install

```bash
cargo install --path .
```

This puts `hermit` in `~/.cargo/bin`. Each app's Uninstall action runs hermit by that path,
so install it there before installing apps. If you move it later, reinstall your apps with
`--force`.

## Usage

See what would be installed, without changing anything:

```bash
hermit install github.com --dry-run
```

hermit reads the site's name, icon and shortcuts, preferably from its web app manifest.
Install it:

```bash
hermit install github.com
```

The app then appears in the app grid and in search, under its name or its web address.
Pin it to the dock like any other app.

Give an app its own browser profile, separate from your main Chromium logins:

```bash
hermit install app.hey.com --isolated
```

Choose the name, id or icon yourself (`--icon` also takes a local file):

```bash
hermit install mail.google.com --name Gmail --id gmail --icon https://www.gstatic.com/images/branding/product/1x/gmail_2020q4_512dp.png
```

List installed apps, as a table or as JSON:

```bash
hermit list
```

```bash
hermit list --json
```

Uninstall an app. You can also right-click it and choose **Uninstall**:

```bash
hermit remove github-com
```

## Profiles and your data

- **Shared (default).** The app uses your main Chromium profile, so you're already logged in.
  Removing the app never touches that profile.
- **Isolated (`--isolated`).** The app has its own profile in
  `~/snap/chromium/common/hermit/<id>/`. Removing the app **keeps** that profile, so
  reinstalling it keeps your logins. To delete it too:

  ```bash
  hermit remove app-hey-com --purge
  ```

  Or choose **Uninstall and Delete Data** from the app's Uninstall action. hermit refuses to
  delete a profile while the app is open.

## What hermit writes

Everything is in your home directory:

- `~/.local/share/applications/hermit-<id>.desktop`: the app. It is also hermit's
  only record of it, so deleting this file uninstalls the app.
- `~/.local/share/icons/hicolor/256x256/apps/hermit-<id>.png`, plus
  `scalable/apps/hermit-<id>.svg` for SVG and generated icons.
- `~/snap/chromium/common/hermit/<id>/` for isolated profiles.

## Known limitations

- **Shortcut windows get their own dock icon.** Right-click shortcuts (such as HEY's "Write an
  email") open windows that don't group under the app's icon: Chromium gives every URL
  path its own window ID. See `spikes/app-id/FINDINGS.md`.
- **Sites behind a login may give the login page's icon.** hermit fetches sites without
  your logins. For example, `mail.google.com` redirects to Google's sign-in page, whose
  small favicon hermit then uses. Pass `--icon` to choose a better one. The app itself still
  opens at the address you gave.
- **Links don't open in installed apps.** A `github.com` link clicked elsewhere opens in
  the browser, not in the GitHub app. That's on the roadmap.
- **DRM sites (Spotify, Netflix) need Chromium restarted once after it's first installed.**
  Chromium downloads its Widevine DRM module shortly after its first start, but on Linux it
  only loads the module at startup. Until the next restart, Play does nothing on those
  sites, in a tab or in a hermit app. Restart Chromium once (`chrome://restart`) and they work.
  See `spikes/eme/FINDINGS.md`.
- **hermit can't close apps.** Snap confinement stops other programs from signalling
  Chromium, so close an isolated app yourself before purging its data.
- **Chromium snap on GNOME only** for now. See `docs/roadmap.md` for what's next.

## Development

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

The tests never touch your desktop. Integration tests run hermit with a temporary home
and stub `gsettings`, `zenity`, `notify-send` and browser scripts.

Design and specs live in `openspec/`.
