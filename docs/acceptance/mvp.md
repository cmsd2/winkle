# MVP acceptance (task 9.2)

Run on the real desktop: Ubuntu 26.04, GNOME 50 on Wayland, Chromium snap 153,
ThinkPad T14s Gen 6 (arm64), hermit installed with `cargo install --path .`.

Setup:

```bash
hermit install github.com
hermit install app.hey.com --isolated
```

| # | Check | Result | Notes |
|---|---|---|---|
| 1 | Both appear in the app grid with their own icons | | |
| 2 | Activities search finds them by name ("GitHub", "HEY") | | |
| 3 | Activities search finds them by host ("github.com", "app.hey.com") | | |
| 4 | Both launch with no tabs, address bar or toolbar | | |
| 5 | Dock: each app window sits under its own icon, separate from Chromium's, with a normal Chromium window also open | | |
| 6 | Pin GitHub; with it running, its Super+number focuses the existing window | | |
| 7 | HEY right-click → "Write an email" opens an app window (may get its own dock icon: known limitation) | | |
| 8 | HEY (isolated): log in, close, relaunch → still logged in; main Chromium not logged in to HEY by this | | |
| 9 | GitHub right-click → Uninstall → Uninstall: app gone from grid, search and dock; notification shown | | |
| 10 | HEY right-click → Uninstall shows "Uninstall and Delete Data"; choosing it with HEY open shows an error and removes nothing | | |
| 11 | Close HEY, repeat 10: app and `~/snap/chromium/common/hermit/app-hey-com` are gone | | |
