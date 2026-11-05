// SPDX-License-Identifier: Apache-2.0
//! Favicon and PWA icon generator.
//!
//! Renders a mark (any IR icon, by default the Aphrody mark) on a preset-colored badge with `resvg`/`tiny-skia`:
//!
//! | File | Size | Shape |
//! | --- | --- | --- |
//! | `favicon.ico` | 16, 32, 48 (PNG entries) | rounded badge, enlarged mark for legibility |
//! | `favicon-16.png`, `favicon-32.png`, `favicon-48.png` | 16, 32, 48 | same |
//! | `favicon.svg` | vector | rounded badge, light and `prefers-color-scheme: dark` colors |
//! | `apple-touch-icon.png` | 180 | full-bleed square (iOS applies its own mask) |
//! | `icon-192.png`, `icon-512.png` | 192, 512 | rounded badge, `purpose: any` |
//! | `icon-maskable-192.png`, `icon-maskable-512.png` | 192, 512 | full bleed, mark inside the 80% safe zone |
//! | `manifest.webmanifest` | | W3C Web App Manifest |
//!
//! The ICO container is written here (ICONDIR + ICONDIRENTRY + PNG payloads, the Vista+ PNG-in-ICO layout) and
//! [`verify_dir`] decodes every output back.

use crate::Error;
use crate::color::IconPalette;
use crate::ir::{Icon, format_number};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;
use serde::Serialize;
use std::fmt::Write as _;
use std::path::Path;

/// Sizes packed into `favicon.ico`.
pub const ICO_SIZES: [u32; 3] = [16, 32, 48];
/// Every PNG size produced.
pub const PNG_SIZES: [u32; 6] = [16, 32, 48, 180, 192, 512];

/// Badge geometry on the 24-unit composition grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// Rounded square badge with transparent corners.
    Rounded { radius: f64, mark_scale: f64 },
    /// Opaque full-bleed square (maskable and Apple touch icons).
    FullBleed { mark_scale: f64 },
}

impl Shape {
    /// Small favicon sizes: tighter corners, larger mark.
    pub const SMALL: Self = Self::Rounded {
        radius: 4.5,
        mark_scale: 0.94,
    };
    /// Large `purpose: any` icons and the SVG favicon.
    pub const ANY: Self = Self::Rounded {
        radius: 5.5,
        mark_scale: 0.8,
    };
    /// Apple touch icon.
    pub const APPLE: Self = Self::FullBleed { mark_scale: 0.76 };
    /// Maskable: the mark's bounding box fits inside the centered circle of radius 40% (W3C safe zone).
    pub const MASKABLE: Self = Self::FullBleed { mark_scale: 0.62 };
}

/// Generator options.
#[derive(Debug, Clone)]
pub struct FaviconOptions {
    pub name: String,
    pub short_name: String,
    pub light: IconPalette,
    pub dark: IconPalette,
    /// Paint the first mark layer with the palette accent.
    pub two_tone: bool,
    pub start_url: String,
}

/// One written file.
#[derive(Debug, Clone, Serialize)]
pub struct OutputFile {
    pub path: String,
    pub bytes: u64,
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
}

/// Result of [`generate`].
#[derive(Debug, Clone, Serialize)]
pub struct FaviconReport {
    pub palette: IconPalette,
    pub dark_palette: IconPalette,
    pub files: Vec<OutputFile>,
}

fn mark_transform(mark: &Icon, mark_scale: f64) -> String {
    let k = 24.0 / mark.grid;
    let (cx, cy) = mark
        .control_bounds()
        .map_or((mark.grid / 2.0, mark.grid / 2.0), |(a, b)| {
            ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
        });
    let s = k * mark_scale;
    format!(
        "translate(12 12) scale({}) translate({} {})",
        format_number(s, 5),
        format_number(-cx, 4),
        format_number(-cy, 4)
    )
}

fn mark_paths(mark: &Icon, fg: &str, accent: Option<&str>) -> String {
    let mut out = String::new();
    for (i, layer) in mark.layers.iter().enumerate() {
        let single = Icon {
            layers: vec![layer.clone()],
            ..mark.clone()
        };
        let color = match (i, accent) {
            (0, Some(a)) if mark.layers.len() > 1 => a,
            _ => fg,
        };
        out.push_str(&single.svg_body(3, Some(color)));
    }
    out
}

/// Composes the badge as an SVG document on a 24x24 user space.
#[must_use]
pub fn compose(mark: &Icon, palette: &IconPalette, shape: Shape, two_tone: bool) -> String {
    let accent = two_tone.then_some(palette.accent.as_str());
    let (background, mark_scale) = match shape {
        Shape::Rounded { radius, mark_scale } => (
            format!(
                "<rect width=\"24\" height=\"24\" rx=\"{}\" fill=\"{}\"/>",
                format_number(radius, 3),
                palette.background
            ),
            mark_scale,
        ),
        Shape::FullBleed { mark_scale } => (
            format!(
                "<rect width=\"24\" height=\"24\" fill=\"{}\"/>",
                palette.background
            ),
            mark_scale,
        ),
    };
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\">{background}<g transform=\"{}\">{}</g></svg>",
        mark_transform(mark, mark_scale),
        mark_paths(mark, &palette.foreground, accent)
    )
}

/// The vector favicon: light colors by default, dark colors under `prefers-color-scheme: dark`.
#[must_use]
pub fn favicon_svg(mark: &Icon, options: &FaviconOptions) -> String {
    let Shape::Rounded { radius, mark_scale } = Shape::ANY else {
        unreachable!("ANY is rounded")
    };
    let (l, d) = (&options.light, &options.dark);
    let mut paths = String::new();
    for (i, layer) in mark.layers.iter().enumerate() {
        let single = Icon {
            layers: vec![layer.clone()],
            ..mark.clone()
        };
        let class = if options.two_tone && i == 0 && mark.layers.len() > 1 {
            "a"
        } else {
            "m"
        };
        let body = single.svg_body(3, None);
        // Inject the class on each <path>; the paint comes from the stylesheet.
        paths.push_str(&body.replace("<path ", &format!("<path class=\"{class}\" ")));
    }
    let mut svg = String::new();
    let _ = write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\">\
<style>.b{{fill:{}}}.m{{fill:{}}}.a{{fill:{}}}@media (prefers-color-scheme:dark){{.b{{fill:{}}}.m{{fill:{}}}.a{{fill:{}}}}}</style>\
<rect class=\"b\" width=\"24\" height=\"24\" rx=\"{}\"/><g transform=\"{}\">{paths}</g></svg>",
        l.background,
        l.foreground,
        l.accent,
        d.background,
        d.foreground,
        d.accent,
        format_number(radius, 3),
        mark_transform(mark, mark_scale)
    );
    svg
}

/// Rasterizes an SVG document to a square PNG of side `size`.
///
/// # Errors
/// [`Error::Svg`] or [`Error::Render`].
pub fn render_png(svg: &str, size: u32) -> Result<Vec<u8>, Error> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
        .map_err(|e| Error::Svg(e.to_string()))?;
    let mut pixmap =
        Pixmap::new(size, size).ok_or_else(|| Error::Render(format!("invalid size {size}")))?;
    let side = tree.size().width().max(tree.size().height());
    let k = size as f32 / side;
    resvg::render(&tree, Transform::from_scale(k, k), &mut pixmap.as_mut());
    pixmap
        .encode_png()
        .map_err(|e| Error::Render(e.to_string()))
}

/// Writes an ICO container holding PNG-compressed images (`(side, png_bytes)`), smallest first.
#[must_use]
pub fn write_ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let count = u16::try_from(images.len()).expect("ICO holds at most 65535 images");
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // type 1 = icon
    out.extend_from_slice(&count.to_le_bytes());
    let mut offset = 6 + 16 * u32::from(count);
    for (side, png) in images {
        let dim = if *side >= 256 { 0u8 } else { *side as u8 };
        out.push(dim); // width
        out.push(dim); // height
        out.push(0); // palette colors
        out.push(0); // reserved
        out.extend_from_slice(&1u16.to_le_bytes()); // color planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        let len = u32::try_from(png.len()).expect("image fits in u32");
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += len;
    }
    for (_, png) in images {
        out.extend_from_slice(png);
    }
    out
}

/// One ICO directory entry read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcoEntry {
    pub width: u32,
    pub height: u32,
    pub bits_per_pixel: u16,
    pub offset: usize,
    pub len: usize,
}

/// Parses an ICO directory and checks that every payload lies inside the file.
///
/// # Errors
/// [`Error::Verify`] on malformed headers.
pub fn read_ico(bytes: &[u8]) -> Result<Vec<IcoEntry>, Error> {
    let u16_at = |i: usize| {
        bytes
            .get(i..i + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let u32_at = |i: usize| {
        bytes
            .get(i..i + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let bad = |m: &str| Error::Verify(format!("ico: {m}"));
    if u16_at(0) != Some(0) || u16_at(2) != Some(1) {
        return Err(bad("not an icon directory"));
    }
    let count = usize::from(u16_at(4).ok_or_else(|| bad("truncated header"))?);
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let base = 6 + 16 * i;
        let w = *bytes.get(base).ok_or_else(|| bad("truncated entry"))?;
        let h = *bytes.get(base + 1).ok_or_else(|| bad("truncated entry"))?;
        let bpp = u16_at(base + 6).ok_or_else(|| bad("truncated entry"))?;
        let len = u32_at(base + 8).ok_or_else(|| bad("truncated entry"))? as usize;
        let offset = u32_at(base + 12).ok_or_else(|| bad("truncated entry"))? as usize;
        if offset.checked_add(len).is_none_or(|end| end > bytes.len()) {
            return Err(bad("payload out of bounds"));
        }
        entries.push(IcoEntry {
            width: if w == 0 { 256 } else { u32::from(w) },
            height: if h == 0 { 256 } else { u32::from(h) },
            bits_per_pixel: bpp,
            offset,
            len,
        });
    }
    Ok(entries)
}

fn write_file(
    dir: &Path,
    name: &str,
    bytes: &[u8],
    kind: &'static str,
    size: Option<u32>,
) -> Result<OutputFile, Error> {
    let path = dir.join(name);
    std::fs::write(&path, bytes).map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;
    Ok(OutputFile {
        path: name.to_owned(),
        bytes: bytes.len() as u64,
        kind,
        size,
    })
}

/// The W3C web app manifest.
#[must_use]
pub fn manifest(options: &FaviconOptions) -> String {
    let icons = serde_json::json!([
        { "src": "icon-192.png", "sizes": "192x192", "type": "image/png", "purpose": "any" },
        { "src": "icon-512.png", "sizes": "512x512", "type": "image/png", "purpose": "any" },
        { "src": "icon-maskable-192.png", "sizes": "192x192", "type": "image/png", "purpose": "maskable" },
        { "src": "icon-maskable-512.png", "sizes": "512x512", "type": "image/png", "purpose": "maskable" },
        { "src": "favicon.svg", "sizes": "any", "type": "image/svg+xml" }
    ]);
    let value = serde_json::json!({
        "name": options.name,
        "short_name": options.short_name,
        "start_url": options.start_url,
        "display": "standalone",
        "theme_color": options.light.theme_color,
        "background_color": options.light.surface,
        "icons": icons,
    });
    serde_json::to_string_pretty(&value).expect("manifest serializes")
}

/// Generates the full favicon set into `out`.
///
/// # Errors
/// I/O, SVG or render errors.
pub fn generate(mark: &Icon, options: &FaviconOptions, out: &Path) -> Result<FaviconReport, Error> {
    std::fs::create_dir_all(out).map_err(|e| Error::Io(format!("{}: {e}", out.display())))?;
    let mut files = Vec::new();
    let light = &options.light;

    let small = compose(mark, light, Shape::SMALL, options.two_tone);
    let any = compose(mark, light, Shape::ANY, options.two_tone);
    let apple = compose(mark, light, Shape::APPLE, options.two_tone);
    let maskable = compose(mark, light, Shape::MASKABLE, options.two_tone);

    let mut ico_images = Vec::new();
    for size in ICO_SIZES {
        let png = render_png(&small, size)?;
        files.push(write_file(
            out,
            &format!("favicon-{size}.png"),
            &png,
            "png",
            Some(size),
        )?);
        ico_images.push((size, png));
    }
    files.push(write_file(
        out,
        "favicon.ico",
        &write_ico(&ico_images),
        "ico",
        None,
    )?);
    files.push(write_file(
        out,
        "favicon.svg",
        favicon_svg(mark, options).as_bytes(),
        "svg",
        None,
    )?);
    files.push(write_file(
        out,
        "apple-touch-icon.png",
        &render_png(&apple, 180)?,
        "png",
        Some(180),
    )?);
    for size in [192, 512] {
        files.push(write_file(
            out,
            &format!("icon-{size}.png"),
            &render_png(&any, size)?,
            "png",
            Some(size),
        )?);
        files.push(write_file(
            out,
            &format!("icon-maskable-{size}.png"),
            &render_png(&maskable, size)?,
            "png",
            Some(size),
        )?);
    }
    files.push(write_file(
        out,
        "manifest.webmanifest",
        manifest(options).as_bytes(),
        "webmanifest",
        None,
    )?);
    Ok(FaviconReport {
        palette: options.light.clone(),
        dark_palette: options.dark.clone(),
        files,
    })
}

/// What [`verify_dir`] checked.
#[derive(Debug, Clone, Serialize)]
pub struct VerifyReport {
    pub pngs: Vec<(String, u32, u32)>,
    pub ico_entries: Vec<(u32, u32)>,
    pub svg_ok: bool,
    pub manifest_icons: usize,
}

/// Decodes every output of [`generate`] back and checks sizes, ICO structure, SVG parse and manifest references.
///
/// # Errors
/// [`Error::Verify`] describing the first failure.
pub fn verify_dir(dir: &Path) -> Result<VerifyReport, Error> {
    let read = |name: &str| {
        std::fs::read(dir.join(name)).map_err(|e| Error::Verify(format!("{name}: {e}")))
    };
    let mut pngs = Vec::new();
    let expect = [
        ("favicon-16.png", 16),
        ("favicon-32.png", 32),
        ("favicon-48.png", 48),
        ("apple-touch-icon.png", 180),
        ("icon-192.png", 192),
        ("icon-512.png", 512),
        ("icon-maskable-192.png", 192),
        ("icon-maskable-512.png", 512),
    ];
    for (name, side) in expect {
        let pixmap =
            Pixmap::decode_png(&read(name)?).map_err(|e| Error::Verify(format!("{name}: {e}")))?;
        if pixmap.width() != side || pixmap.height() != side {
            return Err(Error::Verify(format!(
                "{name}: {}x{} != {side}",
                pixmap.width(),
                pixmap.height()
            )));
        }
        pngs.push((name.to_owned(), pixmap.width(), pixmap.height()));
    }

    let ico = read("favicon.ico")?;
    let entries = read_ico(&ico)?;
    let mut ico_entries = Vec::new();
    for entry in &entries {
        let payload = &ico[entry.offset..entry.offset + entry.len];
        let pixmap =
            Pixmap::decode_png(payload).map_err(|e| Error::Verify(format!("ico payload: {e}")))?;
        if pixmap.width() != entry.width || pixmap.height() != entry.height {
            return Err(Error::Verify(
                "ico payload size differs from its directory entry".to_owned(),
            ));
        }
        ico_entries.push((entry.width, entry.height));
    }
    if ico_entries.iter().map(|e| e.0).collect::<Vec<_>>() != ICO_SIZES.to_vec() {
        return Err(Error::Verify(format!("ico sizes {ico_entries:?}")));
    }

    let svg = read("favicon.svg")?;
    usvg::Tree::from_data(&svg, &usvg::Options::default())
        .map_err(|e| Error::Verify(format!("favicon.svg: {e}")))?;

    let manifest: serde_json::Value = serde_json::from_slice(&read("manifest.webmanifest")?)
        .map_err(|e| Error::Verify(format!("manifest: {e}")))?;
    let icons = manifest["icons"]
        .as_array()
        .ok_or_else(|| Error::Verify("manifest: no icons".to_owned()))?;
    for icon in icons {
        let src = icon["src"]
            .as_str()
            .ok_or_else(|| Error::Verify("manifest icon without src".to_owned()))?;
        if !dir.join(src).is_file() {
            return Err(Error::Verify(format!(
                "manifest references missing `{src}`"
            )));
        }
    }
    if !icons.iter().any(|i| i["purpose"] == "maskable") {
        return Err(Error::Verify("manifest has no maskable icon".to_owned()));
    }
    Ok(VerifyReport {
        pngs,
        ico_entries,
        svg_ok: true,
        manifest_icons: icons.len(),
    })
}
