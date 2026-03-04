use bc_ur::{MultipartEncoder, UR};

use crate::{
    Color, CorrectionLevel, Error, Logo, Result,
    qr_matrix::{QrMatrix, check_qr_density},
    render::render_from_matrix,
    render::RenderedImage,
};

/// Parameters for multipart animated QR generation.
pub struct AnimateParams {
    /// Maximum fragment length for fountain coding (default
    /// 100).
    pub max_fragment_len: usize,
    /// Error correction level. `None` = auto: Low without
    /// logo, Quartile with logo.
    pub correction: Option<CorrectionLevel>,
    /// Target image size in pixels (default 512).
    pub size: u32,
    /// Foreground color (default black).
    pub foreground: Color,
    /// Background color (default white).
    pub background: Color,
    /// Quiet zone modules around the QR code (default 1).
    pub quiet_zone: u32,
    /// Optional logo overlay.
    pub logo: Option<Logo>,
    /// Frames per second (default 8.0).
    pub fps: f64,
    /// Number of complete cycles through all fragments
    /// (default 3).
    pub cycles: u32,
    /// If set, use exactly this many frames instead of
    /// `parts_count * cycles`. Returns `InsufficientFrames`
    /// if fewer than the fountain-coded fragment count.
    pub frame_count: Option<usize>,
    /// If set, check each frame's QR module count against
    /// this limit. Returns `QrCodeTooDense` if exceeded.
    pub max_modules: Option<usize>,
}

impl Default for AnimateParams {
    fn default() -> Self {
        Self {
            max_fragment_len: 100,
            correction: None,
            size: 512,
            foreground: Color::BLACK,
            background: Color::WHITE,
            quiet_zone: 1,
            logo: None,
            fps: 8.0,
            cycles: 3,
            frame_count: None,
            max_modules: None,
        }
    }
}

impl AnimateParams {
    fn effective_correction(&self) -> CorrectionLevel {
        self.correction.unwrap_or(if self.logo.is_some() {
            CorrectionLevel::High
        } else {
            CorrectionLevel::Low
        })
    }
}

/// A single frame of a multipart QR animation.
pub struct QrFrame {
    /// The rendered RGBA image for this frame.
    pub image: RenderedImage,
    /// The part index (0-based).
    pub index: usize,
}

/// Generate all frames for a multipart UR animation.
///
/// Cycles through the fountain-coded parts `params.cycles`
/// times.
pub fn generate_frames(
    ur: &UR,
    params: &AnimateParams,
) -> Result<Vec<QrFrame>> {
    let mut encoder =
        MultipartEncoder::new(ur, params.max_fragment_len)?;
    let parts_count = encoder.parts_count();
    let total_frames = if let Some(n) = params.frame_count {
        n
    } else {
        parts_count * params.cycles as usize
    };

    // Validate frame count is sufficient for decoding.
    if total_frames < parts_count {
        return Err(Error::InsufficientFrames {
            requested: total_frames,
            fragments: parts_count,
        });
    }

    let correction = params.effective_correction();
    let mut frames = Vec::with_capacity(total_frames);

    for i in 0..total_frames {
        let part = encoder.next_part()?;
        let index = encoder.current_index();
        let upper = part.to_ascii_uppercase();
        let matrix =
            QrMatrix::encode(upper.as_bytes(), correction)?;

        // Check density on first frame (all frames use the
        // same QR version for a given fragment length).
        if i == 0 {
            if let Some(limit) = params.max_modules {
                check_qr_density(matrix.width(), limit)?;
            }
        }

        let image = render_from_matrix(
            &matrix,
            params.size,
            params.foreground,
            params.background,
            params.quiet_zone,
            params.logo.as_ref(),
        )?;
        frames.push(QrFrame { image, index });
    }

    Ok(frames)
}

/// Encode frames into an animated GIF.
///
/// For QR codes without logos, uses a small global palette
/// (2–4 colors). For QR codes with logos, uses per-frame
/// quantization.
pub fn encode_animated_gif(
    frames: &[QrFrame],
    fps: f64,
) -> Result<Vec<u8>> {
    if frames.is_empty() {
        return Err(Error::InvalidParameter(
            "no frames to encode".into(),
        ));
    }

    let width = frames[0].image.width as u16;
    let height = frames[0].image.height as u16;
    let delay_cs = (100.0 / fps).round() as u16; // GIF delay in centiseconds

    let mut buf = Vec::new();
    {
        let mut encoder = gif::Encoder::new(
            &mut buf,
            width,
            height,
            &[],
        )
        .map_err(|e| {
            Error::GifEncode(format!("GIF init: {e}"))
        })?;

        encoder
            .set_repeat(gif::Repeat::Infinite)
            .map_err(|e| {
                Error::GifEncode(format!(
                    "GIF set repeat: {e}"
                ))
            })?;

        for frame_data in frames {
            let rgba = &frame_data.image.pixels;
            let (palette, indexed) =
                quantize_frame(rgba, width as u32, height as u32);

            let mut frame = gif::Frame {
                width,
                height,
                delay: delay_cs,
                palette: Some(palette),
                ..Default::default()
            };
            frame.buffer =
                std::borrow::Cow::Owned(indexed);

            encoder.write_frame(&frame).map_err(|e| {
                Error::GifEncode(format!(
                    "GIF write frame: {e}"
                ))
            })?;
        }
    }

    Ok(buf)
}

/// Write frames as numbered PNG files.
pub fn write_frame_pngs(
    frames: &[QrFrame],
    output_dir: &std::path::Path,
) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;
    for (i, frame) in frames.iter().enumerate() {
        let path =
            output_dir.join(format!("{:04}.png", i));
        let png = frame.image.to_png()?;
        std::fs::write(&path, &png)?;
    }
    Ok(())
}

/// Quantize an RGBA frame to a 256-color indexed palette.
///
/// Uses median-cut quantization via the `image` crate's
/// color quantization.
fn quantize_frame(
    rgba: &[u8],
    width: u32,
    height: u32,
) -> (Vec<u8>, Vec<u8>) {
    // Collect unique colors (up to limit)
    let mut unique_colors: Vec<[u8; 4]> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for px in rgba.chunks_exact(4) {
        let key = [px[0], px[1], px[2], px[3]];
        if seen.insert(key) {
            unique_colors.push(key);
            if unique_colors.len() > 256 {
                break;
            }
        }
    }

    if unique_colors.len() <= 256 {
        // Simple case: few enough colors for a direct palette
        let palette: Vec<u8> = unique_colors
            .iter()
            .flat_map(|c| [c[0], c[1], c[2]])
            .collect();

        let indexed: Vec<u8> = rgba
            .chunks_exact(4)
            .map(|px| {
                unique_colors
                    .iter()
                    .position(|c| {
                        c[0] == px[0]
                            && c[1] == px[1]
                            && c[2] == px[2]
                            && c[3] == px[3]
                    })
                    .unwrap_or(0) as u8
            })
            .collect();

        (palette, indexed)
    } else {
        // Many colors (logo present) — use NeuQuant
        let nq = color_quant::NeuQuant::new(
            10,
            256,
            rgba,
        );
        let palette: Vec<u8> = (0..256)
            .flat_map(|i| {
                if let Some(c) = nq.lookup(i) {
                    [c[0], c[1], c[2]]
                } else {
                    [0, 0, 0]
                }
            })
            .collect();

        let indexed: Vec<u8> = rgba
            .chunks_exact(4)
            .map(|px| nq.index_of(px) as u8)
            .collect();

        let _ = (width, height); // used by signature

        (palette, indexed)
    }
}
