use bc_mur::{
    AnimateParams, Color, CorrectionLevel, DEFAULT_MAX_MODULES, Logo,
    LogoClearShape,
};

const TEST_SVG: &[u8] = include_bytes!("test_data/bc-logo.svg");

// A short UR string that fits in a single QR frame.
const SHORT_UR: &str = "ur:bytes/hdcxdwinvezm";

/// Build a valid UR with a large CBOR payload that requires
/// multipart encoding.
fn long_ur() -> bc_ur::UR {
    // 500 bytes of deterministic data, wrapped as CBOR
    let data: Vec<u8> = (0u16..500).map(|i| (i % 256) as u8).collect();
    let cbor: dcbor::CBOR = data.into();
    bc_ur::UR::new("bytes", cbor).unwrap()
}

// ─── Single-frame rendering ────────────────────────────

#[test]
fn single_frame_png_dimensions() {
    let img = bc_mur::render_ur_qr(
        SHORT_UR,
        CorrectionLevel::Low,
        512,
        Color::BLACK,
        Color::WHITE,
        1,
        None,
    )
    .unwrap();
    assert_eq!(img.width, 512);
    assert_eq!(img.height, 512);

    let png = img.to_png().unwrap();
    assert!(png.len() > 100); // valid PNG, non-trivial
    assert_eq!(&png[..4], &[137, 80, 78, 71]); // PNG magic
}

#[test]
fn single_frame_jpeg() {
    let img = bc_mur::render_ur_qr(
        SHORT_UR,
        CorrectionLevel::Medium,
        256,
        Color::BLACK,
        Color::WHITE,
        1,
        None,
    )
    .unwrap();
    let jpeg = img.to_jpeg(85).unwrap();
    // JPEG magic: FF D8 FF
    assert_eq!(&jpeg[..3], &[0xFF, 0xD8, 0xFF]);
}

#[test]
fn single_frame_custom_colors() {
    let img = bc_mur::render_qr(
        b"HELLO",
        CorrectionLevel::High,
        128,
        Color::from_hex("#0000FF").unwrap(),
        Color::from_hex("#FFFF00").unwrap(),
        1,
        None,
    )
    .unwrap();
    assert_eq!(img.width, 128);
    // Verify some pixels have the expected foreground color
    let has_blue = img
        .pixels
        .chunks_exact(4)
        .any(|px| px[0] == 0 && px[1] == 0 && px[2] == 255);
    assert!(has_blue, "expected blue foreground pixels");
}

#[test]
fn single_frame_dark_mode() {
    let img = bc_mur::render_ur_qr(
        SHORT_UR,
        CorrectionLevel::Low,
        256,
        Color::WHITE,
        Color::BLACK,
        1,
        None,
    )
    .unwrap();
    // Corner pixel should be background (black) — quiet zone
    assert_eq!(img.pixels[0], 0); // R
    assert_eq!(img.pixels[1], 0); // G
    assert_eq!(img.pixels[2], 0); // B
}

#[test]
fn single_frame_quiet_zone_0() {
    // No quiet zone — all modules touch the edge
    let img = bc_mur::render_qr(
        b"HELLO",
        CorrectionLevel::Low,
        256,
        Color::BLACK,
        Color::WHITE,
        0,
        None,
    )
    .unwrap();
    assert_eq!(img.width, 256);
}

#[test]
fn single_frame_quiet_zone_4() {
    // Wide quiet zone
    let img = bc_mur::render_qr(
        b"HELLO",
        CorrectionLevel::Low,
        512,
        Color::BLACK,
        Color::WHITE,
        4,
        None,
    )
    .unwrap();
    assert_eq!(img.width, 512);
    // Corner pixel should be background (white)
    assert_eq!(img.pixels[0], 255);
}

// ─── Logo overlay ──────────────────────────────────────

#[test]
fn single_frame_with_svg_logo() {
    let logo =
        Logo::from_svg(TEST_SVG, 0.25, 1, LogoClearShape::Square).unwrap();
    assert_eq!(logo.width, 512);
    assert_eq!(logo.height, 512);

    let img = bc_mur::render_ur_qr(
        SHORT_UR,
        CorrectionLevel::High,
        512,
        Color::BLACK,
        Color::WHITE,
        1,
        Some(&logo),
    )
    .unwrap();

    let png = img.to_png().unwrap();
    assert!(png.len() > 100);
}

#[test]
fn single_frame_with_circle_logo() {
    let logo =
        Logo::from_svg(TEST_SVG, 0.30, 2, LogoClearShape::Circle).unwrap();

    let img = bc_mur::render_qr(
        b"UR:BYTES/TEST",
        CorrectionLevel::High,
        256,
        Color::BLACK,
        Color::WHITE,
        1,
        Some(&logo),
    )
    .unwrap();
    assert_eq!(img.width, 256);
}

// ─── Animated multipart ────────────────────────────────

#[test]
fn animated_gif_basic() {
    let ur = long_ur();

    let params = AnimateParams {
        max_fragment_len: 50,
        size: 256,
        cycles: 2,
        fps: 4.0,
        ..Default::default()
    };

    let frames = bc_mur::generate_frames(&ur, &params).unwrap();
    assert!(frames.len() >= 2, "expected multiple frames");

    let gif = bc_mur::encode_animated_gif(&frames, 4.0).unwrap();
    // GIF magic: GIF89a
    assert_eq!(&gif[..6], b"GIF89a");
    assert!(gif.len() > 100);
}

#[test]
fn animated_gif_with_logo() {
    let ur = long_ur();

    let logo =
        Logo::from_svg(TEST_SVG, 0.20, 1, LogoClearShape::Square).unwrap();

    let params = AnimateParams {
        max_fragment_len: 50,
        size: 256,
        cycles: 1,
        fps: 4.0,
        logo: Some(logo),
        ..Default::default()
    };

    let frames = bc_mur::generate_frames(&ur, &params).unwrap();
    let gif = bc_mur::encode_animated_gif(&frames, 4.0).unwrap();
    assert_eq!(&gif[..6], b"GIF89a");
}

#[test]
fn frame_dump() {
    let ur = long_ur();

    let params = AnimateParams {
        max_fragment_len: 50,
        size: 128,
        cycles: 1,
        fps: 4.0,
        ..Default::default()
    };

    let frames = bc_mur::generate_frames(&ur, &params).unwrap();

    let tmp = tempfile::tempdir().unwrap();
    bc_mur::write_frame_pngs(&frames, tmp.path()).unwrap();

    // Verify at least one PNG file was written
    let entries: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), frames.len());
}

// ─── Error cases ───────────────────────────────────────

#[test]
fn invalid_color_hex() {
    assert!(Color::from_hex("#ZZZZZZ").is_err());
}

#[test]
fn logo_fraction_out_of_range() {
    assert!(Logo::from_svg(TEST_SVG, 0.0, 1, LogoClearShape::Square,).is_err());

    assert!(Logo::from_svg(TEST_SVG, 1.0, 1, LogoClearShape::Square,).is_err());
}

// ─── Density check ────────────────────────────────────

#[test]
fn qr_module_count_small() {
    let count =
        bc_mur::qr_module_count(b"HELLO", CorrectionLevel::Low).unwrap();
    assert_eq!(count, 21); // Version 1
}

#[test]
fn check_qr_density_passes() {
    bc_mur::check_qr_density(21, DEFAULT_MAX_MODULES).unwrap();
}

#[test]
fn check_qr_density_fails() {
    let err = bc_mur::check_qr_density(150, 117).unwrap_err();
    match err {
        bc_mur::Error::QrCodeTooDense { module_count, max_modules } => {
            assert_eq!(module_count, 150);
            assert_eq!(max_modules, 117);
        }
        other => panic!("expected QrCodeTooDense, got {other}"),
    }
}

#[test]
fn density_check_on_dense_qr() {
    // Generate enough data to exceed version 25 (117 modules)
    // at Low correction. ~1000 bytes of uppercase UR data
    // should push well beyond version 25.
    let data: Vec<u8> = (0u16..1000).map(|i| (i % 256) as u8).collect();
    let cbor: dcbor::CBOR = data.into();
    let ur = bc_ur::UR::new("bytes", cbor).unwrap();
    let ur_string = ur.qr_string();
    let upper = ur_string.to_ascii_uppercase();
    let modules =
        bc_mur::qr_module_count(upper.as_bytes(), CorrectionLevel::Low)
            .unwrap();
    assert!(
        modules > DEFAULT_MAX_MODULES,
        "expected dense QR ({modules} modules), \
         but it fits within {DEFAULT_MAX_MODULES}"
    );
    let err =
        bc_mur::check_qr_density(modules, DEFAULT_MAX_MODULES).unwrap_err();
    assert!(matches!(err, bc_mur::Error::QrCodeTooDense { .. }));
}

// ─── Insufficient frames ─────────────────────────────

#[test]
fn insufficient_frames_error() {
    let ur = long_ur();
    let params = AnimateParams {
        max_fragment_len: 50,
        frame_count: Some(1), // fewer than parts_count
        ..Default::default()
    };
    let result = bc_mur::generate_frames(&ur, &params);
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(
        matches!(err, bc_mur::Error::InsufficientFrames { .. }),
        "expected InsufficientFrames, got {err}"
    );
}

#[test]
fn frame_count_exact() {
    let ur = long_ur();
    // Use enough frames (>= parts_count) — should succeed.
    let params = AnimateParams {
        max_fragment_len: 50,
        frame_count: Some(100),
        ..Default::default()
    };
    let frames = bc_mur::generate_frames(&ur, &params).unwrap();
    assert_eq!(frames.len(), 100);
}

#[test]
fn animate_density_check() {
    let ur = long_ur();
    let params = AnimateParams {
        max_fragment_len: 500, // one huge fragment → dense QR
        max_modules: Some(21), // absurdly tight limit
        ..Default::default()
    };
    let result = bc_mur::generate_frames(&ur, &params);
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(
        matches!(err, bc_mur::Error::QrCodeTooDense { .. }),
        "expected QrCodeTooDense, got {err}"
    );
}

// ─── Write test outputs for user review ────────────────

#[test]
fn write_test_outputs() {
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let logo =
        Logo::from_svg(TEST_SVG, 0.25, 1, LogoClearShape::Square).unwrap();
    let circle_logo =
        Logo::from_svg(TEST_SVG, 0.25, 1, LogoClearShape::Circle).unwrap();

    // ── Light mode (default) ──

    // Single frame, no logo
    let img = bc_mur::render_ur_qr(
        SHORT_UR,
        CorrectionLevel::Low,
        512,
        Color::BLACK,
        Color::WHITE,
        1,
        None,
    )
    .unwrap();
    std::fs::write(out_dir.join("single-no-logo.png"), img.to_png().unwrap())
        .unwrap();

    // Single frame, with square logo
    let img = bc_mur::render_ur_qr(
        SHORT_UR,
        CorrectionLevel::High,
        512,
        Color::BLACK,
        Color::WHITE,
        1,
        Some(&logo),
    )
    .unwrap();
    std::fs::write(out_dir.join("single-with-logo.png"), img.to_png().unwrap())
        .unwrap();

    // Single frame, circle logo
    let img = bc_mur::render_ur_qr(
        SHORT_UR,
        CorrectionLevel::High,
        512,
        Color::BLACK,
        Color::WHITE,
        1,
        Some(&circle_logo),
    )
    .unwrap();
    std::fs::write(
        out_dir.join("single-circle-logo.png"),
        img.to_png().unwrap(),
    )
    .unwrap();

    // ── Dark mode ──

    // Single frame, dark, no logo
    let img = bc_mur::render_ur_qr(
        SHORT_UR,
        CorrectionLevel::Low,
        512,
        Color::WHITE,
        Color::BLACK,
        1,
        None,
    )
    .unwrap();
    std::fs::write(
        out_dir.join("single-dark-no-logo.png"),
        img.to_png().unwrap(),
    )
    .unwrap();

    // Single frame, dark, with square logo
    let img = bc_mur::render_ur_qr(
        SHORT_UR,
        CorrectionLevel::High,
        512,
        Color::WHITE,
        Color::BLACK,
        1,
        Some(&logo),
    )
    .unwrap();
    std::fs::write(
        out_dir.join("single-dark-with-logo.png"),
        img.to_png().unwrap(),
    )
    .unwrap();

    // ── Quiet zone variations ──

    // No quiet zone
    let img = bc_mur::render_ur_qr(
        SHORT_UR,
        CorrectionLevel::Low,
        512,
        Color::BLACK,
        Color::WHITE,
        0,
        None,
    )
    .unwrap();
    std::fs::write(out_dir.join("single-qz0.png"), img.to_png().unwrap())
        .unwrap();

    // Wide quiet zone (4 modules)
    let img = bc_mur::render_ur_qr(
        SHORT_UR,
        CorrectionLevel::Low,
        512,
        Color::BLACK,
        Color::WHITE,
        4,
        None,
    )
    .unwrap();
    std::fs::write(out_dir.join("single-qz4.png"), img.to_png().unwrap())
        .unwrap();

    // Dark mode with wide quiet zone and logo
    let img = bc_mur::render_ur_qr(
        SHORT_UR,
        CorrectionLevel::High,
        512,
        Color::WHITE,
        Color::BLACK,
        4,
        Some(&logo),
    )
    .unwrap();
    std::fs::write(
        out_dir.join("single-dark-qz4-logo.png"),
        img.to_png().unwrap(),
    )
    .unwrap();

    // ── Animated ──

    let ur = long_ur();
    let params = AnimateParams {
        max_fragment_len: 50,
        size: 512,
        cycles: 2,
        fps: 8.0,
        ..Default::default()
    };
    let frames = bc_mur::generate_frames(&ur, &params).unwrap();
    let gif = bc_mur::encode_animated_gif(&frames, 8.0).unwrap();
    std::fs::write(out_dir.join("animated.gif"), &gif).unwrap();

    // Animated with logo
    let params_logo = AnimateParams {
        max_fragment_len: 50,
        size: 512,
        cycles: 2,
        fps: 8.0,
        logo: Some(logo.clone()),
        ..Default::default()
    };
    let frames_logo = bc_mur::generate_frames(&ur, &params_logo).unwrap();
    let gif_logo = bc_mur::encode_animated_gif(&frames_logo, 8.0).unwrap();
    std::fs::write(out_dir.join("animated-logo.gif"), &gif_logo).unwrap();

    // Animated with circle logo
    let params_circle_logo = AnimateParams {
        max_fragment_len: 50,
        size: 512,
        cycles: 2,
        fps: 8.0,
        logo: Some(circle_logo),
        ..Default::default()
    };
    let frames_circle_logo =
        bc_mur::generate_frames(&ur, &params_circle_logo).unwrap();
    let gif_circle_logo =
        bc_mur::encode_animated_gif(&frames_circle_logo, 8.0).unwrap();
    std::fs::write(out_dir.join("animated-circle-logo.gif"), &gif_circle_logo)
        .unwrap();

    // Animated dark mode with logo
    let params_dark_logo = AnimateParams {
        max_fragment_len: 50,
        size: 512,
        cycles: 2,
        fps: 8.0,
        foreground: Color::WHITE,
        background: Color::BLACK,
        logo: Some(logo.clone()),
        ..Default::default()
    };
    let frames_dark_logo =
        bc_mur::generate_frames(&ur, &params_dark_logo).unwrap();
    let gif_dark_logo =
        bc_mur::encode_animated_gif(&frames_dark_logo, 8.0).unwrap();
    std::fs::write(out_dir.join("animated-dark-logo.gif"), &gif_dark_logo)
        .unwrap();

    // Animated dark mode
    let params_dark = AnimateParams {
        max_fragment_len: 50,
        size: 512,
        cycles: 2,
        fps: 8.0,
        foreground: Color::WHITE,
        background: Color::BLACK,
        ..Default::default()
    };
    let frames_dark = bc_mur::generate_frames(&ur, &params_dark).unwrap();
    let gif_dark = bc_mur::encode_animated_gif(&frames_dark, 8.0).unwrap();
    std::fs::write(out_dir.join("animated-dark.gif"), &gif_dark).unwrap();

    // ProRes 4444 (requires ffmpeg on PATH)
    let prores_path = out_dir.join("animated.mov");
    bc_mur::encode_prores(&frames, 8.0, &prores_path).unwrap();
    assert!(prores_path.exists());
    assert!(std::fs::metadata(&prores_path).unwrap().len() > 100);
}
