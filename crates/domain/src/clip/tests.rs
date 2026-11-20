use super::*;
use crate::hash::ContentHash;
use crate::ids::ClipId;
use crate::image::{FileKey, ImageMeta};
use crate::text::ClipText;
use crate::time::Timestamp;

fn image_meta() -> ImageMeta {
    ImageMeta {
        file_key: FileKey::from_hash(&ContentHash::of_text("img")),
        width: 1920,
        height: 1080,
        file_size: 123_456,
        thumb_width: 320,
        thumb_height: 180,
    }
}

#[test]
fn content_reports_kind_and_byte_size() {
    let text = ClipContent::Text(ClipText::new("olá").unwrap());
    let image = ClipContent::Image(image_meta());

    assert_eq!((text.kind(), text.byte_size()), (ClipKind::Text, 4));
    assert_eq!(
        (image.kind(), image.byte_size()),
        (ClipKind::Image, 123_456)
    );
}

#[test]
fn pin_state_converts_from_and_to_the_stored_flag() {
    assert_eq!(PinState::from_flag(true), PinState::Pinned);
    assert_eq!(PinState::from_flag(false), PinState::Unpinned);
    assert!(PinState::Pinned.is_pinned());
    assert_eq!(PinState::Pinned.toggled(), PinState::Unpinned);
    assert_eq!(PinState::Unpinned.toggled(), PinState::Pinned);
}

#[test]
fn item_exposes_its_retention_view() {
    let item = ClipItem {
        id: ClipId::new(3),
        content: ClipContent::Text(ClipText::new("x").unwrap()),
        hash: ContentHash::of_text("x"),
        source_app: None,
        pinned: true,
        use_count: 2,
        created_at: Timestamp::from_millis(1),
        last_used_at: Timestamp::from_millis(5),
    };

    let candidate = item.retention_candidate();

    assert_eq!(candidate.id, ClipId::new(3));
    assert_eq!(candidate.last_used_at, Timestamp::from_millis(5));
    assert!(candidate.pinned);
}
