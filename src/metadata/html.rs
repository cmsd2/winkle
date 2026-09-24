//! Extracting app metadata from a page's `<head>`.

use scraper::{Html, Selector};
use url::Url;

use super::icon::IconCandidate;

#[derive(Debug, Default)]
pub struct PageInfo {
    pub manifest: Option<Url>,
    pub application_name: Option<String>,
    pub og_site_name: Option<String>,
    pub title: Option<String>,
    pub theme_color: Option<String>,
    pub apple_touch_icons: Vec<IconCandidate>,
    pub link_icons: Vec<IconCandidate>,
}

/// Parse `html`, resolving relative URLs against `base` (the page's final URL).
pub fn parse_page(html: &str, base: &Url) -> PageInfo {
    let doc = Html::parse_document(html);
    let mut info = PageInfo::default();

    let links = Selector::parse("head link[rel][href]").expect("valid selector");
    for link in doc.select(&links) {
        let el = link.value();
        let (Some(rel), Some(href)) = (el.attr("rel"), el.attr("href")) else {
            continue;
        };
        let Ok(url) = base.join(href.trim()) else {
            continue;
        };
        let rels: Vec<String> = rel
            .split_ascii_whitespace()
            .map(str::to_ascii_lowercase)
            .collect();
        let has = |name: &str| rels.iter().any(|r| r == name);

        if has("manifest") {
            info.manifest.get_or_insert(url);
        } else if has("apple-touch-icon") || has("apple-touch-icon-precomposed") {
            info.apple_touch_icons
                .push(IconCandidate::new(url, el.attr("sizes"), el.attr("type")));
        } else if has("icon") {
            info.link_icons
                .push(IconCandidate::new(url, el.attr("sizes"), el.attr("type")));
        }
    }

    let metas = Selector::parse("head meta[content]").expect("valid selector");
    for meta in doc.select(&metas) {
        let el = meta.value();
        let content = el.attr("content").unwrap_or_default();
        let key = el
            .attr("name")
            .or_else(|| el.attr("property"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let slot = match key.as_str() {
            "application-name" => &mut info.application_name,
            "og:site_name" => &mut info.og_site_name,
            "theme-color" => &mut info.theme_color,
            _ => continue,
        };
        if slot.is_none() {
            *slot = clean_text(content);
        }
    }

    let title = Selector::parse("head title").expect("valid selector");
    info.title = doc
        .select(&title)
        .next()
        .and_then(|t| clean_text(&t.text().collect::<String>()));

    info
}

/// Collapse whitespace and control characters; `None` if nothing is left.
pub fn clean_text(s: &str) -> Option<String> {
    let cleaned = s
        .split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!cleaned.is_empty()).then_some(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/tests/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn github_like_page() {
        let base = Url::parse("https://github.com/").unwrap();
        let info = parse_page(&fixture("github-like.html"), &base);
        assert_eq!(
            info.manifest.unwrap().as_str(),
            "https://github.com/manifest.json"
        );
        assert_eq!(info.application_name.as_deref(), Some("GitHub"));
        assert_eq!(info.og_site_name.as_deref(), Some("GitHub"));
        assert_eq!(info.theme_color.as_deref(), Some("#1e2327"));
        assert!(info.title.unwrap().starts_with("GitHub · Build"));
        assert!(info.apple_touch_icons.is_empty());
        let icons: Vec<_> = info.link_icons.iter().map(|i| i.url.as_str()).collect();
        assert_eq!(
            icons,
            [
                "https://github.githubassets.com/favicons/favicon.svg",
                "https://github.com/favicons/favicon.png",
            ]
        );
        assert!(info.link_icons[0].is_svg_hint());
    }

    #[test]
    fn hey_like_page_resolves_against_final_url() {
        let base = Url::parse("https://app.hey.com/sign_in").unwrap();
        let info = parse_page(&fixture("hey-like.html"), &base);
        assert!(info.manifest.is_none());
        assert_eq!(info.og_site_name.as_deref(), Some("HEY"));
        let touch: Vec<_> = info
            .apple_touch_icons
            .iter()
            .map(|i| i.url.as_str())
            .collect();
        assert_eq!(
            touch,
            [
                "https://app.hey.com/assets/apple-touch-icon.png",
                "https://app.hey.com/assets/apple-touch-icon-precomposed.png",
            ]
        );
        assert_eq!(info.apple_touch_icons[0].size_hint, Some(180));
        let icons: Vec<_> = info.link_icons.iter().map(|i| i.size_hint).collect();
        assert_eq!(icons, [Some(32), Some(96)]);
    }

    #[test]
    fn bare_page_only_has_a_title() {
        let base = Url::parse("https://intranet.example/").unwrap();
        let info = parse_page(&fixture("bare.html"), &base);
        assert_eq!(info.title.as_deref(), Some("Intranet Home"));
        assert!(info.manifest.is_none());
        assert!(info.application_name.is_none() && info.og_site_name.is_none());
        assert!(info.link_icons.is_empty() && info.apple_touch_icons.is_empty());
    }

    #[test]
    fn clean_text_strips_control_characters() {
        assert_eq!(clean_text(" a\n\tb\u{7}c "), Some("a b c".into()));
        assert_eq!(clean_text(" \n "), None);
    }
}
