# site-metadata Specification

## Purpose

Works out how a website should be presented as an app (name, icon, launch URL and shortcuts) from the URL the user gives, preferring the site's own web app manifest.

## Requirements

### Requirement: Accept only web URLs
The system SHALL accept `http` and `https` URLs only. A URL given without a scheme SHALL be treated as `https`. Any other scheme SHALL be rejected with an error and a non-zero exit status.

#### Scenario: Bare host is accepted
- **WHEN** the user runs `hermit install hey.com`
- **THEN** the system treats the URL as `https://hey.com/`

#### Scenario: Non-web scheme is rejected
- **WHEN** the user runs `hermit install file:///etc/passwd`
- **THEN** the system exits with a non-zero status and an error naming the unsupported scheme, and installs nothing

### Requirement: Follow redirects to the real site
The system SHALL follow HTTP redirects when fetching the page and SHALL resolve relative URLs in the page against the final URL after redirects.

#### Scenario: Redirect to an app subdomain
- **WHEN** `https://hey.com/` redirects to `https://app.hey.com/`
- **THEN** manifest and icon URLs found in the page are resolved against `https://app.hey.com/`

### Requirement: Prefer the web app manifest
When the page links a web app manifest, the system SHALL take the app name, icons, start URL, theme colour and shortcuts from it. The app name SHALL be the manifest `name`, or its `short_name` if `name` is absent.

#### Scenario: Site with a manifest
- **WHEN** the page contains `<link rel="manifest">` pointing to a manifest with `"name": "GitHub"`
- **THEN** the resolved app name is `GitHub`

#### Scenario: Manifest with only short_name
- **WHEN** the manifest has `"short_name": "HEY"` and no `name`
- **THEN** the resolved app name is `HEY`

### Requirement: Fall back to HTML metadata
When there is no manifest, it cannot be fetched or parsed, or it lacks a field, the system SHALL fill the gaps from the page itself. For the name, in order: `<meta name="application-name">`, `<meta property="og:site_name">`, `<title>`, then the host name.

#### Scenario: No manifest, has og:site_name
- **WHEN** the page has no manifest link and has `<meta property="og:site_name" content="Basecamp">`
- **THEN** the resolved app name is `Basecamp`

#### Scenario: Broken manifest does not fail the install
- **WHEN** the manifest link returns HTTP 404 or invalid JSON
- **THEN** the system continues using HTML metadata and reports a warning, not an error

### Requirement: Choose the best available icon
The system SHALL choose one icon, trying sources in this order:
1. manifest icons with purpose `any` (or no purpose)
2. manifest icons with purpose `maskable`
3. `apple-touch-icon` links
4. `icon` links
5. `/favicon.ico`

Within a source it SHALL prefer SVG, then the largest square raster image. If no icon can be fetched and decoded, the system SHALL generate a placeholder icon, using the theme colour when one is known, and SHALL still complete the install.

#### Scenario: Manifest offers several sizes
- **WHEN** the manifest lists PNG icons at 192×192 and 512×512 with purpose `any`
- **THEN** the 512×512 icon is chosen

#### Scenario: Maskable used only as fallback
- **WHEN** the manifest lists a 512×512 `maskable` icon and a 192×192 `any` icon
- **THEN** the 192×192 `any` icon is chosen

#### Scenario: No usable icon anywhere
- **WHEN** every icon source is missing or fails to decode
- **THEN** a placeholder icon is generated and the install succeeds with a warning

### Requirement: Determine the launch URL
The launch URL SHALL be the URL the user entered, normalised, **not** the URL reached after redirects. hermit fetches without the user's logins, so redirects often lead to a sign-in page; the browser follows redirects itself at launch. The exception: when the entered URL is the site root (path `/` with no query), the manifest `start_url` SHALL be used if it is same-origin with the fetched page. A cross-origin `start_url` SHALL be ignored.

#### Scenario: User gives a deep link
- **WHEN** the user installs `https://github.com/notifications`
- **THEN** the launch URL is `https://github.com/notifications`, whatever the manifest `start_url` says

#### Scenario: User gives the root, manifest has start_url
- **WHEN** the user installs `https://example.com/` and the manifest has `"start_url": "/app?source=pwa"`
- **THEN** the launch URL is `https://example.com/app?source=pwa`

#### Scenario: Logged-out redirect is not baked in
- **WHEN** the user installs `https://mail.example.com/inbox`, and without cookies it redirects to `https://accounts.example.com/login`
- **THEN** the launch URL is `https://mail.example.com/inbox`

### Requirement: Collect manifest shortcuts
The system SHALL collect up to 10 manifest `shortcuts` that have a name and a same-origin URL, in manifest order. Other shortcuts SHALL be ignored.

#### Scenario: Cross-origin shortcut is dropped
- **WHEN** the manifest has shortcuts to `/compose` and `https://other.example/x`
- **THEN** only the `/compose` shortcut is collected

### Requirement: User overrides win
`--name <text>` and `--icon <path-or-url>` SHALL override the resolved name and icon. If the page cannot be fetched at all, the install SHALL fail unless `--name` is given, in which case it SHALL proceed using the overrides and a placeholder icon if needed.

#### Scenario: Site unreachable with overrides
- **WHEN** the site times out and the user passed `--name Intranet --icon ./logo.png`
- **THEN** the app is installed as "Intranet" with `logo.png` as its icon

#### Scenario: Site unreachable without overrides
- **WHEN** the site times out and no `--name` was given
- **THEN** the system exits with a non-zero status, an error suggesting `--name`, and installs nothing
