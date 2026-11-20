use super::*;
use crate::hash::ContentHash;

#[test]
fn thumbnail_keeps_small_images_and_scales_the_longest_side_to_the_limit() {
    assert_eq!(thumbnail_size(200, 100), (200, 100));
    assert_eq!(thumbnail_size(320, 320), (320, 320));
    assert_eq!(thumbnail_size(4000, 1000), (320, 80));
    assert_eq!(thumbnail_size(1000, 4000), (80, 320));
    assert_eq!(
        thumbnail_size(10_000, 1),
        (320, 1),
        "never collapses to zero"
    );
}

#[test]
fn pixels_require_rgba_buffer_matching_dimensions() {
    assert!(ImagePixels::new(2, 2, vec![0; 16]).is_ok());
    assert_eq!(
        ImagePixels::new(2, 2, vec![0; 15]),
        Err(ImageError::BufferSizeMismatch {
            expected: 16,
            actual: 15
        })
    );
}

#[test]
fn pixels_reject_zero_dimensions() {
    assert_eq!(
        ImagePixels::new(0, 2, vec![]),
        Err(ImageError::ZeroDimension)
    );
    assert_eq!(
        ImagePixels::new(2, 0, vec![]),
        Err(ImageError::ZeroDimension)
    );
}

#[test]
fn pixels_expose_dimensions_and_size() {
    let px = ImagePixels::new(3, 2, vec![7; 24]).unwrap();

    assert_eq!((px.width(), px.height()), (3, 2));
    assert_eq!(px.byte_len(), 24);
    assert_eq!(px.rgba(), &[7; 24][..]);
}

#[test]
fn file_key_is_the_hex_hash_and_shards_by_first_two_chars() {
    let hash = ContentHash::of_text("abc");
    let key = FileKey::from_hash(&hash);

    assert_eq!(key.as_str(), hash.to_hex());
    assert_eq!(key.shard(), "ba");
}

#[test]
fn file_key_parse_accepts_only_valid_hashes() {
    let hex = ContentHash::of_text("abc").to_hex();

    assert_eq!(FileKey::parse(&hex).map(|k| k.as_str().to_owned()), Ok(hex));
    assert_eq!(
        FileKey::parse("../etc/passwd"),
        Err(ImageError::InvalidFileKey)
    );
}
