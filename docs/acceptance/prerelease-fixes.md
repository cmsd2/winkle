# Pre-release fixes: acceptance

Run on the real desktop on 2026-09-24: Ubuntu 26.04, GNOME 50 on Wayland, Chromium snap 153, ThinkPad T14s Gen 6 (arm64). Built from `1c6e94d` and installed with `cargo install --path .`. The main Chromium was running for every launch.

| # | Check | Result | Notes |
|---|---|---|---|
| 1 | GitHub (shared), freshly installed, launched from the app grid: dock icon appears with the window, no lingering spinner | ✅ pass | Reinstalled as `github-com` after being removed, so GNOME had a fresh entry. |
| 2 | A new isolated app with a brand-new profile (`example.com --isolated --id first-run-check`) opens straight to the site, with no Chromium welcome or terms screen | ✅ pass | Removed afterwards with `--purge`. |
| 3 | Spotify (shared), refreshed in place with `winkle install --force` and no logout, launched from **Activities search**: icon appears at once | ✅ pass | Confirms the `--winkle-entry` hash makes gnome-shell reload the edited entry. |
| 4 | Same Spotify entry, launched from the **app grid** without logging out | ⚠️ known limitation | The grid reuses its icon, which still holds the entry loaded at login (`js/ui/appDisplay.js`). Documented in the README; fixed by logging out and back in. |

Background on all four: `spikes/app-id/FINDINGS.md`.
