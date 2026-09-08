/// A physical length, stored internally as millimeters (CLAUDE.md Section
/// 11: mm is the default unit; cm, inch, and px are also supported via
/// conversion).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Length {
    millimeters: f64,
}

const MM_PER_INCH: f64 = 25.4;
const MM_PER_CM: f64 = 10.0;

impl Length {
    pub fn from_millimeters(value: f64) -> Self {
        Self { millimeters: value }
    }

    pub fn from_centimeters(value: f64) -> Self {
        Self::from_millimeters(value * MM_PER_CM)
    }

    pub fn from_inches(value: f64) -> Self {
        Self::from_millimeters(value * MM_PER_INCH)
    }

    /// `dpi` is the source image's dots-per-inch, required to convert a
    /// pixel count into a physical length.
    pub fn from_pixels(pixels: f64, dpi: f64) -> Self {
        Self::from_inches(pixels / dpi)
    }

    pub fn as_millimeters(self) -> f64 {
        self.millimeters
    }

    pub fn as_centimeters(self) -> f64 {
        self.millimeters / MM_PER_CM
    }

    pub fn as_inches(self) -> f64 {
        self.millimeters / MM_PER_INCH
    }

    pub fn as_pixels(self, dpi: f64) -> f64 {
        self.as_inches() * dpi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_between_units() {
        let one_inch = Length::from_inches(1.0);
        assert!((one_inch.as_millimeters() - 25.4).abs() < 1e-9);
        assert!((one_inch.as_centimeters() - 2.54).abs() < 1e-9);
    }

    #[test]
    fn converts_pixels_at_given_dpi() {
        let length = Length::from_pixels(300.0, 300.0);
        assert!((length.as_inches() - 1.0).abs() < 1e-9);
        assert!((length.as_pixels(150.0) - 150.0).abs() < 1e-9);
    }
}
