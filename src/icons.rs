//! Turning a chosen icon (or none) into the files GNOME's icon theme reads.

use std::path::PathBuf;

use anyhow::{Context, Result};
use image::imageops::{self, FilterType};
use image::{DynamicImage, ImageFormat, RgbaImage};
use resvg::{tiny_skia, usvg};

use crate::fsutil::{remove_if_exists, write_atomic};
use crate::metadata::icon::IconImage;
use crate::paths::Paths;

/// Every raster icon is installed at this size, in `hicolor/256x256/apps`.
pub const SIZE: u32 = 256;

/// Families for the placeholder letter, most preferred first. The SVG asks for
/// these and then `sans-serif`, but resvg doesn't ask fontconfig what
/// `sans-serif` means (it defaults to Arial), so it's pointed at an installed
/// family instead.
const SANS_SERIF_PREFERENCES: &[&str] = &[
    "Ubuntu",
    "Cantarell",
    "Noto Sans",
    "DejaVu Sans",
    "Liberation Sans",
];

/// Neutral fallback background (GNOME's dark grey) when the site has no theme colour.
const PLACEHOLDER_GREY: (u8, u8, u8) = (0x5e, 0x5c, 0x64);

#[derive(Debug)]
pub struct IconFiles {
    pub png: Vec<u8>,
    /// Scalable version, when the source was SVG (or the placeholder).
    pub svg: Option<Vec<u8>>,
}

/// Render the icon files: the chosen icon, or a placeholder built from the
/// app name and theme colour.
pub fn render(
    icon: Option<&IconImage>,
    name: &str,
    theme_color: Option<&str>,
) -> Result<IconFiles> {
    match icon {
        Some(IconImage::Raster(img)) => Ok(IconFiles {
            png: encode_png(&normalise_raster(img))?,
            svg: None,
        }),
        Some(IconImage::Svg(svg)) => Ok(IconFiles {
            png: rasterise_svg(svg)?,
            svg: Some(svg.clone()),
        }),
        None => {
            let svg = placeholder_svg(name, theme_color).into_bytes();
            Ok(IconFiles {
                png: rasterise_svg(&svg)?,
                svg: Some(svg),
            })
        }
    }
}

/// Install the icon files for app `id`, replacing any from an earlier install.
pub fn install(paths: &Paths, id: &str, files: &IconFiles) -> Result<Vec<PathBuf>> {
    let png = paths.icon_png(id);
    let svg = paths.icon_svg(id);
    write_atomic(&png, &files.png)?;
    let mut written = vec![png];
    match &files.svg {
        Some(data) => {
            write_atomic(&svg, data)?;
            written.push(svg);
        }
        // A stale SVG from an earlier install would win over the new PNG.
        None => {
            remove_if_exists(&svg)?;
        }
    }
    Ok(written)
}

/// Remove app `id`'s icon files. Returns the ones that existed.
pub fn uninstall(paths: &Paths, id: &str) -> Result<Vec<PathBuf>> {
    let mut removed = Vec::new();
    for path in [paths.icon_png(id), paths.icon_svg(id)] {
        if remove_if_exists(&path)? {
            removed.push(path);
        }
    }
    Ok(removed)
}

/// Pad to a transparent square (never stretch), then scale to `SIZE`.
pub fn normalise_raster(img: &DynamicImage) -> RgbaImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let side = w.max(h);
    let mut square = RgbaImage::new(side, side);
    imageops::overlay(
        &mut square,
        &rgba,
        i64::from((side - w) / 2),
        i64::from((side - h) / 2),
    );
    if side == SIZE {
        square
    } else {
        imageops::resize(&square, SIZE, SIZE, FilterType::Lanczos3)
    }
}

/// Render an SVG to a `SIZE`×`SIZE` PNG, centred and scaled to fit.
pub fn rasterise_svg(svg: &[u8]) -> Result<Vec<u8>> {
    let mut opts = usvg::Options::default();
    if String::from_utf8_lossy(svg).contains("<text") {
        let fonts = opts.fontdb_mut();
        fonts.load_system_fonts();
        use_installed_sans_serif(fonts);
    }
    render_svg(svg, &opts)
}

/// Map `sans-serif` to the first preferred family that is installed, or to any
/// installed family, so text never silently disappears.
fn use_installed_sans_serif(fonts: &mut usvg::fontdb::Database) {
    let installed: Vec<String> = fonts
        .faces()
        .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
        .collect();
    let choice = SANS_SERIF_PREFERENCES
        .iter()
        .find(|preferred| installed.iter().any(|name| name == *preferred))
        .map(|preferred| preferred.to_string())
        .or_else(|| installed.first().cloned());
    if let Some(family) = choice {
        fonts.set_sans_serif_family(family);
    }
}

fn render_svg(svg: &[u8], opts: &usvg::Options) -> Result<Vec<u8>> {
    let tree = usvg::Tree::from_data(svg, opts).context("invalid SVG icon")?;
    let size = tree.size();
    let scale = SIZE as f32 / size.width().max(size.height());
    let dx = (SIZE as f32 - size.width() * scale) / 2.0;
    let dy = (SIZE as f32 - size.height() * scale) / 2.0;
    let mut pixmap = tiny_skia::Pixmap::new(SIZE, SIZE).context("allocating icon pixmap")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale).post_translate(dx, dy),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().context("encoding icon PNG")
}

/// A rounded square in the theme colour with the name's first letter.
pub fn placeholder_svg(name: &str, theme_color: Option<&str>) -> String {
    let (r, g, b) = theme_color
        .and_then(parse_hex_colour)
        .unwrap_or(PLACEHOLDER_GREY);
    // Relative luminance decides between light and dark text.
    let luminance = 0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b);
    let fg = if luminance > 160.0 {
        "#1e1e1e"
    } else {
        "#ffffff"
    };
    let letter = name
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().collect::<String>())
        .unwrap_or_else(|| "?".into());
    let letter = letter
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
  <rect x="16" y="16" width="224" height="224" rx="52" fill="#{r:02x}{g:02x}{b:02x}"/>
  <text x="128" y="173" text-anchor="middle" font-family="Ubuntu, Cantarell, Noto Sans, sans-serif" font-weight="700" font-size="128" fill="{fg}">{letter}</text>
</svg>
"##
    )
}

/// `#rgb`, `#rrggbb` or `#rrggbbaa` (alpha ignored).
fn parse_hex_colour(s: &str) -> Option<(u8, u8, u8)> {
    let hex = s.trim().strip_prefix('#')?;
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize, len: usize| u8::from_str_radix(&hex[i..i + len], 16).ok();
    match hex.len() {
        3 => Some((
            channel(0, 1)? * 17,
            channel(1, 1)? * 17,
            channel(2, 1)? * 17,
        )),
        6 | 8 => Some((channel(0, 2)?, channel(2, 2)?, channel(4, 2)?)),
        _ => None,
    }
}

fn encode_png(img: &RgbaImage) -> Result<Vec<u8>> {
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png)
        .context("encoding icon PNG")?;
    Ok(out.into_inner())
}

/// Decode a PNG we produced (for tests and dry-run descriptions).
#[cfg(test)]
fn decode(png: &[u8]) -> RgbaImage {
    image::load_from_memory(png).unwrap().to_rgba8()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::icon::tests::SVG;
    use image::{ImageEncoder, Rgba, codecs::ico::IcoEncoder};

    fn solid(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(w, h, Rgba([200, 30, 30, 255])))
    }

    #[test]
    fn tiny_ico_is_upscaled() {
        let mut ico = Vec::new();
        IcoEncoder::new(&mut ico)
            .write_image(
                solid(16, 16).to_rgba8().as_raw(),
                16,
                16,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        let Some(IconImage::Raster(img)) = crate::metadata::icon::decode_icon(&ico, None, None)
        else {
            panic!("ICO did not decode");
        };
        let out = decode(
            &render(Some(&IconImage::Raster(img)), "x", None)
                .unwrap()
                .png,
        );
        assert_eq!(out.dimensions(), (256, 256));
        assert_eq!(out.get_pixel(128, 128)[3], 255);
    }

    #[test]
    fn large_png_is_downscaled() {
        let files = render(Some(&IconImage::Raster(solid(512, 512))), "x", None).unwrap();
        assert!(files.svg.is_none());
        assert_eq!(decode(&files.png).dimensions(), (256, 256));
    }

    #[test]
    fn non_square_jpeg_is_padded_not_stretched() {
        let rgb = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            300,
            200,
            image::Rgb([10, 120, 200]),
        ));
        let mut jpeg = std::io::Cursor::new(Vec::new());
        rgb.write_to(&mut jpeg, ImageFormat::Jpeg).unwrap();
        let img = image::load_from_memory(jpeg.get_ref()).unwrap();
        let out = normalise_raster(&img);
        assert_eq!(out.dimensions(), (256, 256));
        // 300×200 padded to 300×300: 50px bands top and bottom, ≈43px after scaling.
        assert_eq!(
            out.get_pixel(128, 10)[3],
            0,
            "top band should be transparent"
        );
        assert_eq!(
            out.get_pixel(128, 245)[3],
            0,
            "bottom band should be transparent"
        );
        assert_eq!(out.get_pixel(128, 128)[3], 255);
        assert_eq!(
            out.get_pixel(5, 128)[3],
            255,
            "full width, so no side bands"
        );
    }

    #[test]
    fn svg_is_kept_and_rasterised() {
        let files = render(Some(&IconImage::Svg(SVG.as_bytes().to_vec())), "x", None).unwrap();
        assert_eq!(files.svg.as_deref(), Some(SVG.as_bytes()));
        let png = decode(&files.png);
        assert_eq!(png.dimensions(), (256, 256));
        assert_eq!(png.get_pixel(128, 128), &Rgba([0x11, 0x22, 0x33, 255]));
    }

    #[test]
    fn install_writes_both_and_removes_stale_svg() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::from_vars(|k| match k {
            "HOME" => Some(dir.path().into()),
            _ => None,
        })
        .unwrap();
        let svg_files = render(Some(&IconImage::Svg(SVG.as_bytes().to_vec())), "x", None).unwrap();
        let written = install(&paths, "demo", &svg_files).unwrap();
        assert_eq!(written, [paths.icon_png("demo"), paths.icon_svg("demo")]);
        assert!(decode(&std::fs::read(paths.icon_png("demo")).unwrap()).dimensions() == (256, 256));

        let raster_files = render(Some(&IconImage::Raster(solid(64, 64))), "x", None).unwrap();
        install(&paths, "demo", &raster_files).unwrap();
        assert!(!paths.icon_svg("demo").exists(), "stale SVG should be gone");

        assert_eq!(uninstall(&paths, "demo").unwrap(), [paths.icon_png("demo")]);
        assert!(uninstall(&paths, "demo").unwrap().is_empty());
    }

    #[test]
    fn hex_colours() {
        assert_eq!(parse_hex_colour("#24292f"), Some((0x24, 0x29, 0x2f)));
        assert_eq!(parse_hex_colour("#fff"), Some((255, 255, 255)));
        assert_eq!(parse_hex_colour("#11223344"), Some((0x11, 0x22, 0x33)));
        assert_eq!(parse_hex_colour("rebeccapurple"), None);
        assert_eq!(parse_hex_colour("#12345"), None);
    }

    #[test]
    fn placeholder_escapes_and_contrasts() {
        let svg = placeholder_svg("<script>", Some("#ffffff"));
        assert!(svg.contains(">S</text>"), "{svg}");
        assert!(
            svg.contains("fill=\"#1e1e1e\""),
            "dark text on light background"
        );
        let svg = placeholder_svg("&co", None);
        assert!(svg.contains(">C</text>"));
        assert!(svg.contains("#5e5c64"));
        assert!(placeholder_svg("", None).contains(">?</text>"));
    }

    fn white_pixels(png: &RgbaImage) -> usize {
        png.pixels()
            .filter(|p| p[0] > 240 && p[1] > 240 && p[2] > 240 && p[3] == 255)
            .count()
    }

    /// CI runners have DejaVu but none of the families the placeholder names;
    /// the letter must still be drawn.
    #[test]
    fn placeholder_letter_is_drawn_with_only_dejavu_installed() {
        let dejavu = std::path::Path::new("/usr/share/fonts/truetype/dejavu");
        if !dejavu.is_dir() {
            eprintln!("skipping: {} not installed", dejavu.display());
            return;
        }
        let mut opts = usvg::Options::default();
        let fonts = opts.fontdb_mut();
        fonts.load_fonts_dir(dejavu);
        use_installed_sans_serif(fonts);
        let svg = placeholder_svg("GitHub", Some("#24292f"));
        let png = decode(&render_svg(svg.as_bytes(), &opts).unwrap());
        let drawn = white_pixels(&png);
        assert!(drawn > 1500, "letter was not drawn ({drawn} white pixels)");
    }

    /// Renders the placeholder snapshot for eyeballing. Pixel checks keep it
    /// robust to font differences between machines.
    #[test]
    fn placeholder_snapshot() {
        let files = render(None, "GitHub", Some("#24292f")).unwrap();
        let snapshot =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/placeholder-G.png");
        if std::env::var_os("UPDATE_SNAPSHOTS").is_some() || !snapshot.exists() {
            write_atomic(&snapshot, &files.png).unwrap();
        }
        let png = decode(&files.png);
        assert_eq!(png.dimensions(), (256, 256));
        assert_eq!(
            png.get_pixel(2, 2)[3],
            0,
            "corner outside the rounded square is transparent"
        );
        assert_eq!(
            png.get_pixel(40, 128),
            &Rgba([0x24, 0x29, 0x2f, 255]),
            "background colour"
        );
        let white_pixels = png
            .pixels()
            .filter(|p| p[0] > 240 && p[1] > 240 && p[2] > 240 && p[3] == 255)
            .count();
        assert!(
            white_pixels > 1500,
            "letter was not drawn ({white_pixels} white pixels)"
        );
    }
}
