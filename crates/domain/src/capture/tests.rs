use super::*;
use crate::hash::ContentHash;
use crate::image::ImagePixels;
use crate::text::ClipText;

#[test]
fn cap_004_new_clip_hashes_its_text() {
    let clip = NewClip::new(CapturedContent::Text(ClipText::new("abc").unwrap()), None);

    assert_eq!(clip.hash, ContentHash::of_text("abc"));
}

#[test]
fn cap_004_new_clip_hashes_its_image_pixels() {
    let pixels = ImagePixels::new(1, 1, vec![1, 2, 3, 4]).unwrap();
    let clip = NewClip::new(CapturedContent::Image(pixels.clone()), None);

    assert_eq!(clip.hash, ContentHash::of_image(&pixels));
}

#[test]
fn captured_content_reports_its_size() {
    let text = CapturedContent::Text(ClipText::new("olá").unwrap());
    let image = CapturedContent::Image(ImagePixels::new(2, 1, vec![0; 8]).unwrap());

    assert_eq!(text.byte_len(), 4);
    assert_eq!(image.byte_len(), 8);
}
