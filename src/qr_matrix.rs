use qrcode::QrCode;

use crate::{CorrectionLevel, Error, Result};

/// Default maximum QR module count for reliable phone scanning.
/// Corresponds to QR version 25 (117×117 modules).
pub const DEFAULT_MAX_MODULES: usize = 117;

/// Get the QR module count for a message without rendering.
pub fn qr_module_count(
    message: &[u8],
    correction: CorrectionLevel,
) -> Result<usize> {
    let matrix = QrMatrix::encode(message, correction)?;
    Ok(matrix.width())
}

/// Check that a module count is within a density limit.
///
/// Returns `Error::QrCodeTooDense` if `module_count > max_modules`.
pub fn check_qr_density(
    module_count: usize,
    max_modules: usize,
) -> Result<()> {
    if module_count > max_modules {
        Err(Error::QrCodeTooDense {
            module_count,
            max_modules,
        })
    } else {
        Ok(())
    }
}

/// A boolean QR module matrix.
pub struct QrMatrix {
    modules: Vec<bool>,
    width: usize,
}

impl QrMatrix {
    /// Encode a byte message into a QR matrix at the given
    /// correction level.
    pub fn encode(
        message: &[u8],
        correction: CorrectionLevel,
    ) -> Result<Self> {
        let code = QrCode::with_error_correction_level(
            message,
            correction.to_qrcode(),
        )
        .map_err(|e| Error::QrEncode(e.to_string()))?;

        let width = code.width();
        let modules: Vec<bool> = code
            .to_colors()
            .into_iter()
            .map(|c| c == qrcode::Color::Dark)
            .collect();

        Ok(Self { modules, width })
    }

    /// Module count (width == height for QR codes).
    pub fn width(&self) -> usize { self.width }

    /// True if the module at (col, row) is dark.
    pub fn is_dark(&self, col: usize, row: usize) -> bool {
        self.modules[row * self.width + col]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_small() {
        let m =
            QrMatrix::encode(b"HELLO", CorrectionLevel::Low)
                .unwrap();
        // Version 1 QR: 21x21 modules
        assert_eq!(m.width(), 21);
        assert_eq!(m.modules.len(), 21 * 21);
    }

    #[test]
    fn encode_ur_string() {
        // UR strings are uppercased for alphanumeric QR
        // efficiency
        let ur = "UR:BYTES/HDCXDWINVEZM";
        let m =
            QrMatrix::encode(ur.as_bytes(), CorrectionLevel::Low)
                .unwrap();
        assert!(m.width() >= 21);
    }
}
