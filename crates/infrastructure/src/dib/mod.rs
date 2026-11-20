//! Device-independent bitmaps (`CF_DIB` / `CF_DIBV5`) ↔ RGBA pixels. Pure code: no Win32 calls,
//! so it is tested on any platform.
//!
//! Supported: 24- and 32-bit uncompressed (`BI_RGB`) and bit-field (`BI_BITFIELDS`,
//! `BI_ALPHABITFIELDS`) bitmaps, top-down or bottom-up, with `BITMAPINFOHEADER` or later
//! headers. Palette-based and compressed (RLE, JPEG, PNG) bitmaps are rejected; the clipboard
//! adapter then falls back to another format.

use clipvault_domain::image::ImagePixels;

const INFO_HEADER_SIZE: usize = 40;
const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;
const BI_ALPHABITFIELDS: u32 = 6;

/// Why a bitmap could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DibError {
    /// The buffer is shorter than its header or pixel data require.
    #[error("bitmap data is truncated")]
    Truncated,
    /// Zero or out-of-range dimensions.
    #[error("bitmap dimensions are invalid")]
    InvalidDimensions,
    /// Bit depth or compression not supported.
    #[error("bitmap format is not supported")]
    Unsupported,
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, DibError> {
    bytes
        .get(at..at + 2)
        .and_then(|b| b.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or(DibError::Truncated)
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, DibError> {
    bytes
        .get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or(DibError::Truncated)
}

fn i32_at(bytes: &[u8], at: usize) -> Result<i32, DibError> {
    u32_at(bytes, at).map(|v| i32::from_le_bytes(v.to_le_bytes()))
}

/// A channel described by a bit mask (e.g. `0x00FF0000` for red in BGRA).
#[derive(Debug, Clone, Copy)]
struct Channel {
    mask: u32,
}

impl Channel {
    /// Extracts the channel scaled to 0..=255; `None` when the mask is empty.
    fn extract(self, pixel: u32) -> Option<u8> {
        if self.mask == 0 {
            return None;
        }
        let shift = self.mask.trailing_zeros();
        let max = self.mask >> shift;
        let value = (pixel & self.mask) >> shift;
        u8::try_from(value * 255 / max).ok()
    }
}

#[derive(Debug, Clone, Copy)]
struct Masks {
    red: Channel,
    green: Channel,
    blue: Channel,
    alpha: Channel,
}

impl Masks {
    const BGRX: Self = Self {
        red: Channel { mask: 0x00FF_0000 },
        green: Channel { mask: 0x0000_FF00 },
        blue: Channel { mask: 0x0000_00FF },
        alpha: Channel { mask: 0 },
    };
}

/// Decodes a packed DIB (header, optional masks/color table, pixel rows) to RGBA.
///
/// # Errors
/// [`DibError`] for truncated, invalid or unsupported bitmaps.
pub fn decode_dib(bytes: &[u8]) -> Result<ImagePixels, DibError> {
    let header_size = u32_at(bytes, 0)? as usize;
    if header_size < INFO_HEADER_SIZE || header_size > bytes.len() {
        return Err(DibError::Truncated);
    }
    let width = i32_at(bytes, 4)?;
    let height = i32_at(bytes, 8)?;
    let bit_count = u16_at(bytes, 14)?;
    let compression = u32_at(bytes, 16)?;
    let colors_used = u32_at(bytes, 32)? as usize;

    let (Ok(width), Some(abs_height)) = (u32::try_from(width), height.checked_abs()) else {
        return Err(DibError::InvalidDimensions);
    };
    let height_px = u32::try_from(abs_height).map_err(|_| DibError::InvalidDimensions)?;
    if width == 0 || height_px == 0 {
        return Err(DibError::InvalidDimensions);
    }
    let top_down = height < 0;

    let mut pixel_offset = header_size;
    let masks = match (bit_count, compression) {
        (24 | 32, BI_RGB) => Masks::BGRX,
        (32, BI_BITFIELDS | BI_ALPHABITFIELDS) => {
            // Masks live inside V2+ headers, otherwise right after the basic header.
            let masks_at = if header_size >= 52 {
                40
            } else {
                pixel_offset += if compression == BI_ALPHABITFIELDS {
                    16
                } else {
                    12
                };
                header_size
            };
            let has_alpha = header_size >= 56 || compression == BI_ALPHABITFIELDS;
            Masks {
                red: Channel {
                    mask: u32_at(bytes, masks_at)?,
                },
                green: Channel {
                    mask: u32_at(bytes, masks_at + 4)?,
                },
                blue: Channel {
                    mask: u32_at(bytes, masks_at + 8)?,
                },
                alpha: Channel {
                    mask: if has_alpha {
                        u32_at(bytes, masks_at + 12)?
                    } else {
                        0
                    },
                },
            }
        }
        _ => return Err(DibError::Unsupported),
    };
    pixel_offset += colors_used * 4;

    let bytes_per_pixel = usize::from(bit_count / 8);
    let (w, h) = (width as usize, height_px as usize);
    let stride = (w * usize::from(bit_count)).div_ceil(32) * 4;
    let data = bytes
        .get(pixel_offset..)
        .filter(|d| d.len() >= stride * h)
        .ok_or(DibError::Truncated)?;

    let mut rgba = Vec::with_capacity(w * h * 4);
    let mut any_alpha = false;
    for y in 0..h {
        let source_row = if top_down { y } else { h - 1 - y };
        let row = &data[source_row * stride..source_row * stride + w * bytes_per_pixel];
        for px in row.chunks_exact(bytes_per_pixel) {
            let value = px
                .iter()
                .rev()
                .fold(0_u32, |acc, b| (acc << 8) | u32::from(*b));
            let alpha = if bit_count == 32 && compression == BI_RGB {
                px[3] // BI_RGB stores an unofficial alpha byte; validated below
            } else {
                masks.alpha.extract(value).unwrap_or(255)
            };
            any_alpha |= alpha != 0;
            rgba.extend_from_slice(&[
                masks.red.extract(value).unwrap_or(0),
                masks.green.extract(value).unwrap_or(0),
                masks.blue.extract(value).unwrap_or(0),
                alpha,
            ]);
        }
    }
    // A fully transparent 32-bit BI_RGB bitmap means "no alpha channel": make it opaque.
    if bit_count == 32 && compression == BI_RGB && !any_alpha {
        rgba.as_chunks_mut::<4>()
            .0
            .iter_mut()
            .for_each(|px| px[3] = 255);
    }
    ImagePixels::new(width, height_px, rgba).map_err(|_| DibError::InvalidDimensions)
}

/// Encodes RGBA pixels as a bottom-up 32-bit `BI_RGB` DIB with a `BITMAPINFOHEADER` — the most
/// widely accepted `CF_DIB` layout. The alpha byte is kept in the fourth position.
pub fn encode_dib(pixels: &ImagePixels) -> Vec<u8> {
    let (w, h) = (pixels.width() as usize, pixels.height() as usize);
    let mut dib = Vec::with_capacity(INFO_HEADER_SIZE + w * h * 4);
    let image_size = u32::try_from(w * h * 4).unwrap_or(u32::MAX);
    dib.extend_from_slice(&40_u32.to_le_bytes());
    dib.extend_from_slice(&pixels.width().to_le_bytes());
    dib.extend_from_slice(&pixels.height().to_le_bytes());
    dib.extend_from_slice(&1_u16.to_le_bytes()); // planes
    dib.extend_from_slice(&32_u16.to_le_bytes()); // bit count
    dib.extend_from_slice(&BI_RGB.to_le_bytes());
    dib.extend_from_slice(&image_size.to_le_bytes());
    dib.extend_from_slice(&[0; 16]); // resolution, colors used, colors important
    for row in pixels.rgba().chunks_exact(w * 4).rev() {
        for px in row.as_chunks::<4>().0 {
            dib.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }
    }
    dib
}

#[cfg(test)]
mod tests;
