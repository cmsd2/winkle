//! Lenient web app manifest parsing: a wrongly-typed field is ignored rather
//! than failing the whole manifest.

use serde_json::Value;
use url::Url;

use super::html::clean_text;
use super::icon::IconCandidate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    pub name: String,
    pub url: Url,
}

#[derive(Debug, Default)]
pub struct Manifest {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub start_url: Option<Url>,
    pub theme_color: Option<String>,
    /// Icons with purpose `any` (or no purpose).
    pub icons_any: Vec<IconCandidate>,
    /// Icons whose only usable purpose is `maskable`.
    pub icons_maskable: Vec<IconCandidate>,
    /// All named shortcuts with valid URLs, unfiltered, in manifest order.
    pub shortcuts: Vec<Shortcut>,
}

#[derive(Debug, thiserror::Error)]
#[error("manifest is not a JSON object: {0}")]
pub struct ManifestError(String);

/// Parse a manifest, resolving its URLs against `manifest_url`.
pub fn parse_manifest(json: &[u8], manifest_url: &Url) -> Result<Manifest, ManifestError> {
    let value: Value = serde_json::from_slice(json).map_err(|e| ManifestError(e.to_string()))?;
    let obj = value
        .as_object()
        .ok_or_else(|| ManifestError("top-level value is not an object".into()))?;

    let text = |key: &str| obj.get(key).and_then(Value::as_str).and_then(clean_text);
    let url = |v: Option<&Value>| {
        v.and_then(Value::as_str)
            .and_then(|s| manifest_url.join(s.trim()).ok())
    };

    let mut manifest = Manifest {
        name: text("name"),
        short_name: text("short_name"),
        start_url: url(obj.get("start_url")),
        theme_color: text("theme_color"),
        ..Default::default()
    };

    for icon in obj.get("icons").and_then(Value::as_array).into_iter().flatten() {
        let Some(src) = url(icon.get("src")) else { continue };
        let candidate = IconCandidate::new(
            src,
            icon.get("sizes").and_then(Value::as_str),
            icon.get("type").and_then(Value::as_str),
        );
        let purpose = icon
            .get("purpose")
            .and_then(Value::as_str)
            .unwrap_or("any")
            .to_ascii_lowercase();
        let purposes: Vec<&str> = purpose.split_ascii_whitespace().collect();
        if purposes.is_empty() || purposes.contains(&"any") {
            manifest.icons_any.push(candidate);
        } else if purposes.contains(&"maskable") {
            manifest.icons_maskable.push(candidate);
        }
        // `monochrome`-only icons are silhouettes; not usable as app icons.
    }

    for shortcut in obj.get("shortcuts").and_then(Value::as_array).into_iter().flatten() {
        let name = shortcut
            .get("name")
            .or_else(|| shortcut.get("short_name"))
            .and_then(Value::as_str)
            .and_then(clean_text);
        if let (Some(name), Some(url)) = (name, url(shortcut.get("url"))) {
            manifest.shortcuts.push(Shortcut { name, url });
        }
    }

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    fn manifest_url() -> Url {
        Url::parse("https://github.com/manifest.json").unwrap()
    }

    #[test]
    fn github_like_manifest() {
        let m = parse_manifest(&fixture("manifest-github-like.json"), &manifest_url()).unwrap();
        assert_eq!(m.name.as_deref(), Some("GitHub"));
        assert_eq!(m.start_url.unwrap().as_str(), "https://github.com/?source=pwa");
        assert_eq!(m.theme_color.as_deref(), Some("#1e2327"));

        let any: Vec<_> = m.icons_any.iter().map(|i| (i.url.as_str(), i.size_hint)).collect();
        assert_eq!(
            any,
            [
                ("https://github.githubassets.com/assets/app-icon-192.png", Some(192)),
                ("https://github.githubassets.com/assets/app-icon-512.png", Some(512)),
            ]
        );
        let maskable: Vec<_> = m.icons_maskable.iter().map(|i| i.url.as_str()).collect();
        assert_eq!(maskable, ["https://github.com/assets/maskable-512.png"]);

        let shortcuts: Vec<_> = m
            .shortcuts
            .iter()
            .map(|s| (s.name.as_str(), s.url.as_str()))
            .collect();
        assert_eq!(
            shortcuts,
            [
                ("New issue", "https://github.com/issues/new"),
                ("Notifications", "https://github.com/notifications"),
                ("Elsewhere", "https://other.example/x"),
            ]
        );
    }

    #[test]
    fn invalid_json_is_an_error() {
        assert!(parse_manifest(&fixture("manifest-broken.json"), &manifest_url()).is_err());
        assert!(parse_manifest(b"[1, 2]", &manifest_url()).is_err());
    }

    #[test]
    fn wrongly_typed_fields_are_ignored() {
        let m = parse_manifest(&fixture("manifest-odd-types.json"), &manifest_url()).unwrap();
        assert_eq!(m.name, None);
        assert_eq!(m.short_name.as_deref(), Some("Odd"));
        assert_eq!(m.start_url, None);
        assert!(m.icons_any.is_empty());
        assert_eq!(m.shortcuts.len(), 1);
        assert_eq!(m.shortcuts[0].url.as_str(), "https://github.com/ok");
    }
}
