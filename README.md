# winkle

Install websites as apps that feel native on GNOME. Each one gets its own icon in the app
grid, search and dock, opens in its own window with no browser toolbar, and can be
uninstalled from its right-click menu.

Built for Ubuntu 26.04 (GNOME on Wayland) with the Chromium snap.

## Install

```bash
cargo install winkle
```

To build from a clone of this repository instead, run `cargo install --path .`.

Either way this puts `winkle` in `~/.cargo/bin`. Each app's Uninstall action runs winkle by that path,
so install it there before installing apps. If you move it later, reinstall your apps with
`--force`.

## Usage

See what would be installed, without changing anything:

```bash
winkle install github.com --dry-run
```

winkle reads the site's name, icon and shortcuts, preferably from its web app manifest.
Install it:

```bash
winkle install github.com
```

The app then appears in the app grid and in search, under its name or its web address.
Pin it to the dock like any other app.

Give an app its own browser profile, separate from your main Chromium logins:

```bash
winkle install app.hey.com --isolated
```

Choose the name, id or icon yourself (`--icon` also takes a local file):

```bash
winkle install mail.google.com --name Gmail --id gmail --icon https://www.gstatic.com/images/branding/product/1x/gmail_2020q4_512dp.png
```

List installed apps, as a table or as JSON:

```bash
winkle list
```

```bash
winkle list --json
```

Uninstall an app. You can also right-click it and choose **Uninstall**:

```bash
winkle remove github-com
```

## Profiles and your data

- **Shared (default).** The app uses your main Chromium profile, so you're already logged in.
  Removing the app never touches that profile.
- **Isolated (`--isolated`).** The app has its own profile in
  `~/snap/chromium/common/winkle/<id>/`. Removing the app **keeps** that profile, so
  reinstalling it keeps your logins. To delete it too:

  ```bash
  winkle remove app-hey-com --purge
  ```

  Or choose **Uninstall and Delete Data** from the app's Uninstall action. winkle refuses to
  delete a profile while the app is open.

## What winkle writes

Everything is in your home directory:

- `~/.local/share/applications/winkle-<id>.desktop`: the app. It is also winkle's
  only record of it, so deleting this file uninstalls the app.
- `~/.local/share/icons/hicolor/256x256/apps/winkle-<id>.png`, plus
  `scalable/apps/winkle-<id>.svg` for SVG and generated icons.
- `~/snap/chromium/common/winkle/<id>/` for isolated profiles.

## Known limitations

- **Shortcut windows get their own dock icon.** Right-click shortcuts (such as HEY's "Write an
  email") open windows that don't group under the app's icon: Chromium gives every URL
  path its own window ID. See [the app-id spike findings](https://github.com/cmsd2/winkle/blob/main/spikes/app-id/FINDINGS.md).
- **Sites behind a login may give the login page's icon.** winkle fetches sites without
  your logins. For example, `mail.google.com` redirects to Google's sign-in page, whose
  small favicon winkle then uses. Pass `--icon` to choose a better one. The app itself still
  opens at the address you gave.
- **Links don't open in installed apps.** A `github.com` link clicked elsewhere opens in
  the browser, not in the GitHub app. That's on the roadmap.
- **DRM sites (Spotify, Netflix) need Chromium restarted once after it's first installed.**
  Chromium downloads its Widevine DRM module shortly after its first start, but on Linux it
  only loads the module at startup. Until the next restart, Play does nothing on those
  sites, in a tab or in a winkle app. Restart Chromium once (`chrome://restart`) and they work.
  See [the DRM spike findings](https://github.com/cmsd2/winkle/blob/main/spikes/eme/FINDINGS.md).
- **After `winkle install --force`, the app grid may launch the old version until you log in
  again.** Activities search picks up the change within a few seconds, but GNOME Shell's app
  grid holds on to the entry it loaded at login. Log out and back in to refresh it. Fresh
  installs aren't affected.
- **Apps installed before 0.1.0 should be reinstalled.** Earlier development builds wrote
  entries whose dock icon appeared about 10 seconds late, and isolated apps opened to Chromium's
  first-run screen. Run `winkle install <url> --force` (adding `--isolated` where it applied)
  to refresh them, then log out and back in once (see above). Isolated apps keep their logins.
- **winkle can't close apps.** Snap confinement stops other programs from signalling
  Chromium, so close an isolated app yourself before purging its data.
- **Chromium snap on GNOME only** for now. See [the roadmap](https://github.com/cmsd2/winkle/blob/main/docs/roadmap.md) for what's next.

## Development

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

The tests never touch your desktop. Integration tests run winkle with a temporary home
and stub `gsettings`, `zenity`, `notify-send` and browser scripts.

Design and specs live in `openspec/`.

## Licence

winkle is free software: you can redistribute it and/or modify it under the terms of the
GNU General Public License as published by the Free Software Foundation, either version 3
of the License, or (at your option) any later version (`GPL-3.0-or-later`). See
[LICENSE](LICENSE).
