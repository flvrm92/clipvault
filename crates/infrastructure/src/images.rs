//! [`ImageStore`] on the file system (ADR-0004, specs/05 §4).
//!
//! Layout: `<root>/<shard>/<key>.png` and `<root>/<shard>/<key>.thumb.png`, where the shard is
//! the first two characters of the key. Writes are atomic: encode to `*.tmp`, `fsync`, rename.
//! The thumbnail is written before the original, so an original on disk implies a complete
//! pair; [`ImageStore::keys`] lists originals only.

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use clipvault_application::ports::{ImageStore, PortError, StoredImage};
use clipvault_domain::image::{FileKey, ImagePixels, thumbnail_size};
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{ExtendedColorType, ImageEncoder, RgbaImage};

const ORIGINAL_SUFFIX: &str = ".png";
const THUMBNAIL_SUFFIX: &str = ".thumb.png";
const TEMP_EXTENSION: &str = "tmp";

fn io_error(context: &str, error: &std::io::Error) -> PortError {
    PortError::Storage(format!("{context}: {error}"))
}

fn image_error(error: &image::ImageError) -> PortError {
    PortError::Storage(format!("image encoding failed: {error}"))
}

/// Images stored as PNG files under a root folder.
#[derive(Debug, Clone)]
pub struct FsImageStore {
    root: PathBuf,
}

impl FsImageStore {
    /// Opens the store, creating `root` if needed and deleting temporary files left by an
    /// interrupted save (REL-002).
    ///
    /// # Errors
    /// I/O failures.
    pub fn open(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        for shard in fs::read_dir(&root)?.flatten().filter(|e| e.path().is_dir()) {
            for file in fs::read_dir(shard.path())?.flatten() {
                let path = file.path();
                if path.extension().is_some_and(|e| e == TEMP_EXTENSION) {
                    fs::remove_file(&path)?;
                }
            }
        }
        Ok(Self { root })
    }

    /// Path of the original image (served to the UI through the `clip://` protocol, SEC-003).
    pub fn image_path(&self, key: &FileKey) -> PathBuf {
        self.shard_dir(key)
            .join(format!("{}{ORIGINAL_SUFFIX}", key.as_str()))
    }

    /// Path of the thumbnail.
    pub fn thumbnail_path(&self, key: &FileKey) -> PathBuf {
        self.shard_dir(key)
            .join(format!("{}{THUMBNAIL_SUFFIX}", key.as_str()))
    }

    fn shard_dir(&self, key: &FileKey) -> PathBuf {
        self.root.join(key.shard())
    }
}

/// Encodes `image` as PNG into `path` atomically.
fn write_png_atomically(path: &Path, image: &RgbaImage) -> Result<u64, PortError> {
    let mut temp = path.as_os_str().to_owned();
    temp.push(format!(".{TEMP_EXTENSION}"));
    let temp = PathBuf::from(temp);

    let result = (|| {
        let file = File::create(&temp).map_err(|e| io_error("creating image file", &e))?;
        let mut writer = BufWriter::new(file);
        PngEncoder::new(&mut writer)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgba8,
            )
            .map_err(|e| image_error(&e))?;
        let file = writer
            .into_inner()
            .map_err(|e| io_error("writing image file", e.error()))?;
        file.sync_all()
            .map_err(|e| io_error("flushing image file", &e))?;
        let size = file
            .metadata()
            .map_err(|e| io_error("reading image size", &e))?
            .len();
        drop(file);
        fs::rename(&temp, path).map_err(|e| io_error("moving image file", &e))?;
        Ok(size)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn remove_if_present(path: &Path) -> Result<(), PortError> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(io_error("removing image file", &error))
        }
        _ => Ok(()),
    }
}

impl ImageStore for FsImageStore {
    fn save(&self, key: &FileKey, pixels: &ImagePixels) -> Result<StoredImage, PortError> {
        fs::create_dir_all(self.shard_dir(key))
            .map_err(|e| io_error("creating image folder", &e))?;
        let original = RgbaImage::from_raw(pixels.width(), pixels.height(), pixels.rgba().to_vec())
            .ok_or_else(|| PortError::Storage("pixel buffer does not match size".into()))?;

        let (thumb_width, thumb_height) = thumbnail_size(pixels.width(), pixels.height());
        let thumbnail = if (thumb_width, thumb_height) == (pixels.width(), pixels.height()) {
            original.clone()
        } else {
            image::imageops::resize(&original, thumb_width, thumb_height, FilterType::Triangle)
        };

        write_png_atomically(&self.thumbnail_path(key), &thumbnail)?;
        let file_size = write_png_atomically(&self.image_path(key), &original)?;
        Ok(StoredImage {
            file_size,
            thumb_width,
            thumb_height,
        })
    }

    fn remove(&self, key: &FileKey) -> Result<(), PortError> {
        remove_if_present(&self.image_path(key))?;
        remove_if_present(&self.thumbnail_path(key))
    }

    fn load(&self, key: &FileKey) -> Result<ImagePixels, PortError> {
        let path = self.image_path(key);
        if !path.exists() {
            return Err(PortError::NotFound);
        }
        let decoded = image::open(&path).map_err(|e| image_error(&e))?.to_rgba8();
        let (width, height) = decoded.dimensions();
        ImagePixels::new(width, height, decoded.into_raw())
            .map_err(|e| PortError::Storage(format!("invalid stored image: {e}")))
    }

    fn keys(&self) -> Result<Vec<FileKey>, PortError> {
        let read = |dir: &Path| fs::read_dir(dir).map_err(|e| io_error("listing images", &e));
        let mut keys = Vec::new();
        for shard in read(&self.root)?.flatten().filter(|e| e.path().is_dir()) {
            for file in read(&shard.path())?.flatten() {
                let name = file.file_name();
                let Some(name) = name.to_str() else { continue };
                if name.ends_with(THUMBNAIL_SUFFIX) {
                    continue;
                }
                let key = name
                    .strip_suffix(ORIGINAL_SUFFIX)
                    .and_then(|stem| FileKey::parse(stem).ok());
                if let Some(key) = key.filter(|k| shard.file_name() == k.shard()) {
                    keys.push(key);
                }
            }
        }
        keys.sort();
        Ok(keys)
    }
}
