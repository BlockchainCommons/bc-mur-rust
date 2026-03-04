/// QR error correction level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionLevel {
    Low,
    Medium,
    Quartile,
    High,
}

impl CorrectionLevel {
    pub(crate) fn to_qrcode(self) -> qrcode::EcLevel {
        match self {
            Self::Low => qrcode::EcLevel::L,
            Self::Medium => qrcode::EcLevel::M,
            Self::Quartile => qrcode::EcLevel::Q,
            Self::High => qrcode::EcLevel::H,
        }
    }
}

impl std::fmt::Display for CorrectionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::Quartile => write!(f, "quartile"),
            Self::High => write!(f, "high"),
        }
    }
}

impl std::str::FromStr for CorrectionLevel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "low" | "l" => Ok(Self::Low),
            "medium" | "m" => Ok(Self::Medium),
            "quartile" | "q" => Ok(Self::Quartile),
            "high" | "h" => Ok(Self::High),
            _ => Err(format!(
                "unknown correction level: {s} (expected low, medium, quartile, or high)"
            )),
        }
    }
}
