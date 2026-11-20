use super::*;

const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;

/// A `BITMAPINFOHEADER`-compatible header of `size` bytes (40 or 124).
fn header(size: u32, width: i32, height: i32, bit_count: u16, compression: u32) -> Vec<u8> {
    // At least 40 bytes are written so the fields exist even for a (rejected) 12-byte header.
    let mut h = vec![0_u8; (size as usize).max(40)];
    h[0..4].copy_from_slice(&size.to_le_bytes());
    h[4..8].copy_from_slice(&width.to_le_bytes());
    h[8..12].copy_from_slice(&height.to_le_bytes());
    h[12..14].copy_from_slice(&1_u16.to_le_bytes());
    h[14..16].copy_from_slice(&bit_count.to_le_bytes());
    h[16..20].copy_from_slice(&compression.to_le_bytes());
    h
}

fn rgba(width: u32, height: u32, pixels: &[[u8; 4]]) -> ImagePixels {
    ImagePixels::new(width, height, pixels.concat()).unwrap()
}

#[test]
fn encoded_dibs_decode_back_to_the_same_pixels_including_alpha() {
    let original = rgba(
        2,
        2,
        &[
            [255, 0, 0, 255],
            [0, 255, 0, 128],
            [0, 0, 255, 64],
            [9, 8, 7, 0],
        ],
    );

    assert_eq!(decode_dib(&encode_dib(&original)).unwrap(), original);
}

#[test]
fn encoded_dib_is_a_bottom_up_32_bit_bgra_bitmap() {
    let original = rgba(1, 2, &[[1, 2, 3, 4], [5, 6, 7, 8]]);

    let dib = encode_dib(&original);

    assert_eq!(&dib[0..4], &40_u32.to_le_bytes());
    assert_eq!(
        &dib[8..12],
        &2_i32.to_le_bytes(),
        "positive height = bottom-up"
    );
    assert_eq!(&dib[14..16], &32_u16.to_le_bytes());
    assert_eq!(
        &dib[40..],
        &[7, 6, 5, 8, 3, 2, 1, 4],
        "last row first, BGRA"
    );
}

#[test]
fn top_down_bitmaps_keep_their_row_order() {
    let mut dib = header(40, 1, -2, 32, BI_RGB);
    dib.extend_from_slice(&[3, 2, 1, 255, 6, 5, 4, 255]);

    let pixels = decode_dib(&dib).unwrap();

    assert_eq!(pixels.rgba(), &[1, 2, 3, 255, 4, 5, 6, 255]);
}

#[test]
fn rgb_32_bit_bitmaps_with_an_empty_alpha_channel_are_opaque() {
    let mut dib = header(40, 2, 1, 32, BI_RGB);
    dib.extend_from_slice(&[10, 20, 30, 0, 40, 50, 60, 0]);

    assert_eq!(
        decode_dib(&dib).unwrap().rgba(),
        &[30, 20, 10, 255, 60, 50, 40, 255]
    );
}

#[test]
fn rows_of_24_bit_bitmaps_are_padded_to_four_bytes() {
    // width 3 → 9 bytes per row, padded to 12.
    let mut dib = header(40, 3, 1, 24, BI_RGB);
    dib.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 0, 0]);

    assert_eq!(
        decode_dib(&dib).unwrap().rgba(),
        &[3, 2, 1, 255, 6, 5, 4, 255, 9, 8, 7, 255]
    );
}

#[test]
fn bitfield_masks_after_a_basic_header_are_honoured() {
    let mut dib = header(40, 1, 1, 32, BI_BITFIELDS);
    for mask in [0x0000_00FF_u32, 0x0000_FF00, 0x00FF_0000] {
        dib.extend_from_slice(&mask.to_le_bytes()); // R, G, B masks: RGBX byte order
    }
    dib.extend_from_slice(&[11, 22, 33, 0]);

    assert_eq!(decode_dib(&dib).unwrap().rgba(), &[11, 22, 33, 255]);
}

#[test]
fn v5_headers_provide_rgba_masks() {
    let mut dib = header(124, 1, 1, 32, BI_BITFIELDS);
    dib[40..44].copy_from_slice(&0x00FF_0000_u32.to_le_bytes());
    dib[44..48].copy_from_slice(&0x0000_FF00_u32.to_le_bytes());
    dib[48..52].copy_from_slice(&0x0000_00FF_u32.to_le_bytes());
    dib[52..56].copy_from_slice(&0xFF00_0000_u32.to_le_bytes());
    dib.extend_from_slice(&[30, 20, 10, 77]);

    assert_eq!(decode_dib(&dib).unwrap().rgba(), &[10, 20, 30, 77]);
}

#[test]
fn a_color_table_is_skipped() {
    let mut dib = header(40, 1, 1, 32, BI_RGB);
    dib[32..36].copy_from_slice(&2_u32.to_le_bytes()); // biClrUsed = 2
    dib.extend_from_slice(&[0; 8]);
    dib.extend_from_slice(&[3, 2, 1, 255]);

    assert_eq!(decode_dib(&dib).unwrap().rgba(), &[1, 2, 3, 255]);
}

#[test]
fn malformed_bitmaps_are_rejected() {
    assert_eq!(decode_dib(&[0; 10]), Err(DibError::Truncated));
    assert_eq!(
        decode_dib(&header(40, 2, 2, 32, BI_RGB)),
        Err(DibError::Truncated)
    );
    assert_eq!(
        decode_dib(&header(40, 0, 2, 32, BI_RGB)),
        Err(DibError::InvalidDimensions)
    );
    assert_eq!(
        decode_dib(&header(40, 2, 2, 8, BI_RGB)),
        Err(DibError::Unsupported)
    );
    assert_eq!(
        decode_dib(&header(40, 2, 2, 32, 1)),
        Err(DibError::Unsupported),
        "RLE"
    );
    assert_eq!(
        decode_dib(&header(12, 2, 2, 32, BI_RGB)),
        Err(DibError::Truncated),
        "OS/2 core header"
    );
}
