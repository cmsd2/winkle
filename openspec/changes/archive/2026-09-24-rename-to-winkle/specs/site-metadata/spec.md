# Spec Delta

## MODIFIED Requirements

### Requirement: Accept only web URLs
The system SHALL accept `http` and `https` URLs only. A URL given without a scheme SHALL be treated as `https`. Any other scheme SHALL be rejected with an error and a non-zero exit status.

#### Scenario: Bare host is accepted
- **WHEN** the user runs `winkle install hey.com`
- **THEN** the system treats the URL as `https://hey.com/`

#### Scenario: Non-web scheme is rejected
- **WHEN** the user runs `winkle install file:///etc/passwd`
- **THEN** the system exits with a non-zero status and an error naming the unsupported scheme, and installs nothing

### Requirement: Determine the launch URL
The launch URL SHALL be the URL the user entered, normalised, **not** the URL reached after redirects. winkle fetches without the user's logins, so redirects often lead to a sign-in page; the browser follows redirects itself at launch. The exception: when the entered URL is the site root (path `/` with no query), the manifest `start_url` SHALL be used if it is same-origin with the fetched page. A cross-origin `start_url` SHALL be ignored.

#### Scenario: User gives a deep link
- **WHEN** the user installs `https://github.com/notifications`
- **THEN** the launch URL is `https://github.com/notifications`, whatever the manifest `start_url` says

#### Scenario: User gives the root, manifest has start_url
- **WHEN** the user installs `https://example.com/` and the manifest has `"start_url": "/app?source=pwa"`
- **THEN** the launch URL is `https://example.com/app?source=pwa`

#### Scenario: Logged-out redirect is not baked in
- **WHEN** the user installs `https://mail.example.com/inbox`, and without cookies it redirects to `https://accounts.example.com/login`
- **THEN** the launch URL is `https://mail.example.com/inbox`
