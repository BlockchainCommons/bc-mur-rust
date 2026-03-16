use crate::{Error, Result};

/// RGBA color with 8-bit channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self { Self { r, g, b, a } }

    /// Parse hex color: `#RGB`, `#RRGGBB`, or `#RRGGBBAA`.
    pub fn from_hex(s: &str) -> Result<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        match s.len() {
            3 => {
                let r = hex_nibble(s.as_bytes()[0])?;
                let g = hex_nibble(s.as_bytes()[1])?;
                let b = hex_nibble(s.as_bytes()[2])?;
                Ok(Self::new(r << 4 | r, g << 4 | g, b << 4 | b, 255))
            }
            6 => {
                let r = hex_byte(&s[0..2])?;
                let g = hex_byte(&s[2..4])?;
                let b = hex_byte(&s[4..6])?;
                Ok(Self::new(r, g, b, 255))
            }
            8 => {
                let r = hex_byte(&s[0..2])?;
                let g = hex_byte(&s[2..4])?;
                let b = hex_byte(&s[4..6])?;
                let a = hex_byte(&s[6..8])?;
                Ok(Self::new(r, g, b, a))
            }
            _ => Err(Error::InvalidColor(format!(
                "expected #RGB, #RRGGBB, or #RRGGBBAA, got: #{s}"
            ))),
        }
    }

    /// True if alpha < 3 (effectively transparent).
    pub fn is_transparent(self) -> bool { self.a < 3 }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.a == 255 {
            write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            write!(
                f,
                "#{:02X}{:02X}{:02X}{:02X}",
                self.r, self.g, self.b, self.a
            )
        }
    }
}

fn hex_nibble(b: u8) -> Result<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(Error::InvalidColor(format!("invalid hex digit: {b}"))),
    }
}

fn hex_byte(s: &str) -> Result<u8> {
    let hi = hex_nibble(s.as_bytes()[0])?;
    let lo = hex_nibble(s.as_bytes()[1])?;
    Ok(hi << 4 | lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_6() {
        let c = Color::from_hex("#FF8000").unwrap();
        assert_eq!(c, Color::new(255, 128, 0, 255));
    }

    #[test]
    fn parse_hex_8() {
        let c = Color::from_hex("#FF800080").unwrap();
        assert_eq!(c, Color::new(255, 128, 0, 128));
    }

    #[test]
    fn parse_hex_3() {
        let c = Color::from_hex("#F80").unwrap();
        assert_eq!(c, Color::new(0xFF, 0x88, 0x00, 255));
    }

    #[test]
    fn display_rgb() {
        assert_eq!(Color::BLACK.to_string(), "#000000");
    }

    #[test]
    fn display_rgba() {
        assert_eq!(Color::new(255, 128, 0, 128).to_string(), "#FF800080");
    }
}
