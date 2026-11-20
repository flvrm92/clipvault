//! Generates every brand asset from the master SVGs in `assets/brand/` (specs/07 §6).
//!
//! Output is deterministic, so `--check` can compare it byte for byte with the committed files.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::ExtendedColorType;
use image::codecs::ico::{IcoEncoder, IcoFrame};
use resvg::{tiny_skia, usvg};

/// Largest size drawn with the flat symbol (specs/07 §1).
const FLAT_MAX: u32 = 32;
/// Background of the Store art (`ink-900`).
const INK: &str = "#14131F";
/// Sizes inside the executable icon (specs/07 §6.2).
const APP_ICO_SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];
/// MSIX scale qualifiers (specs/07 §6.3).
const SCALES: [u32; 5] = [100, 125, 150, 200, 400];
/// `Square44x44Logo.targetsize-*` sizes.
const TARGET_SIZES: [u32; 14] = [16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256];
/// Folders whose whole content is generated: stray files there are reported (and removed).
pub const OWNED_DIRS: [&str; 3] = ["app/icons/tray", "packaging/msix/Assets", "packaging/store"];

/// One generated file, relative to the repository root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    /// Path relative to the repository root, with `/` separators.
    pub path: String,
    /// File content.
    pub bytes: Vec<u8>,
}

/// The master SVGs.
pub struct Masters {
    symbol: String,
    flat: String,
    tray: [(String, String); 4],
}

impl Masters {
    /// Reads the masters under `root/assets/brand`.
    ///
    /// # Errors
    /// A master is missing or unreadable.
    pub fn load(root: &Path) -> Result<Self> {
        let read = |rel: &str| {
            let path = root.join("assets/brand").join(rel);
            std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
        };
        let tray = |name: &str| -> Result<(String, String)> {
            Ok((name.to_owned(), read(&format!("tray/{name}.svg"))?))
        };
        Ok(Self {
            symbol: read("logo-symbol.svg")?,
            flat: read("logo-symbol-flat.svg")?,
            tray: [
                tray("tray-light")?,
                tray("tray-dark")?,
                tray("tray-paused-light")?,
                tray("tray-paused-dark")?,
            ],
        })
    }

    /// The symbol for an icon of `size` pixels.
    fn symbol_for(&self, size: u32) -> &str {
        if size <= FLAT_MAX {
            &self.flat
        } else {
            &self.symbol
        }
    }
}

/// The markup inside the root `<svg>` element.
fn inner(svg: &str) -> &str {
    let start = svg
        .find("<svg")
        .and_then(|i| svg[i..].find('>').map(|j| i + j + 1))
        .unwrap_or(0);
    let end = svg.rfind("</svg>").unwrap_or(svg.len());
    &svg[start..end]
}

/// A `width × height` canvas with an optional background and the 24-unit symbol centered at
/// `side` pixels.
fn compose(symbol: &str, width: u32, height: u32, side: f32, background: Option<&str>) -> String {
    #[allow(clippy::cast_precision_loss)] // asset sizes are small integers
    let (w, h) = (width as f32, height as f32);
    let fill = background.map_or_else(String::new, |c| {
        format!(r#"<rect width="{w}" height="{h}" fill="{c}"/>"#)
    });
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">{fill}<svg x="{x}" y="{y}" width="{side}" height="{side}" viewBox="0 0 24 24">{body}</svg></svg>"#,
        x = (w - side) / 2.0,
        y = (h - side) / 2.0,
        body = inner(symbol),
    )
}

/// Rasterizes `svg` to an RGBA image of `width × height`, stretching its viewBox to fit.
///
/// # Errors
/// Invalid SVG or an empty size.
pub fn render(svg: &str, width: u32, height: u32) -> Result<tiny_skia::Pixmap> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())?;
    let mut pixmap = tiny_skia::Pixmap::new(width, height).context("empty size")?;
    let view = tree.size();
    #[allow(clippy::cast_precision_loss)]
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / view.width(),
        height as f32 / view.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Ok(pixmap)
}

fn png(svg: &str, width: u32, height: u32) -> Result<Vec<u8>> {
    Ok(render(svg, width, height)?.encode_png()?)
}

/// A multi-size `.ico` with one PNG frame per size.
fn ico(masters: &Masters, sizes: &[u32]) -> Result<Vec<u8>> {
    let frames = sizes
        .iter()
        .map(|&size| {
            let bytes = png(masters.symbol_for(size), size, size)?;
            Ok(IcoFrame::with_encoded(
                bytes,
                size,
                size,
                ExtendedColorType::Rgba8,
            )?)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut out = Vec::new();
    IcoEncoder::new(&mut out).encode_images(&frames)?;
    Ok(out)
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn scaled(base: u32, scale: u32) -> u32 {
    (base as f32 * scale as f32 / 100.0).round() as u32
}

/// An MSIX logo of `width × height` at 100%: full-frame for small squares, centered otherwise.
fn msix_logo(
    masters: &Masters,
    name: &str,
    width: u32,
    height: u32,
    full: bool,
) -> Result<Vec<Asset>> {
    SCALES
        .iter()
        .map(|&scale| {
            let (w, h) = (scaled(width, scale), scaled(height, scale));
            let symbol = masters.symbol_for(w.min(h));
            #[allow(clippy::cast_precision_loss)]
            let side = if full {
                w.min(h) as f32
            } else {
                w.min(h) as f32 / 2.0
            };
            Ok(Asset {
                path: format!("packaging/msix/Assets/{name}.scale-{scale}.png"),
                bytes: png(&compose(symbol, w, h, side, None), w, h)?,
            })
        })
        .collect()
}

/// Store art: the symbol centered on `ink-900` (`share` of the shorter side), or transparent.
fn store_art(
    masters: &Masters,
    name: &str,
    width: u32,
    height: u32,
    share: f32,
    ink: bool,
) -> Result<Asset> {
    #[allow(clippy::cast_precision_loss)]
    let side = width.min(height) as f32 * share;
    let svg = compose(&masters.symbol, width, height, side, ink.then_some(INK));
    Ok(Asset {
        path: format!("packaging/store/{name}-{width}x{height}.png"),
        bytes: png(&svg, width, height)?,
    })
}

/// Every generated asset, in a stable order.
///
/// # Errors
/// Rendering or encoding failures.
pub fn generate(masters: &Masters) -> Result<Vec<Asset>> {
    let mut assets = vec![Asset {
        path: "app/icons/icon.ico".into(),
        bytes: ico(masters, &APP_ICO_SIZES)?,
    }];
    for (file, size) in [
        ("32x32.png", 32),
        ("128x128.png", 128),
        ("128x128@2x.png", 256),
        ("icon.png", 512),
    ] {
        assets.push(Asset {
            path: format!("app/icons/{file}"),
            bytes: png(masters.symbol_for(size), size, size)?,
        });
    }
    for (name, svg) in &masters.tray {
        assets.push(Asset {
            path: format!("app/icons/tray/{name}.png"),
            bytes: png(svg, 32, 32)?,
        });
    }

    assets.extend(msix_logo(masters, "Square44x44Logo", 44, 44, true)?);
    for size in TARGET_SIZES {
        let bytes = png(masters.symbol_for(size), size, size)?;
        for suffix in ["", "_altform-unplated", "_altform-lightunplated"] {
            assets.push(Asset {
                path: format!(
                    "packaging/msix/Assets/Square44x44Logo.targetsize-{size}{suffix}.png"
                ),
                bytes: bytes.clone(),
            });
        }
    }
    assets.extend(msix_logo(masters, "Square71x71Logo", 71, 71, false)?);
    assets.extend(msix_logo(masters, "Square150x150Logo", 150, 150, false)?);
    assets.extend(msix_logo(masters, "Wide310x150Logo", 310, 150, false)?);
    assets.extend(msix_logo(masters, "Square310x310Logo", 310, 310, false)?);
    assets.extend(msix_logo(masters, "StoreLogo", 50, 50, true)?);
    assets.extend(msix_logo(masters, "SplashScreen", 620, 300, false)?);

    for size in [300, 2160] {
        assets.push(store_art(masters, "StoreLogo", size, size, 1.0, false)?);
    }
    for (w, h) in [(720, 1080), (1440, 2160)] {
        assets.push(store_art(masters, "Poster", w, h, 0.5, true)?);
    }
    for size in [1080, 2160] {
        assets.push(store_art(masters, "BoxArt", size, size, 0.45, true)?);
    }
    for (w, h) in [(1920, 1080), (3840, 2160)] {
        assets.push(store_art(masters, "Hero", w, h, 0.4, true)?);
    }
    Ok(assets)
}

/// Files under the owned folders that no asset produces.
fn strays(root: &Path, assets: &[Asset]) -> Vec<String> {
    let expected: BTreeSet<&str> = assets.iter().map(|a| a.path.as_str()).collect();
    let mut found = Vec::new();
    for dir in OWNED_DIRS {
        let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            let rel = format!("{dir}/{}", entry.file_name().to_string_lossy());
            if entry.path().is_file() && !expected.contains(rel.as_str()) {
                found.push(rel);
            }
        }
    }
    found.sort();
    found
}

/// Paths that are missing, different or not generated at all (`--check`).
pub fn stale(root: &Path, assets: &[Asset]) -> Vec<String> {
    let mut out: Vec<String> = assets
        .iter()
        .filter(|a| std::fs::read(root.join(&a.path)).ok().as_deref() != Some(&a.bytes[..]))
        .map(|a| a.path.clone())
        .collect();
    out.extend(
        strays(root, assets)
            .into_iter()
            .map(|p| format!("{p} (not generated)")),
    );
    out
}

/// Writes the changed assets and deletes stray files; returns the paths written or removed.
///
/// # Errors
/// File system failures.
pub fn write(root: &Path, assets: &[Asset]) -> Result<Vec<String>> {
    let mut changed = Vec::new();
    for asset in assets {
        let path: PathBuf = root.join(&asset.path);
        if std::fs::read(&path).ok().as_deref() == Some(&asset.bytes[..]) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &asset.bytes)
            .with_context(|| format!("writing {}", path.display()))?;
        changed.push(asset.path.clone());
    }
    for stray in strays(root, assets) {
        std::fs::remove_file(root.join(&stray))?;
        changed.push(format!("{stray} (removed)"));
    }
    Ok(changed)
}

#[cfg(test)]
mod tests;
