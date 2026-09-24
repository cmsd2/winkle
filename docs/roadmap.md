# hermit roadmap

Where hermit is heading. Each milestone becomes one or more OpenSpec changes under
`openspec/changes/`, with its own specs; this page is direction, not a contract, and
the order may shift as we learn.

1. **MVP** (change `mvp`): install, list and remove; manifest-driven metadata; dock grouping; shared or isolated profiles; shortcut actions; Uninstall from the right-click menu.
2. **Everyday polish**
   - `hermit refresh` updates names, icons and shortcuts from the site.
   - `hermit doctor` finds broken entries, a moved `hermit` binary, missing icons or stale dock pins.
   - Activate-or-launch for arbitrary hotkeys (not only dock Super+1–9), probably via a small GNOME Shell extension.
3. **Link routing**
   - hermit registers itself as the default browser and sends links to installed apps by domain and scope (for example `github.com/...` → the GitHub app); everything else goes on to the real browser.
   - Apps register as handlers for protocols such as `mailto:` where the manifest declares them.
4. **Discovery**
   - A GNOME Shell search provider: typing a site or app name in Activities offers "Install *X* as an app".
   - A small curated catalogue of known-good apps with tuned metadata.
   - A way to install the page you're currently viewing in the browser.
5. **Native feel, deeper**
   - Unread badges on the dock via the LauncherEntry D-Bus API, if Chromium exposes badge counts or a helper can supply them.
   - Notifications attributed to the app rather than to Chromium.
   - Per-app window defaults.
6. **Broader reach (maybe)**
   - Other Chromium builds (deb, Flatpak, Chrome) and other desktops (KDE, Hyprland).
   - Adopting apps installed through Chromium's own "Install app" flow.
   - Packaging as a `.deb`.
