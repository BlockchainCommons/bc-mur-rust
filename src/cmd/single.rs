use std::io::Read;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use bc_mur::{
    Color, CorrectionLevel, Logo, LogoClearShape,
    DEFAULT_MAX_MODULES,
};

use crate::exec::Exec;

/// Render a single-frame QR code from a UR string.
#[derive(Debug, Args)]
pub struct CommandArgs {
    /// UR string to encode, or `-` to read from stdin.
    pub ur_string: String,

    /// Output file path (default: stdout as raw PNG).
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Image size in pixels.
    #[arg(short, long, default_value = "512")]
    pub size: u32,

    /// Foreground color (hex).
    #[arg(long, default_value = "#000000")]
    pub fg: String,

    /// Background color (hex).
    #[arg(long, default_value = "#FFFFFF")]
    pub bg: String,

    /// Path to SVG logo file.
    #[arg(long)]
    pub logo: Option<PathBuf>,

    /// Logo fraction of QR width (0.01–0.99).
    #[arg(long, default_value = "0.25")]
    pub logo_fraction: f64,

    /// Logo clear border in modules (0–5).
    #[arg(long, default_value = "1")]
    pub logo_border: usize,

    /// Logo clear shape (square or circle).
    #[arg(long, default_value = "square")]
    pub logo_shape: String,

    /// Error correction level (low, medium, quartile, high).
    #[arg(short, long)]
    pub correction: Option<String>,

    /// Quiet zone modules around the QR code.
    #[arg(long, default_value = "1")]
    pub quiet_zone: u32,

    /// Dark mode (white-on-black).
    #[arg(long, default_value = "false")]
    pub dark: bool,

    /// Output format (png or jpeg).
    #[arg(long, default_value = "png")]
    pub format: String,

    /// JPEG quality (1–100).
    #[arg(long, default_value = "90")]
    pub jpeg_quality: u8,

    /// Maximum QR module count for reliable scanning.
    #[arg(long, default_value_t = DEFAULT_MAX_MODULES)]
    pub max_modules: usize,

    /// Disable the QR density check.
    #[arg(long, default_value = "false")]
    pub no_density_check: bool,
}

impl Exec for CommandArgs {
    fn exec(&self) -> Result<String> {
        let ur_string = read_input(&self.ur_string)?;
        let (fg, bg) = if self.dark {
            (
                Color::from_hex(&self.bg)?,
                Color::from_hex(&self.fg)?,
            )
        } else {
            (
                Color::from_hex(&self.fg)?,
                Color::from_hex(&self.bg)?,
            )
        };

        let logo = if let Some(path) = &self.logo {
            let svg_data = std::fs::read(path)?;
            let shape: LogoClearShape =
                self.logo_shape.parse().map_err(|e: String| {
                    anyhow::anyhow!(e)
                })?;
            Some(Logo::from_svg(
                &svg_data,
                self.logo_fraction,
                self.logo_border,
                shape,
            )?)
        } else {
            None
        };

        let correction = match &self.correction {
            Some(s) => s
                .parse::<CorrectionLevel>()
                .map_err(|e| anyhow::anyhow!(e))?,
            None => {
                if logo.is_some() {
                    CorrectionLevel::High
                } else {
                    CorrectionLevel::Low
                }
            }
        };

        // Check QR density before rendering.
        if !self.no_density_check {
            let upper = ur_string.to_ascii_uppercase();
            let modules = bc_mur::qr_module_count(
                upper.as_bytes(),
                correction,
            )?;
            bc_mur::check_qr_density(
                modules,
                self.max_modules,
            )?;
        }

        let img = bc_mur::render_ur_qr(
            &ur_string,
            correction,
            self.size,
            fg,
            bg,
            self.quiet_zone,
            logo.as_ref(),
        )?;

        let data = match self.format.as_str() {
            "png" => img.to_png()?,
            "jpeg" | "jpg" => {
                img.to_jpeg(self.jpeg_quality)?
            }
            other => {
                anyhow::bail!("unknown format: {other} (expected png or jpeg)")
            }
        };

        if let Some(path) = &self.output {
            std::fs::write(path, &data)?;
            Ok(format!(
                "Wrote {} bytes to {}",
                data.len(),
                path.display()
            ))
        } else {
            use std::io::Write;
            std::io::stdout().write_all(&data)?;
            Ok(String::new())
        }
    }
}

pub fn read_input(s: &str) -> Result<String> {
    if s == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        Ok(buf.trim().to_string())
    } else {
        Ok(s.to_string())
    }
}
