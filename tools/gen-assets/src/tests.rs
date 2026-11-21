#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::OnceLock;

use super::*;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn masters() -> &'static Masters {
    static MASTERS: OnceLock<Masters> = OnceLock::new();
    MASTERS.get_or_init(|| Masters::load(&repo()).unwrap())
}

fn all() -> &'static [Asset] {
    static ASSETS: OnceLock<Vec<Asset>> = OnceLock::new();
    ASSETS.get_or_init(|| generate(masters()).unwrap())
}

fn find(path: &str) -> &'static Asset {
    all()
        .iter()
        .find(|a| a.path == path)
        .unwrap_or_else(|| panic!("{path} not generated"))
}

fn dimensions(path: &str) -> (u32, u32) {
    let image = image::load_from_memory(&find(path).bytes).unwrap();
    (image.width(), image.height())
}

#[test]
fn generates_every_asset_of_specs_07_once() {
    let paths: BTreeSet<&str> = all().iter().map(|a| a.path.as_str()).collect();

    assert_eq!(paths.len(), all().len(), "duplicate paths");
    // 1 ico + 4 app PNGs + 4 tray + 5 Square44 scales + 14×3 targetsizes + 6 logos × 5 scales
    // + 8 Store pieces.
    assert_eq!(all().len(), 1 + 4 + 4 + 5 + 42 + 30 + 8);
}

#[test]
fn msix_logos_follow_the_scale_qualifiers() {
    assert_eq!(
        dimensions("packaging/msix/Assets/Square44x44Logo.scale-100.png"),
        (44, 44)
    );
    assert_eq!(
        dimensions("packaging/msix/Assets/Square44x44Logo.scale-400.png"),
        (176, 176)
    );
    assert_eq!(
        dimensions("packaging/msix/Assets/Square71x71Logo.scale-125.png"),
        (89, 89)
    );
    assert_eq!(
        dimensions("packaging/msix/Assets/Wide310x150Logo.scale-200.png"),
        (620, 300)
    );
    assert_eq!(
        dimensions("packaging/msix/Assets/SplashScreen.scale-150.png"),
        (930, 450)
    );
    assert_eq!(
        dimensions("packaging/msix/Assets/Square44x44Logo.targetsize-24_altform-unplated.png"),
        (24, 24)
    );
}

#[test]
fn store_art_has_the_partner_center_sizes() {
    assert_eq!(
        dimensions("packaging/store/StoreLogo-2160x2160.png"),
        (2160, 2160)
    );
    assert_eq!(
        dimensions("packaging/store/Poster-720x1080.png"),
        (720, 1080)
    );
    assert_eq!(
        dimensions("packaging/store/BoxArt-1080x1080.png"),
        (1080, 1080)
    );
    assert_eq!(
        dimensions("packaging/store/Hero-3840x2160.png"),
        (3840, 2160)
    );
}

#[test]
fn the_app_icon_holds_every_size() {
    let bytes = &find("app/icons/icon.ico").bytes;
    let count = usize::from(u16::from_le_bytes([bytes[4], bytes[5]]));
    let sizes: Vec<u32> = (0..count)
        .map(|i| match bytes[6 + i * 16] {
            0 => 256,
            w => u32::from(w),
        })
        .collect();

    assert_eq!(sizes, APP_ICO_SIZES);
}

#[test]
fn small_sizes_use_the_flat_symbol() {
    assert!(std::ptr::eq(
        masters().symbol_for(32),
        masters().flat.as_str()
    ));
    assert!(std::ptr::eq(
        masters().symbol_for(44),
        masters().symbol.as_str()
    ));
}

fn luminance(r: u8, g: u8, b: u8) -> f64 {
    let channel = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

fn contrast(a: f64, b: f64) -> f64 {
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// specs/07 §6.5: the tray glyph reaches 3:1 against its taskbar.
#[test]
fn thm_003_tray_glyphs_contrast_with_their_taskbar() {
    for (name, taskbar) in [
        ("tray-light", (0xF3, 0xF3, 0xF3)),
        ("tray-paused-light", (0xF3, 0xF3, 0xF3)),
        ("tray-dark", (0x20, 0x20, 0x20)),
        ("tray-paused-dark", (0x20, 0x20, 0x20)),
    ] {
        let image = image::load_from_memory(&find(&format!("app/icons/tray/{name}.png")).bytes)
            .unwrap()
            .to_rgba8();
        let glyph = image
            .pixels()
            .find(|p| p[3] == 255)
            .expect("an opaque pixel");
        let ratio = contrast(
            luminance(glyph[0], glyph[1], glyph[2]),
            luminance(taskbar.0, taskbar.1, taskbar.2),
        );
        assert!(ratio >= 3.0, "{name}: {ratio:.2}");
    }
}

#[test]
fn tray_states_look_different() {
    assert_ne!(
        find("app/icons/tray/tray-light.png").bytes,
        find("app/icons/tray/tray-paused-light.png").bytes
    );
}

#[test]
fn inner_strips_the_root_element() {
    let svg = "<!-- c --><svg xmlns=\"x\" viewBox=\"0 0 24 24\"><rect/></svg>\n";
    assert_eq!(inner(svg), "<rect/>");
}

#[test]
fn write_and_check_round_trip_and_report_strays() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let assets = vec![
        Asset {
            path: "app/icons/tray/a.png".into(),
            bytes: vec![1, 2, 3],
        },
        Asset {
            path: "other/b.bin".into(),
            bytes: vec![4],
        },
    ];

    assert_eq!(stale(root, &assets).len(), 2, "missing files are stale");
    assert_eq!(write(root, &assets).unwrap().len(), 2);
    assert_eq!(stale(root, &assets), Vec::<String>::new());
    assert_eq!(
        write(root, &assets).unwrap(),
        Vec::<String>::new(),
        "idempotent"
    );

    std::fs::write(root.join("other/b.bin"), [9]).unwrap();
    std::fs::write(root.join("app/icons/tray/old.png"), [0]).unwrap();
    assert_eq!(
        stale(root, &assets),
        vec![
            "other/b.bin".to_owned(),
            "app/icons/tray/old.png (not generated)".to_owned()
        ]
    );

    write(root, &assets).unwrap();
    assert_eq!(stale(root, &assets), Vec::<String>::new());
    assert!(!root.join("app/icons/tray/old.png").exists());
}
