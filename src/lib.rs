#![doc(html_root_url = "https://docs.rs/bc-mur/0.1.0")]
#![warn(rust_2018_idioms)]

//! # bc-mur
//!
//! Multipart UR QR code generator — single-frame and animated
//! fountain-coded QR sequences with optional logo overlay.
//!
//! This crate provides:
//! - Single-frame QR code rendering from raw bytes or UR strings
//! - Logo overlay with module-aligned compositing
//! - Animated multipart fountain-coded QR sequences (GIF)
//! - ProRes 4444 encoding via optional ffmpeg integration
//! - Frame dump for custom pipelines
//!
//! # Example
//!
//! ```rust
//! use bc_mur::{render_qr, CorrectionLevel, Color};
//!
//! let img = render_qr(
//!     b"UR:BYTES/HDCXDWINVEZM",
//!     CorrectionLevel::Low,
//!     512,
//!     Color::BLACK,
//!     Color::WHITE,
//!     1,    // quiet zone modules
//!     None, // no logo
//! ).unwrap();
//! let png_bytes = img.to_png().unwrap();
//! assert!(!png_bytes.is_empty());
//! ```

mod animate;
mod color;
mod correction;
mod error;
mod logo;
mod prores;
mod qr_matrix;
mod render;

pub use animate::{
    encode_animated_gif, generate_frames, write_frame_pngs,
    AnimateParams, QrFrame,
};
pub use color::Color;
pub use correction::CorrectionLevel;
pub use error::{Error, Result};
pub use logo::{Logo, LogoClearShape};
pub use prores::encode_prores;
pub use qr_matrix::{
    check_qr_density, qr_module_count, DEFAULT_MAX_MODULES,
};
pub use render::{render_qr, render_ur_qr, RenderedImage};
