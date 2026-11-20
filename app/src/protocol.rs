//! `clip://` protocol serving image files to the UI by item id (SEC-003): the webview never
//! sees file paths. On Windows the webview requests `http://clip.localhost/<kind>/<id>`.

use clipvault_application::use_cases::GetItem;
use clipvault_domain::clip::ClipContent;
use clipvault_domain::ids::ClipId;
use tauri::http::{Response, StatusCode, header};
use tauri::{AppHandle, Manager, Wry};

use crate::container::Container;

/// Scheme name registered with the webview.
pub const SCHEME: &str = "clip";

/// Which file of an image item is requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageVariant {
    Thumbnail,
    Original,
}

/// Parses `/thumb/<id>` or `/image/<id>`.
pub fn parse_path(path: &str) -> Option<(ImageVariant, ClipId)> {
    let mut parts = path.trim_start_matches('/').split('/');
    let variant = match parts.next()? {
        "thumb" => ImageVariant::Thumbnail,
        "image" => ImageVariant::Original,
        _ => return None,
    };
    let id = parts.next()?.parse::<i64>().ok()?;
    parts.next().is_none().then_some((variant, ClipId::new(id)))
}

fn not_found() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Vec::new())
        .unwrap_or_default()
}

/// Handles one protocol request.
pub fn handle(app: &AppHandle<Wry>, path: &str) -> Response<Vec<u8>> {
    let Some((variant, id)) = parse_path(path) else {
        return not_found();
    };
    let container = app.state::<Container>();
    let Ok(item) = GetItem::new(container.history.clone()).execute(id) else {
        return not_found();
    };
    let ClipContent::Image(meta) = item.content else {
        return not_found();
    };
    let file = match variant {
        ImageVariant::Thumbnail => container.images.thumbnail_path(&meta.file_key),
        ImageVariant::Original => container.images.image_path(&meta.file_key),
    };
    match std::fs::read(file) {
        Ok(bytes) => Response::builder()
            .header(header::CONTENT_TYPE, "image/png")
            // Content of an id never changes (a new capture gets a new id).
            .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
            .body(bytes)
            .unwrap_or_else(|_| not_found()),
        Err(error) => {
            tracing::warn!(%error, "image file missing; reconciliation will clean up");
            not_found()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_003_only_well_formed_id_paths_are_accepted() {
        assert_eq!(
            parse_path("/thumb/42"),
            Some((ImageVariant::Thumbnail, ClipId::new(42)))
        );
        assert_eq!(
            parse_path("/image/7"),
            Some((ImageVariant::Original, ClipId::new(7)))
        );
        for bad in [
            "/thumb/",
            "/thumb/abc",
            "/thumb/1/extra",
            "/../../etc/passwd",
            "/file/1",
            "",
        ] {
            assert_eq!(parse_path(bad), None, "{bad}");
        }
    }
}
