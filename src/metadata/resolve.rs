//! Putting it together: from an entered URL to everything needed to install.

use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use url::Url;

use super::fetch::{Fetcher, MAX_HTML, MAX_ICON, MAX_MANIFEST};
use super::html::{clean_text, parse_page};
use super::icon::{ChosenIcon, IconCandidate, IconGroup, IconSource, choose_icon, decode_icon};
use super::manifest::{Manifest, Shortcut, parse_manifest};

pub const MAX_SHORTCUTS: usize = 10;

#[derive(Debug, Default)]
pub struct Overrides {
    pub name: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug)]
pub struct AppMetadata {
    pub name: String,
    pub launch_url: Url,
    pub theme_color: Option<String>,
    /// `None` means no usable icon was found: use a placeholder.
    pub icon: Option<ChosenIcon>,
    pub shortcuts: Vec<Shortcut>,
    pub warnings: Vec<String>,
}

pub fn resolve(entered: &Url, overrides: &Overrides, fetcher: &Fetcher) -> Result<AppMetadata> {
    let mut warnings = Vec::new();
    let name_override = match &overrides.name {
        Some(n) => Some(clean_text(n).context("--name is empty")?),
        None => None,
    };
    let icon_override = overrides
        .icon
        .as_deref()
        .map(|spec| load_icon_override(spec, fetcher))
        .transpose()?;

    let page = match fetcher.get(entered, MAX_HTML) {
        Ok(page) => page,
        Err(err) => {
            let Some(name) = name_override else {
                bail!(
                    "couldn't fetch {entered}: {err}\n\
                     hint: pass --name (and optionally --icon) to install it anyway"
                );
            };
            warnings.push(format!(
                "couldn't fetch {entered} ({err}); using --name and --icon only"
            ));
            if icon_override.is_none() {
                warnings.push("no icon; using a generated placeholder".into());
            }
            return Ok(AppMetadata {
                name,
                launch_url: entered.clone(),
                theme_color: None,
                icon: icon_override,
                shortcuts: Vec::new(),
                warnings,
            });
        }
    };

    let page_url = page.final_url.clone();
    let info = parse_page(&String::from_utf8_lossy(&page.body), &page_url);

    let manifest = match &info.manifest {
        None => Manifest::default(),
        Some(manifest_url) => match fetcher
            .get(manifest_url, MAX_MANIFEST)
            .map_err(|e| e.to_string())
            .and_then(|r| parse_manifest(&r.body, &r.final_url).map_err(|e| e.to_string()))
        {
            Ok(m) => m,
            Err(err) => {
                warnings.push(format!("ignoring manifest {manifest_url}: {err}"));
                Manifest::default()
            }
        },
    };

    let name = name_override
        .or(manifest.name.clone())
        .or(manifest.short_name.clone())
        .or(info.application_name.clone())
        .or(info.og_site_name.clone())
        .or(info.title.clone())
        .unwrap_or_else(|| host_name(entered));

    let launch_url = launch_url(entered, manifest.start_url.as_ref(), &page_url);

    let shortcuts = manifest
        .shortcuts
        .iter()
        .filter(|s| s.url.origin() == page_url.origin())
        .take(MAX_SHORTCUTS)
        .cloned()
        .collect();

    let icon = match icon_override {
        Some(icon) => Some(icon),
        None => {
            let favicon = page_url
                .join("/favicon.ico")
                .ok()
                .map(|u| IconCandidate::new(u, None, None));
            let groups = [
                IconGroup {
                    source: IconSource::ManifestAny,
                    candidates: manifest.icons_any.clone(),
                },
                IconGroup {
                    source: IconSource::ManifestMaskable,
                    candidates: manifest.icons_maskable.clone(),
                },
                IconGroup {
                    source: IconSource::AppleTouch,
                    candidates: info.apple_touch_icons.clone(),
                },
                IconGroup {
                    source: IconSource::LinkIcon,
                    candidates: info.link_icons.clone(),
                },
                IconGroup {
                    source: IconSource::Favicon,
                    candidates: favicon.into_iter().collect(),
                },
            ];
            let chosen = choose_icon(&groups, |url| {
                fetcher
                    .get(url, MAX_ICON)
                    .ok()
                    .map(|r| (r.body, r.content_type))
            });
            if chosen.is_none() {
                warnings.push("no usable icon found; using a generated placeholder".into());
            }
            chosen
        }
    };

    Ok(AppMetadata {
        name,
        launch_url,
        theme_color: manifest.theme_color.or(info.theme_color),
        icon,
        shortcuts,
        warnings,
    })
}

/// The entered URL, unless it is the bare site root and the manifest has a
/// `start_url` on the same origin as the fetched page.
fn launch_url(entered: &Url, start_url: Option<&Url>, page_url: &Url) -> Url {
    let is_root = entered.path() == "/" && entered.query().is_none();
    match start_url {
        Some(start) if is_root && start.origin() == page_url.origin() => start.clone(),
        _ => entered.clone(),
    }
}

fn host_name(url: &Url) -> String {
    let host = url.host_str().unwrap_or("app");
    host.strip_prefix("www.").unwrap_or(host).to_string()
}

fn load_icon_override(spec: &str, fetcher: &Fetcher) -> Result<ChosenIcon> {
    let (bytes, content_type, url) = if spec.starts_with("http://") || spec.starts_with("https://")
    {
        let url = Url::parse(spec).with_context(|| format!("invalid --icon URL `{spec}`"))?;
        let resp = fetcher
            .get(&url, MAX_ICON)
            .map_err(|e| anyhow!("couldn't fetch --icon {spec}: {e}"))?;
        (resp.body, resp.content_type, Some(url))
    } else {
        let bytes = std::fs::read(Path::new(spec))
            .with_context(|| format!("couldn't read --icon {spec}"))?;
        (bytes, None, None)
    };
    let is_svg_path = url.is_none() && spec.to_ascii_lowercase().ends_with(".svg");
    let content_type = content_type.or(is_svg_path.then(|| "image/svg+xml".to_string()));
    let image = decode_icon(&bytes, content_type.as_deref(), url.as_ref())
        .with_context(|| format!("--icon {spec} is not a PNG, ICO, JPEG, WebP or SVG image"))?;
    Ok(ChosenIcon {
        origin: spec.to_string(),
        source: IconSource::Override,
        image,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::icon::tests::{SVG, png};
    use httpmock::prelude::*;

    fn fetcher() -> Fetcher {
        Fetcher::with_timeout(std::time::Duration::from_secs(2)).unwrap()
    }

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    fn html_with_manifest() -> &'static str {
        r#"<html><head><title>Ignored</title><link rel="manifest" href="/m.json"></head></html>"#
    }

    #[test]
    fn launch_url_rules() {
        let page = url("https://example.com/");
        let start = url("https://example.com/app?source=pwa");
        // Deep link wins.
        assert_eq!(
            launch_url(
                &url("https://example.com/notifications"),
                Some(&start),
                &page
            )
            .as_str(),
            "https://example.com/notifications"
        );
        // Root uses start_url.
        assert_eq!(
            launch_url(&url("https://example.com/"), Some(&start), &page),
            start
        );
        // Root with a query is not the bare root.
        assert_eq!(
            launch_url(&url("https://example.com/?a=1"), Some(&start), &page).as_str(),
            "https://example.com/?a=1"
        );
        // Cross-origin start_url is ignored.
        assert_eq!(
            launch_url(
                &url("https://example.com/"),
                Some(&url("https://evil.example/")),
                &page
            )
            .as_str(),
            "https://example.com/"
        );
        // start_url on the redirected-to origin is accepted.
        assert_eq!(
            launch_url(
                &url("https://hey.com/"),
                Some(&url("https://app.hey.com/imbox")),
                &url("https://app.hey.com/")
            )
            .as_str(),
            "https://app.hey.com/imbox"
        );
    }

    #[test]
    fn manifest_site_end_to_end() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("content-type", "text/html")
                .body(html_with_manifest());
        });
        let manifest = r##"{"name": "Example App", "start_url": "/app?source=pwa", "theme_color": "#224466",
            "icons": [{"src": "/i192.png", "sizes": "192x192"}, {"src": "/i512.png", "sizes": "512x512"}],
            "shortcuts": [{"name": "Compose", "url": "/compose"}, {"name": "Away", "url": "https://other.example/x"}]}"##;
        server.mock(|when, then| {
            when.method(GET).path("/m.json");
            then.status(200).body(manifest);
        });
        server.mock(|when, then| {
            when.method(GET).path("/i192.png");
            then.status(200)
                .header("content-type", "image/png")
                .body(png(192, 192));
        });
        server.mock(|when, then| {
            when.method(GET).path("/i512.png");
            then.status(200)
                .header("content-type", "image/png")
                .body(png(512, 512));
        });

        let meta = resolve(&url(&server.url("/")), &Overrides::default(), &fetcher()).unwrap();
        assert_eq!(meta.name, "Example App");
        assert_eq!(meta.launch_url.as_str(), server.url("/app?source=pwa"));
        assert_eq!(meta.theme_color.as_deref(), Some("#224466"));
        let icon = meta.icon.unwrap();
        assert_eq!(icon.origin, server.url("/i512.png"));
        assert_eq!(icon.source, IconSource::ManifestAny);
        assert_eq!(meta.shortcuts.len(), 1);
        assert_eq!(meta.shortcuts[0].name, "Compose");
        assert_eq!(meta.shortcuts[0].url.as_str(), server.url("/compose"));
        assert!(meta.warnings.is_empty(), "{:?}", meta.warnings);
    }

    #[test]
    fn redirected_page_resolves_against_final_url_but_launches_entered_url() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/inbox");
            then.status(302).header("location", "/login/");
        });
        server.mock(|when, then| {
            when.method(GET).path("/login/");
            then.status(200).body(
                r#"<html><head><meta property="og:site_name" content="Mail"><link rel="icon" href="logo.svg"></head></html>"#,
            );
        });
        server.mock(|when, then| {
            when.method(GET).path("/login/logo.svg");
            then.status(200).body(SVG);
        });
        let entered = url(&server.url("/inbox"));
        let meta = resolve(&entered, &Overrides::default(), &fetcher()).unwrap();
        assert_eq!(meta.launch_url, entered);
        assert_eq!(meta.name, "Mail");
        assert_eq!(meta.icon.unwrap().origin, server.url("/login/logo.svg"));
    }

    #[test]
    fn broken_manifest_warns_and_falls_back_to_html() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200).body(
                r#"<html><head><link rel="manifest" href="/m.json"><meta property="og:site_name" content="Basecamp"></head></html>"#,
            );
        });
        server.mock(|when, then| {
            when.method(GET).path("/m.json");
            then.status(404);
        });
        let meta = resolve(&url(&server.url("/")), &Overrides::default(), &fetcher()).unwrap();
        assert_eq!(meta.name, "Basecamp");
        assert!(
            meta.warnings
                .iter()
                .any(|w| w.contains("ignoring manifest")),
            "{:?}",
            meta.warnings
        );
        // No icons anywhere, including /favicon.ico: placeholder.
        assert!(meta.icon.is_none());
        assert!(meta.warnings.iter().any(|w| w.contains("placeholder")));
    }

    #[test]
    fn name_falls_back_to_host() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200).body("<html><head></head></html>");
        });
        let meta = resolve(&url(&server.url("/")), &Overrides::default(), &fetcher()).unwrap();
        assert_eq!(meta.name, "127.0.0.1");
    }

    #[test]
    fn unreachable_without_name_fails_with_hint() {
        let err = resolve(
            &url("http://127.0.0.1:1/"),
            &Overrides::default(),
            &fetcher(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("--name"), "{err}");
    }

    #[test]
    fn unreachable_with_overrides_uses_them() {
        let dir = tempfile::tempdir().unwrap();
        let logo = dir.path().join("logo.png");
        std::fs::write(&logo, png(64, 64)).unwrap();
        let overrides = Overrides {
            name: Some("Intranet".into()),
            icon: Some(logo.to_str().unwrap().into()),
        };
        let meta = resolve(&url("http://127.0.0.1:1/"), &overrides, &fetcher()).unwrap();
        assert_eq!(meta.name, "Intranet");
        assert_eq!(meta.launch_url.as_str(), "http://127.0.0.1:1/");
        let icon = meta.icon.unwrap();
        assert_eq!(icon.source, IconSource::Override);
        assert_eq!(icon.image.describe(), "64×64");
    }

    #[test]
    fn overrides_win_over_site_metadata() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .body(r#"<html><head><title>Site</title></head></html>"#);
        });
        server.mock(|when, then| {
            when.method(GET).path("/custom.svg");
            then.status(200)
                .header("content-type", "image/svg+xml")
                .body(SVG);
        });
        let overrides = Overrides {
            name: Some("  My   Name ".into()),
            icon: Some(server.url("/custom.svg")),
        };
        let meta = resolve(&url(&server.url("/")), &overrides, &fetcher()).unwrap();
        assert_eq!(meta.name, "My Name");
        assert_eq!(meta.icon.unwrap().origin, server.url("/custom.svg"));
    }

    #[test]
    fn bad_icon_override_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.png");
        std::fs::write(&bad, b"nope").unwrap();
        let overrides = Overrides {
            name: Some("X".into()),
            icon: Some(bad.to_str().unwrap().into()),
        };
        assert!(resolve(&url("http://127.0.0.1:1/"), &overrides, &fetcher()).is_err());
    }
}
