use super::*;
use crate::image::ImagePixels;

#[test]
fn text_hash_is_sha256_of_utf8_bytes() {
    // sha256("abc")
    let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    assert_eq!(ContentHash::of_text("abc").to_hex(), expected);
}

#[test]
fn cap_004_whitespace_differences_produce_different_hashes() {
    assert_ne!(ContentHash::of_text("abc"), ContentHash::of_text("abc "));
}

#[test]
fn image_hash_depends_on_pixels_and_dimensions() {
    let a = ImagePixels::new(2, 1, vec![0; 8]).unwrap();
    let same = ImagePixels::new(2, 1, vec![0; 8]).unwrap();
    let transposed = ImagePixels::new(1, 2, vec![0; 8]).unwrap();

    assert_eq!(ContentHash::of_image(&a), ContentHash::of_image(&same));
    assert_ne!(
        ContentHash::of_image(&a),
        ContentHash::of_image(&transposed)
    );
}

#[test]
fn hex_round_trips_and_rejects_invalid_input() {
    let hash = ContentHash::of_text("x");

    assert_eq!(ContentHash::from_hex(&hash.to_hex()), Ok(hash));
    assert_eq!(ContentHash::from_hex("zz"), Err(HashError::InvalidHex));
    assert_eq!(
        ContentHash::from_hex(&"0".repeat(63)),
        Err(HashError::InvalidHex)
    );
}

#[test]
fn bytes_round_trip() {
    let hash = ContentHash::of_text("x");

    assert_eq!(ContentHash::from_bytes(*hash.as_bytes()), hash);
}
