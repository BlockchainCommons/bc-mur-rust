use std::path::Path;
use std::process::Command;

use crate::{Error, QrFrame, Result};

/// Encode frames to ProRes 4444 via ffmpeg subprocess.
///
/// Writes frames as temporary PNGs, invokes ffmpeg, and
/// cleans up the temp directory. Requires `ffmpeg` on PATH.
pub fn encode_prores(
    frames: &[QrFrame],
    fps: f64,
    output_path: &Path,
) -> Result<()> {
    // Check for ffmpeg
    let ffmpeg = which_ffmpeg()?;

    // Write frames to temp directory
    let tmp_dir = tempfile::tempdir()?;
    crate::write_frame_pngs(frames, tmp_dir.path())?;

    let fps_str = format!("{fps}");
    let input_pattern =
        tmp_dir.path().join("%04d.png");

    let status = Command::new(&ffmpeg)
        .args([
            "-y",
            "-r",
            &fps_str,
            "-i",
            input_pattern.to_str().unwrap(),
            "-c:v",
            "prores_ks",
            "-profile:v",
            "4444",
            "-pix_fmt",
            "yuva444p10le",
            output_path.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()?;

    if !status.success() {
        return Err(Error::FfmpegFailed(format!(
            "ffmpeg exited with status {status}"
        )));
    }

    // tmp_dir auto-cleans up on drop
    Ok(())
}

fn which_ffmpeg() -> Result<String> {
    // Try `which ffmpeg` on Unix
    let output = Command::new("which")
        .arg("ffmpeg")
        .output()
        .map_err(|_| Error::FfmpegNotFound)?;

    if output.status.success() {
        let path =
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }

    Err(Error::FfmpegNotFound)
}
