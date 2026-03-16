use crate::{Error, Result};

/// Shape used to clear the center area behind the logo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoClearShape {
    Square,
    Circle,
}

impl std::fmt::Display for LogoClearShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Square => write!(f, "square"),
            Self::Circle => write!(f, "circle"),
        }
    }
}

impl std::str::FromStr for LogoClearShape {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "square" => Ok(Self::Square),
            "circle" => Ok(Self::Circle),
            _ => Err(format!(
                "unknown clear shape: {s} (expected square or circle)"
            )),
        }
    }
}

/// A pre-rendered logo for compositing onto QR codes.
#[derive(Clone)]
pub struct Logo {
    /// RGBA pixels, row-major.
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Fraction of QR width to occupy (0.01–0.99, default
    /// 0.25).
    pub fraction: f64,
    /// Number of clear-border modules around the logo
    /// (0–5, default 1).
    pub clear_border: usize,
    /// Shape of the cleared center area.
    pub clear_shape: LogoClearShape,
}

impl Logo {
    /// Create a logo from SVG data, rendered at 512×512 via
    /// resvg (pure Rust, no system deps).
    pub fn from_svg(
        svg_data: &[u8],
        fraction: f64,
        clear_border: usize,
        clear_shape: LogoClearShape,
    ) -> Result<Self> {
        let fraction = validate_fraction(fraction)?;
        let clear_border = validate_clear_border(clear_border)?;

        let tree = resvg::usvg::Tree::from_data(
            svg_data,
            &resvg::usvg::Options::default(),
        )
        .map_err(|e| Error::SvgRender(format!("SVG parse: {e}")))?;

        let render_size = 512u32;
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(render_size, render_size)
                .ok_or_else(|| {
                    Error::SvgRender("failed to allocate pixmap".into())
                })?;

        let svg_size = tree.size();
        let sx = render_size as f32 / svg_size.width();
        let sy = render_size as f32 / svg_size.height();
        let scale = sx.min(sy);
        let tx = (render_size as f32 - svg_size.width() * scale) / 2.0;
        let ty = (render_size as f32 - svg_size.height() * scale) / 2.0;
        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale)
            .post_translate(tx, ty);

        resvg::render(&tree, transform, &mut pixmap.as_mut());

        // resvg outputs premultiplied RGBA — demultiply
        let pixels = demultiply_alpha(pixmap.data());

        Ok(Self {
            pixels,
            width: render_size,
            height: render_size,
            fraction,
            clear_border,
            clear_shape,
        })
    }

    /// Create a logo from raw RGBA pixels.
    pub fn from_rgba(
        pixels: Vec<u8>,
        width: u32,
        height: u32,
        fraction: f64,
        clear_border: usize,
        clear_shape: LogoClearShape,
    ) -> Result<Self> {
        let fraction = validate_fraction(fraction)?;
        let clear_border = validate_clear_border(clear_border)?;
        if pixels.len() != (width * height * 4) as usize {
            return Err(Error::InvalidParameter(format!(
                "pixel buffer size {} doesn't match {}x{}x4",
                pixels.len(),
                width,
                height
            )));
        }
        Ok(Self {
            pixels,
            width,
            height,
            fraction,
            clear_border,
            clear_shape,
        })
    }

    /// Create a logo from PNG or JPEG image bytes.
    pub fn from_image_bytes(
        data: &[u8],
        fraction: f64,
        clear_border: usize,
        clear_shape: LogoClearShape,
    ) -> Result<Self> {
        let fraction = validate_fraction(fraction)?;
        let clear_border = validate_clear_border(clear_border)?;

        let img = image::load_from_memory(data)
            .map_err(|e| {
                Error::ImageEncode(format!("failed to decode image: {e}"))
            })?
            .into_rgba8();

        let width = img.width();
        let height = img.height();
        let pixels = img.into_raw();

        Ok(Self {
            pixels,
            width,
            height,
            fraction,
            clear_border,
            clear_shape,
        })
    }
}

fn validate_fraction(f: f64) -> Result<f64> {
    if !(0.01..=0.99).contains(&f) {
        return Err(Error::InvalidParameter(format!(
            "logo fraction must be 0.01–0.99, got {f}"
        )));
    }
    Ok(f)
}

fn validate_clear_border(b: usize) -> Result<usize> {
    if b > 5 {
        return Err(Error::InvalidParameter(format!(
            "clear_border must be 0–5, got {b}"
        )));
    }
    Ok(b)
}

/// Convert premultiplied RGBA to straight RGBA.
fn demultiply_alpha(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; data.len()];
    for i in (0..data.len()).step_by(4) {
        let a = data[i + 3] as u16;
        if a == 0 {
            // Fully transparent
            out[i] = 0;
            out[i + 1] = 0;
            out[i + 2] = 0;
            out[i + 3] = 0;
        } else if a == 255 {
            out[i..i + 4].copy_from_slice(&data[i..i + 4]);
        } else {
            out[i] = ((data[i] as u16 * 255 + a / 2) / a) as u8;
            out[i + 1] = ((data[i + 1] as u16 * 255 + a / 2) / a) as u8;
            out[i + 2] = ((data[i + 2] as u16 * 255 + a / 2) / a) as u8;
            out[i + 3] = a as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fraction_validation() {
        assert!(validate_fraction(0.25).is_ok());
        assert!(validate_fraction(0.0).is_err());
        assert!(validate_fraction(1.0).is_err());
    }

    #[test]
    fn clear_border_validation() {
        assert!(validate_clear_border(0).is_ok());
        assert!(validate_clear_border(5).is_ok());
        assert!(validate_clear_border(6).is_err());
    }

    #[test]
    fn demultiply_identity() {
        let data = vec![255, 128, 0, 255]; // fully opaque
        let out = demultiply_alpha(&data);
        assert_eq!(out, data);
    }

    #[test]
    fn demultiply_transparent() {
        let data = vec![0, 0, 0, 0];
        let out = demultiply_alpha(&data);
        assert_eq!(out, vec![0, 0, 0, 0]);
    }
}
