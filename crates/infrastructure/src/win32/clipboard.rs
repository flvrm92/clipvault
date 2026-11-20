//! [`ClipboardReader`] and [`ClipboardWriter`] over the Win32 clipboard (CAP-001..011, PST-003).

use std::io::Cursor;

use clipvault_application::ports::{
    ClipboardContent, ClipboardReader, ClipboardSequence, ClipboardSnapshot, ClipboardWriter,
    PortError, PrivacyFlags,
};
use clipvault_domain::image::ImagePixels;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, ImageFormat};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber,
    IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::Win32::System::Ole::{CF_DIB, CF_DIBV5, CF_UNICODETEXT};
use windows::core::w;

use super::owner::clipboard_owner;
use super::window::MessageWindow;
use crate::dib::{decode_dib, encode_dib};
use crate::retry::RetryPolicy;

fn unavailable(what: &str, error: &windows::core::Error) -> PortError {
    PortError::Unavailable(format!("{what}: {error}"))
}

/// Holds the clipboard open; closes it on drop.
struct OpenClipboardGuard;

impl OpenClipboardGuard {
    /// Opens the clipboard, retrying while another process holds it (REL-003).
    fn open(owner: Option<HWND>) -> Result<Self, PortError> {
        RetryPolicy::CLIPBOARD
            .run(std::thread::sleep, || {
                // SAFETY: `owner` is either None or a live window owned by the caller.
                unsafe { OpenClipboard(owner) }
            })
            .map(|()| Self)
            .map_err(|e| unavailable("clipboard is locked by another application", &e))
    }
}

impl Drop for OpenClipboardGuard {
    fn drop(&mut self) {
        // SAFETY: this guard exists only while the clipboard is open by this thread.
        let _ = unsafe { CloseClipboard() };
    }
}

/// Copies the bytes of a clipboard global memory block.
fn global_bytes(handle: HANDLE) -> Option<Vec<u8>> {
    let memory = HGLOBAL(handle.0);
    // SAFETY: `memory` was returned by GetClipboardData while the clipboard is open; the
    // block stays valid until the clipboard is closed. Lock/unlock are paired.
    unsafe {
        let size = GlobalSize(memory);
        let ptr = GlobalLock(memory).cast::<u8>();
        if ptr.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
        let _ = GlobalUnlock(memory);
        Some(bytes)
    }
}

/// Places `bytes` on the (open, emptied) clipboard under `format`.
fn set_global(format: u32, bytes: &[u8]) -> Result<(), PortError> {
    // SAFETY: the block is allocated with the exact size and filled before being handed to
    // the clipboard, which takes ownership on success; on failure it is freed here.
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1))
            .map_err(|e| unavailable("cannot allocate clipboard memory", &e))?;
        let ptr = GlobalLock(memory).cast::<u8>();
        if ptr.is_null() {
            let _ = GlobalFree(Some(memory));
            return Err(PortError::Unavailable(
                "cannot lock clipboard memory".into(),
            ));
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        let _ = GlobalUnlock(memory);
        if let Err(error) = SetClipboardData(format, Some(HANDLE(memory.0))) {
            let _ = GlobalFree(Some(memory));
            return Err(unavailable("cannot set clipboard data", &error));
        }
    }
    Ok(())
}

fn encode_png(pixels: &ImagePixels) -> Result<Vec<u8>, PortError> {
    let mut png = Vec::new();
    PngEncoder::new(Cursor::new(&mut png))
        .write_image(
            pixels.rgba(),
            pixels.width(),
            pixels.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| PortError::Storage(format!("cannot encode image: {e}")))?;
    Ok(png)
}

/// The Windows clipboard.
#[derive(Debug, Clone, Copy)]
pub struct Win32Clipboard {
    png: u32,
    exclude_from_monitor: u32,
    viewer_ignore: u32,
    can_include_in_history: u32,
}

impl Default for Win32Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Win32Clipboard {
    /// Registers the custom clipboard formats used by ClipVault and by privacy-aware apps.
    pub fn new() -> Self {
        // SAFETY: format names are static, null-terminated wide strings.
        unsafe {
            Self {
                png: RegisterClipboardFormatW(w!("PNG")),
                exclude_from_monitor: RegisterClipboardFormatW(w!(
                    "ExcludeClipboardContentFromMonitorProcessing"
                )),
                viewer_ignore: RegisterClipboardFormatW(w!("Clipboard Viewer Ignore")),
                can_include_in_history: RegisterClipboardFormatW(w!(
                    "CanIncludeInClipboardHistory"
                )),
            }
        }
    }

    fn data(format: u32) -> Option<Vec<u8>> {
        // SAFETY: called with the clipboard open; a missing format returns an error.
        unsafe { IsClipboardFormatAvailable(format) }.ok()?;
        // SAFETY: as above.
        let handle = unsafe { GetClipboardData(format) }.ok()?;
        global_bytes(handle)
    }

    fn has(format: u32) -> bool {
        // SAFETY: called with the clipboard open.
        unsafe { IsClipboardFormatAvailable(format) }.is_ok()
    }

    fn privacy(self) -> PrivacyFlags {
        let history_disallowed = Self::data(self.can_include_in_history)
            .and_then(|bytes| bytes.get(..4)?.try_into().ok().map(u32::from_le_bytes))
            == Some(0);
        PrivacyFlags {
            exclude_from_monitor: Self::has(self.exclude_from_monitor),
            viewer_ignore: Self::has(self.viewer_ignore),
            history_disallowed,
        }
    }

    fn text() -> Option<String> {
        let bytes = Self::data(u32::from(CF_UNICODETEXT.0))?;
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .take_while(|unit| *unit != 0)
            .collect();
        Some(String::from_utf16_lossy(&units))
    }

    /// PNG first (keeps transparency), then `CF_DIBV5`, then `CF_DIB` (CAP-002).
    fn image(self) -> Option<ImagePixels> {
        if let Some(png) = Self::data(self.png) {
            match image::load_from_memory_with_format(&png, ImageFormat::Png) {
                Ok(decoded) => {
                    let rgba = decoded.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    if let Ok(pixels) = ImagePixels::new(w, h, rgba.into_raw()) {
                        return Some(pixels);
                    }
                }
                Err(error) => tracing::debug!(%error, "clipboard PNG could not be decoded"),
            }
        }
        [CF_DIBV5, CF_DIB].into_iter().find_map(|format| {
            let dib = Self::data(u32::from(format.0))?;
            decode_dib(&dib)
                .inspect_err(|error| tracing::debug!(%error, "clipboard bitmap skipped"))
                .ok()
        })
    }
}

impl ClipboardReader for Win32Clipboard {
    fn snapshot(&self) -> Result<ClipboardSnapshot, PortError> {
        let guard = OpenClipboardGuard::open(None)?;
        // SAFETY: no preconditions.
        let sequence = ClipboardSequence::new(unsafe { GetClipboardSequenceNumber() });
        let privacy = self.privacy();
        // Private content is never read (CAP-007); an image is only decoded when there is no
        // usable text, since text takes precedence (CAP-003).
        let (text, image) = if privacy.any() {
            (None, None)
        } else {
            let text = Self::text().filter(|t| !t.trim().is_empty());
            let image = if text.is_none() { self.image() } else { None };
            (text, image)
        };
        drop(guard);
        Ok(ClipboardSnapshot {
            sequence,
            text,
            image,
            privacy,
            owner: clipboard_owner(),
        })
    }
}

impl ClipboardWriter for Win32Clipboard {
    fn write(&self, content: &ClipboardContent) -> Result<ClipboardSequence, PortError> {
        // EmptyClipboard needs an owner window, otherwise SetClipboardData fails.
        let owner = MessageWindow::create()
            .map_err(|e| unavailable("cannot create clipboard owner window", &e))?;
        let guard = OpenClipboardGuard::open(Some(owner.hwnd()))?;
        // SAFETY: the clipboard is open by this thread.
        unsafe { EmptyClipboard() }.map_err(|e| unavailable("cannot empty clipboard", &e))?;
        match content {
            ClipboardContent::Text(text) => {
                let bytes: Vec<u8> = text
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .flat_map(u16::to_le_bytes)
                    .collect();
                set_global(u32::from(CF_UNICODETEXT.0), &bytes)?;
            }
            ClipboardContent::Image(pixels) => {
                set_global(self.png, &encode_png(pixels)?)?;
                set_global(u32::from(CF_DIB.0), &encode_dib(pixels))?;
            }
        }
        drop(guard);
        // SAFETY: no preconditions.
        Ok(ClipboardSequence::new(unsafe {
            GetClipboardSequenceNumber()
        }))
    }
}
