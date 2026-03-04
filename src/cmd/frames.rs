use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use bc_mur::{
    AnimateParams, Color, CorrectionLevel, Logo,
    LogoClearShape, DEFAULT_MAX_MODULES,
};

use crate::exec::Exec;

/// Dump multipart QR frames as numbered PNGs.
#[derive(Debug, Args)]
pub struct CommandArgs {
    /// UR string to encode, or `-` to read from stdin.
    pub ur_string: String,

    /// Output directory for numbered PNGs.
    #[arg(short, long)]
    pub output: PathBuf,

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

    /// Maximum fragment length for fountain coding.
    #[arg(long, default_value = "100")]
    pub max_fragment_len: usize,

    /// Frames per second (affects cycle count).
    #[arg(long, default_value = "8")]
    pub fps: f64,

    /// Number of complete cycles through all fragments.
    #[arg(long, default_value = "3")]
    pub cycles: u32,

    /// Exact number of frames (overrides --cycles).
    #[arg(long)]
    pub frame_count: Option<usize>,

    /// Maximum QR module count for reliable scanning.
    #[arg(long, default_value_t = DEFAULT_MAX_MODULES)]
    pub max_modules: usize,

    /// Disable the QR density check.
    #[arg(long, default_value = "false")]
    pub no_density_check: bool,
}

impl Exec for CommandArgs {
    fn exec(&self) -> Result<String> {
        let ur_string =
            super::single::read_input(&self.ur_string)?;
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

        let correction = self
            .correction
            .as_ref()
            .map(|s| {
                s.parse::<CorrectionLevel>()
                    .map_err(|e| anyhow::anyhow!(e))
            })
            .transpose()?;

        let ur = bc_ur::UR::from_ur_string(&ur_string)?;

        let params = AnimateParams {
            max_fragment_len: self.max_fragment_len,
            correction,
            size: self.size,
            foreground: fg,
            background: bg,
            quiet_zone: self.quiet_zone,
            logo,
            fps: self.fps,
            cycles: self.cycles,
            frame_count: self.frame_count,
            max_modules: if self.no_density_check {
                None
            } else {
                Some(self.max_modules)
            },
        };

        let frames =
            bc_mur::generate_frames(&ur, &params)?;
        bc_mur::write_frame_pngs(&frames, &self.output)?;

        Ok(format!(
            "Wrote {} frames to {}",
            frames.len(),
            self.output.display()
        ))
    }
}
