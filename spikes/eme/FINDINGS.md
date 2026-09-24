# Spike: why Spotify wouldn't play in a winkle app

Symptom: Spotify web player (installed with winkle, shared profile) loads and shows
the library, but Play does nothing and the volume slider has no handle; the usual
signs that the web player couldn't register this browser as a playback device.

## Ruled out

- **Snap audio**: `chromium:audio-playback` is connected; PipeWire's default sink
  is the speaker.
- **Protected content blocked**: no `protected_media_identifier` override in the
  Default profile's Preferences.
- **Codecs**: in Chromium snap 153 (arm64), `MediaSource.isTypeSupported` is true
  for AAC (`mp4a.40.2`), FLAC, Opus and H.264; false for Ogg Vorbis.
- **Widevine missing**: Google's arm64 Widevine 4.10.3057.0 is downloaded in
  `~/snap/chromium/common/chromium/WidevineCdm/` and Chromium registers it.
- **AppArmor**: no denials mentioning Widevine (only unrelated Vulkan ICD paths).
- **winkle's launch**: the app is opened in the existing browser session
  ("Opening in existing browser session"), same process and profile as a tab.

## Observed (headless, `probe.html` over http://127.0.0.1)

- `requestMediaKeySystemAccess('com.widevine.alpha', AAC)` → resolves.
- `createMediaKeys()` → never settles (15 s), and no CDM process is logged.
  Headless mode may not support library CDMs, so this is **not conclusive**.

## Cause (confirmed)

In the user's real browser, `probe.html` failed at the first step:
`requestMediaKeySystemAccess` → `NotSupportedError: Unsupported keySystem or
supportedConfigurations`. Chromium had started at 06:39:03 and the component
updater downloaded Widevine at 06:40:06. On Linux, Chromium registers Widevine at
startup (from the hint file), so a browser session that was already running
when Widevine arrived can't use it. Headless runs started later could, which is
why they got further.

**Fix:** restart Chromium once (`chrome://restart`). Afterwards Spotify played in
both a normal tab and the winkle app. Not a winkle bug; only hit on a fresh
Chromium install, before its first restart.

Side note: headless mode found Widevine but `createMediaKeys()` never settled,
so headless isn't a reliable way to test DRM.
