use image::{ImageEncoder, codecs::png::PngEncoder, codecs::jpeg::JpegEncoder};

use crate::{
    Color, CorrectionLevel, Error, Logo, Result,
    qr_matrix::QrMatrix,
};

/// An RGBA pixel buffer with encoding methods.
pub struct RenderedImage {
    /// RGBA pixels, row-major, 4 bytes per pixel.
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl RenderedImage {
    /// Encode as PNG.
    pub fn to_png(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        PngEncoder::new(&mut buf)
            .write_image(
                &self.pixels,
                self.width,
                self.height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| Error::ImageEncode(e.to_string()))?;
        Ok(buf)
    }

    /// Encode as JPEG at the given quality (1–100).
    pub fn to_jpeg(&self, quality: u8) -> Result<Vec<u8>> {
        // JPEG doesn't support alpha — convert to RGB
        let rgb: Vec<u8> = self
            .pixels
            .chunks_exact(4)
            .flat_map(|px| [px[0], px[1], px[2]])
            .collect();
        let mut buf = Vec::new();
        JpegEncoder::new_with_quality(&mut buf, quality)
            .write_image(
                &rgb,
                self.width,
                self.height,
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| Error::ImageEncode(e.to_string()))?;
        Ok(buf)
    }
}

/// Render a single-frame QR code from raw bytes.
///
/// - `message`: bytes to encode in the QR code
/// - `correction`: error correction level
/// - `size`: target image size in pixels (square)
/// - `fg` / `bg`: foreground and background colors
/// - `quiet_zone`: number of background-colored modules
///   around the QR code (default 1)
/// - `logo`: optional logo overlay
pub fn render_qr(
    message: &[u8],
    correction: CorrectionLevel,
    size: u32,
    fg: Color,
    bg: Color,
    quiet_zone: u32,
    logo: Option<&Logo>,
) -> Result<RenderedImage> {
    let matrix = QrMatrix::encode(message, correction)?;
    render_from_matrix(&matrix, size, fg, bg, quiet_zone, logo)
}

/// Render a single-frame QR code from a UR string.
///
/// The UR string is automatically uppercased for QR
/// alphanumeric mode efficiency.
pub fn render_ur_qr(
    ur_string: &str,
    correction: CorrectionLevel,
    size: u32,
    fg: Color,
    bg: Color,
    quiet_zone: u32,
    logo: Option<&Logo>,
) -> Result<RenderedImage> {
    let upper = ur_string.to_ascii_uppercase();
    render_qr(
        upper.as_bytes(),
        correction,
        size,
        fg,
        bg,
        quiet_zone,
        logo,
    )
}

/// Paint the QR matrix into a pixel buffer with module-aligned
/// rendering, then composite the logo if present.
pub(crate) fn render_from_matrix(
    matrix: &QrMatrix,
    size: u32,
    fg: Color,
    bg: Color,
    quiet_zone: u32,
    logo: Option<&Logo>,
) -> Result<RenderedImage> {
    let qr_modules = matrix.width();
    let total_modules = qr_modules + 2 * quiet_zone as usize;
    let pixels_per_module =
        (size as usize / total_modules).max(1);
    let compositing_size = total_modules * pixels_per_module;
    let qz_px = quiet_zone as usize * pixels_per_module;

    // Allocate RGBA buffer, fill with background
    let mut pixels =
        vec![0u8; compositing_size * compositing_size * 4];
    // Fill entire buffer with background color
    for px in pixels.chunks_exact_mut(4) {
        px[0] = bg.r;
        px[1] = bg.g;
        px[2] = bg.b;
        px[3] = bg.a;
    }

    // Paint QR modules offset by quiet zone
    for row in 0..qr_modules {
        for col in 0..qr_modules {
            let color = if matrix.is_dark(col, row) {
                fg
            } else {
                bg
            };
            let px = qz_px + col * pixels_per_module;
            let py = qz_px + row * pixels_per_module;
            fill_rect(
                &mut pixels,
                compositing_size,
                px,
                py,
                pixels_per_module,
                pixels_per_module,
                color,
            );
        }
    }

    // Logo overlay (centered within the QR modules area)
    if let Some(logo) = logo {
        composite_logo(
            &mut pixels,
            compositing_size,
            qr_modules,
            pixels_per_module,
            qz_px,
            bg,
            logo,
        );
    }

    // Scale to final requested size if different
    let pixels = if compositing_size as u32 != size {
        nearest_neighbor_scale(
            &pixels,
            compositing_size as u32,
            compositing_size as u32,
            size,
            size,
        )
    } else {
        pixels
    };

    Ok(RenderedImage { pixels, width: size, height: size })
}

/// Fill a rectangle in the RGBA buffer.
fn fill_rect(
    pixels: &mut [u8],
    stride: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: Color,
) {
    for row in y..y + h {
        for col in x..x + w {
            let offset = (row * stride + col) * 4;
            pixels[offset] = color.r;
            pixels[offset + 1] = color.g;
            pixels[offset + 2] = color.b;
            pixels[offset + 3] = color.a;
        }
    }
}

/// Composite the logo into the center of the QR code.
///
/// `qz_px` is the quiet-zone offset in pixels so the logo
/// centers on the QR data area, not the entire image.
fn composite_logo(
    pixels: &mut [u8],
    compositing_size: usize,
    module_count: usize,
    pixels_per_module: usize,
    qz_px: usize,
    bg: Color,
    logo: &Logo,
) {
    let layout = LogoLayout::new(
        module_count,
        logo.fraction,
        logo.clear_border,
    );

    if layout.logo_modules == 0 {
        return;
    }

    // Clear color: use white if background is transparent
    let clear_color =
        if bg.is_transparent() { Color::WHITE } else { bg };

    let center_module = module_count as f64 / 2.0;
    let qr_px = module_count * pixels_per_module;

    // Clear the center area (offset by quiet zone)
    let start_module =
        (module_count - layout.cleared_modules) / 2;
    match logo.clear_shape {
        crate::LogoClearShape::Square => {
            let clear_pixels =
                layout.cleared_modules * pixels_per_module;
            let clear_origin =
                qz_px + (qr_px - clear_pixels) / 2;
            fill_rect(
                pixels,
                compositing_size,
                clear_origin,
                clear_origin,
                clear_pixels,
                clear_pixels,
                clear_color,
            );
        }
        crate::LogoClearShape::Circle => {
            let radius = layout.cleared_modules as f64 / 2.0;
            for row in 0..layout.cleared_modules {
                for col in 0..layout.cleared_modules {
                    let mx =
                        (start_module + col) as f64 + 0.5;
                    let my =
                        (start_module + row) as f64 + 0.5;
                    let dx = mx - center_module;
                    let dy = my - center_module;
                    if dx * dx + dy * dy <= radius * radius {
                        let px = qz_px
                            + (start_module + col)
                                * pixels_per_module;
                        let py = qz_px
                            + (start_module + row)
                                * pixels_per_module;
                        fill_rect(
                            pixels,
                            compositing_size,
                            px,
                            py,
                            pixels_per_module,
                            pixels_per_module,
                            clear_color,
                        );
                    }
                }
            }
        }
    }

    // Draw the logo centered within the QR data area
    let logo_pixels = layout.logo_modules * pixels_per_module;
    let logo_origin = qz_px + (qr_px - logo_pixels) / 2;

    // Scale logo to fit the logo area using bilinear
    // interpolation
    let scaled = bilinear_scale(
        &logo.pixels,
        logo.width,
        logo.height,
        logo_pixels as u32,
        logo_pixels as u32,
    );

    // Alpha-composite the scaled logo onto the QR
    for row in 0..logo_pixels {
        for col in 0..logo_pixels {
            let src_offset = (row * logo_pixels + col) * 4;
            let dst_x = logo_origin + col;
            let dst_y = logo_origin + row;
            let dst_offset =
                (dst_y * compositing_size + dst_x) * 4;

            let sa = scaled[src_offset + 3] as u32;
            if sa == 0 {
                continue;
            }
            if sa == 255 {
                pixels[dst_offset] = scaled[src_offset];
                pixels[dst_offset + 1] =
                    scaled[src_offset + 1];
                pixels[dst_offset + 2] =
                    scaled[src_offset + 2];
                pixels[dst_offset + 3] = 255;
            } else {
                let da = pixels[dst_offset + 3] as u32;
                let inv_sa = 255 - sa;
                let out_a = sa + da * inv_sa / 255;
                if out_a > 0 {
                    for c in 0..3 {
                        let sc =
                            scaled[src_offset + c] as u32;
                        let dc =
                            pixels[dst_offset + c] as u32;
                        pixels[dst_offset + c] = ((sc * sa
                            + dc * da * inv_sa / 255)
                            / out_a)
                            as u8;
                    }
                    pixels[dst_offset + 3] =
                        out_a.min(255) as u8;
                }
            }
        }
    }
}

/// Logo layout calculation — mirrors Swift `LogoLayout` /
/// Kotlin `LogoLayout`.
struct LogoLayout {
    logo_modules: usize,
    cleared_modules: usize,
}

impl LogoLayout {
    fn new(
        module_count: usize,
        fraction: f64,
        clear_border: usize,
    ) -> Self {
        let mut logo =
            (module_count as f64 * fraction).round() as usize;
        // Force odd for symmetry
        if logo.is_multiple_of(2) {
            logo += 1;
        }
        let mut cleared = logo + 2 * clear_border;
        // Cap cleared area at 40% of module count
        let max_cleared =
            (module_count as f64 * 0.40).floor() as usize;
        if cleared > max_cleared {
            cleared = max_cleared;
            logo = cleared.saturating_sub(2 * clear_border);
        }
        // Re-ensure odd after capping
        if logo.is_multiple_of(2) && logo > 0 {
            logo -= 1;
        }
        Self {
            logo_modules: logo,
            cleared_modules: cleared,
        }
    }
}

/// Nearest-neighbor scale for crisp QR modules.
fn nearest_neighbor_scale(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> Vec<u8> {
    let mut dst = vec![0u8; (dst_w * dst_h * 4) as usize];
    for y in 0..dst_h {
        let sy = (y * src_h / dst_h).min(src_h - 1);
        for x in 0..dst_w {
            let sx = (x * src_w / dst_w).min(src_w - 1);
            let si = (sy * src_w + sx) as usize * 4;
            let di = (y * dst_w + x) as usize * 4;
            dst[di..di + 4].copy_from_slice(&src[si..si + 4]);
        }
    }
    dst
}

/// Bilinear scale for smooth logo rendering.
fn bilinear_scale(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> Vec<u8> {
    let mut dst = vec![0u8; (dst_w * dst_h * 4) as usize];
    for y in 0..dst_h {
        let fy = y as f64 * (src_h - 1) as f64
            / (dst_h - 1).max(1) as f64;
        let y0 = fy.floor() as u32;
        let y1 = (y0 + 1).min(src_h - 1);
        let wy = fy - y0 as f64;

        for x in 0..dst_w {
            let fx = x as f64 * (src_w - 1) as f64
                / (dst_w - 1).max(1) as f64;
            let x0 = fx.floor() as u32;
            let x1 = (x0 + 1).min(src_w - 1);
            let wx = fx - x0 as f64;

            let i00 = (y0 * src_w + x0) as usize * 4;
            let i10 = (y0 * src_w + x1) as usize * 4;
            let i01 = (y1 * src_w + x0) as usize * 4;
            let i11 = (y1 * src_w + x1) as usize * 4;

            let di = (y * dst_w + x) as usize * 4;
            for c in 0..4 {
                let v = src[i00 + c] as f64 * (1.0 - wx) * (1.0 - wy)
                    + src[i10 + c] as f64 * wx * (1.0 - wy)
                    + src[i01 + c] as f64 * (1.0 - wx) * wy
                    + src[i11 + c] as f64 * wx * wy;
                dst[di + c] = v.round() as u8;
            }
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_layout_basic() {
        let l = LogoLayout::new(25, 0.25, 1);
        // 25 * 0.25 = 6.25 → round to 6 → force odd → 7
        assert_eq!(l.logo_modules, 7);
        // 7 + 2*1 = 9
        assert_eq!(l.cleared_modules, 9);
    }

    #[test]
    fn logo_layout_cap_at_40_pct() {
        // 21 * 0.40 = 8.4 → floor = 8
        let l = LogoLayout::new(21, 0.5, 2);
        // 21 * 0.5 = 10.5 → 11 (odd), cleared = 11+4=15 >
        // 8 → capped
        assert!(l.cleared_modules <= 8);
    }

    #[test]
    fn render_basic_qr() {
        let img = render_qr(
            b"HELLO",
            CorrectionLevel::Low,
            256,
            Color::BLACK,
            Color::WHITE,
            1,
            None,
        )
        .unwrap();
        assert_eq!(img.width, 256);
        assert_eq!(img.height, 256);
        assert_eq!(
            img.pixels.len(),
            256 * 256 * 4
        );
    }

    #[test]
    fn render_to_png() {
        let img = render_qr(
            b"TEST",
            CorrectionLevel::Medium,
            128,
            Color::BLACK,
            Color::WHITE,
            1,
            None,
        )
        .unwrap();
        let png = img.to_png().unwrap();
        // PNG magic bytes
        assert_eq!(&png[..4], &[137, 80, 78, 71]);
    }

    #[test]
    fn render_ur_qr_uppercases() {
        let img = render_ur_qr(
            "ur:bytes/hdcxdwinvezm",
            CorrectionLevel::Low,
            256,
            Color::BLACK,
            Color::WHITE,
            1,
            None,
        )
        .unwrap();
        assert_eq!(img.width, 256);
    }
}
