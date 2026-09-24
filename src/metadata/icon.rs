//! Icon candidates, decoding and choosing the best one.

use std::fmt;

use image::{DynamicImage, GenericImageView};
use url::Url;

/// How many candidates of one source to download before choosing.
const MAX_FETCHES_PER_SOURCE: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconCandidate {
    pub url: Url,
    /// Largest size declared in `sizes`: only a hint, real sizes come from decoding.
    pub size_hint: Option<u32>,
    pub mime: Option<String>,
}

impl IconCandidate {
    pub fn new(url: Url, sizes: Option<&str>, mime: Option<&str>) -> Self {
        let size_hint = sizes.and_then(|s| {
            s.split_ascii_whitespace()
                .filter_map(|token| {
                    let token = token.to_ascii_lowercase();
                    let (w, h) = token.split_once('x')?;
                    Some(w.parse::<u32>().ok()?.max(h.parse::<u32>().ok()?))
                })
                .max()
        });
        Self {
            url,
            size_hint,
            mime: mime.map(|m| m.trim().to_ascii_lowercase()).filter(|m| !m.is_empty()),
        }
    }

    pub fn is_svg_hint(&self) -> bool {
        self.mime.as_deref().is_some_and(|m| m.contains("svg"))
            || has_svg_extension(&self.url)
    }
}

/// Where an icon came from, in the priority order of the `site-metadata` spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IconSource {
    ManifestAny,
    ManifestMaskable,
    AppleTouch,
    LinkIcon,
    Favicon,
    Override,
}

impl fmt::Display for IconSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ManifestAny => "manifest icon",
            Self::ManifestMaskable => "manifest icon (maskable)",
            Self::AppleTouch => "apple-touch-icon",
            Self::LinkIcon => "page icon",
            Self::Favicon => "favicon.ico",
            Self::Override => "--icon",
        })
    }
}

#[derive(Debug)]
pub struct IconGroup {
    pub source: IconSource,
    pub candidates: Vec<IconCandidate>,
}

#[derive(Debug, Clone)]
pub enum IconImage {
    /// A valid SVG document (possibly gzipped).
    Svg(Vec<u8>),
    Raster(DynamicImage),
}

impl IconImage {
    pub fn describe(&self) -> String {
        match self {
            Self::Svg(_) => "SVG".into(),
            Self::Raster(img) => {
                let (w, h) = img.dimensions();
                format!("{w}×{h}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChosenIcon {
    /// Where it came from: a URL or a local path.
    pub origin: String,
    pub source: IconSource,
    pub image: IconImage,
}

/// Decode icon bytes as SVG or a raster format; `None` if they are neither.
pub fn decode_icon(bytes: &[u8], content_type: Option<&str>, url: Option<&Url>) -> Option<IconImage> {
    let looks_svg = content_type.is_some_and(|ct| ct.contains("svg"))
        || url.is_some_and(has_svg_extension)
        || sniff_svg(bytes);
    if looks_svg {
        let opts = resvg::usvg::Options::default();
        return resvg::usvg::Tree::from_data(bytes, &opts)
            .ok()
            .map(|_| IconImage::Svg(bytes.to_vec()));
    }
    let img = image::load_from_memory(bytes).ok()?;
    let (w, h) = img.dimensions();
    (w > 0 && h > 0).then_some(IconImage::Raster(img))
}

/// Try each source in priority order and return the best icon of the first
/// source that yields any usable one. Within a source, SVG wins, then the
/// largest square raster (decoded size, not the declared one).
pub fn choose_icon(
    groups: &[IconGroup],
    mut fetch: impl FnMut(&Url) -> Option<(Vec<u8>, Option<String>)>,
) -> Option<ChosenIcon> {
    for group in groups {
        let mut candidates: Vec<&IconCandidate> = Vec::new();
        for c in &group.candidates {
            if !candidates.iter().any(|seen| seen.url == c.url) {
                candidates.push(c);
            }
        }
        // Most promising first, so the fetch cap drops the least likely ones.
        candidates.sort_by_key(|c| (!c.is_svg_hint(), std::cmp::Reverse(c.size_hint)));

        let mut rasters = Vec::new();
        for c in candidates.into_iter().take(MAX_FETCHES_PER_SOURCE) {
            let Some((bytes, content_type)) = fetch(&c.url) else { continue };
            match decode_icon(&bytes, content_type.as_deref(), Some(&c.url)) {
                Some(image @ IconImage::Svg(_)) => {
                    return Some(ChosenIcon {
                        origin: c.url.to_string(),
                        source: group.source,
                        image,
                    });
                }
                Some(IconImage::Raster(img)) => rasters.push((c, img)),
                None => {}
            }
        }
        if let Some((c, img)) = rasters.into_iter().max_by_key(|(_, img)| raster_rank(img)) {
            return Some(ChosenIcon {
                origin: c.url.to_string(),
                source: group.source,
                image: IconImage::Raster(img),
            });
        }
    }
    None
}

/// Square beats non-square; then the larger short side; then the larger area.
fn raster_rank(img: &DynamicImage) -> (bool, u32, u64) {
    let (w, h) = img.dimensions();
    (w == h, w.min(h), u64::from(w) * u64::from(h))
}

fn has_svg_extension(url: &Url) -> bool {
    let path = url.path().to_ascii_lowercase();
    path.ends_with(".svg") || path.ends_with(".svgz")
}

fn sniff_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(1024)];
    let text = String::from_utf8_lossy(head);
    let text = text.trim_start_matches('\u{feff}').trim_start();
    text.starts_with("<svg") || (text.starts_with("<?xml") && text.contains("<svg"))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Cursor;

    pub fn png(w: u32, h: u32) -> Vec<u8> {
        let img = DynamicImage::new_rgba8(w, h);
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    pub const SVG: &str =
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="#123"/></svg>"##;

    fn cand(url: &str, sizes: Option<&str>) -> IconCandidate {
        IconCandidate::new(Url::parse(url).unwrap(), sizes, None)
    }

    fn server(files: &[(&str, Vec<u8>)]) -> impl FnMut(&Url) -> Option<(Vec<u8>, Option<String>)> + use<> {
        let files: HashMap<String, Vec<u8>> =
            files.iter().map(|(u, b)| (u.to_string(), b.clone())).collect();
        move |url: &Url| files.get(url.as_str()).map(|b| (b.clone(), None))
    }

    #[test]
    fn size_hints() {
        assert_eq!(cand("https://x/a.png", Some("16x16 32X32")).size_hint, Some(32));
        assert_eq!(cand("https://x/a.png", Some("any")).size_hint, None);
        assert_eq!(cand("https://x/a.png", None).size_hint, None);
        assert!(cand("https://x/a.SVG", None).is_svg_hint());
    }

    #[test]
    fn several_sizes_picks_the_largest_decoded() {
        // The declared sizes lie; decoded sizes decide.
        let groups = [IconGroup {
            source: IconSource::ManifestAny,
            candidates: vec![
                cand("https://x/192.png", Some("512x512")),
                cand("https://x/512.png", Some("192x192")),
            ],
        }];
        let fetch = server(&[("https://x/192.png", png(192, 192)), ("https://x/512.png", png(512, 512))]);
        let chosen = choose_icon(&groups, fetch).unwrap();
        assert_eq!(chosen.origin, "https://x/512.png");
        assert_eq!(chosen.image.describe(), "512×512");
    }

    #[test]
    fn maskable_only_as_fallback() {
        let groups = [
            IconGroup {
                source: IconSource::ManifestAny,
                candidates: vec![cand("https://x/any-192.png", Some("192x192"))],
            },
            IconGroup {
                source: IconSource::ManifestMaskable,
                candidates: vec![cand("https://x/mask-512.png", Some("512x512"))],
            },
        ];
        let fetch = server(&[
            ("https://x/any-192.png", png(192, 192)),
            ("https://x/mask-512.png", png(512, 512)),
        ]);
        let chosen = choose_icon(&groups, fetch).unwrap();
        assert_eq!(chosen.origin, "https://x/any-192.png");
        assert_eq!(chosen.source, IconSource::ManifestAny);
    }

    #[test]
    fn falls_through_sources_when_fetches_fail() {
        let groups = [
            IconGroup {
                source: IconSource::ManifestAny,
                candidates: vec![cand("https://x/missing.png", None)],
            },
            IconGroup {
                source: IconSource::AppleTouch,
                candidates: vec![cand("https://x/garbage.png", None), cand("https://x/touch.png", None)],
            },
        ];
        let fetch = server(&[
            ("https://x/garbage.png", b"not an image".to_vec()),
            ("https://x/touch.png", png(180, 180)),
        ]);
        let chosen = choose_icon(&groups, fetch).unwrap();
        assert_eq!(chosen.source, IconSource::AppleTouch);
        assert_eq!(chosen.origin, "https://x/touch.png");
    }

    #[test]
    fn svg_beats_bigger_raster_in_the_same_source() {
        let groups = [IconGroup {
            source: IconSource::LinkIcon,
            candidates: vec![cand("https://x/big.png", Some("512x512")), cand("https://x/logo", None)],
        }];
        let fetch = server(&[("https://x/big.png", png(512, 512)), ("https://x/logo", SVG.as_bytes().to_vec())]);
        let chosen = choose_icon(&groups, fetch).unwrap();
        assert_eq!(chosen.origin, "https://x/logo");
        assert!(matches!(chosen.image, IconImage::Svg(_)));
    }

    #[test]
    fn square_beats_larger_non_square() {
        let groups = [IconGroup {
            source: IconSource::LinkIcon,
            candidates: vec![cand("https://x/wide.png", None), cand("https://x/square.png", None)],
        }];
        let fetch = server(&[("https://x/wide.png", png(600, 200)), ("https://x/square.png", png(64, 64))]);
        assert_eq!(choose_icon(&groups, fetch).unwrap().origin, "https://x/square.png");
    }

    #[test]
    fn no_usable_icon_anywhere() {
        let groups = [
            IconGroup { source: IconSource::ManifestAny, candidates: vec![cand("https://x/a.png", None)] },
            IconGroup { source: IconSource::Favicon, candidates: vec![cand("https://x/favicon.ico", None)] },
        ];
        let fetch = server(&[("https://x/favicon.ico", b"<html>nope</html>".to_vec())]);
        assert!(choose_icon(&groups, fetch).is_none());
    }

    #[test]
    fn broken_svg_is_rejected() {
        assert!(decode_icon(b"<svg><rect", Some("image/svg+xml"), None).is_none());
        assert!(matches!(decode_icon(SVG.as_bytes(), None, None), Some(IconImage::Svg(_))));
    }
}
