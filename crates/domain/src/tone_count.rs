use serde::{Deserialize, Serialize};

/// Number of grayscale tone levels a conversion is quantized into.
///
/// CLAUDE.md Section 7: 2 to 16 tones, with 5 as the recommended default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct ToneCount(u8);

impl ToneCount {
    pub const MIN: u8 = 2;
    pub const MAX: u8 = 16;
    pub const RECOMMENDED_DEFAULT: ToneCount = ToneCount(5);

    pub fn new(value: u8) -> Result<Self, ToneCountError> {
        if (Self::MIN..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(ToneCountError::OutOfRange(value))
        }
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl Default for ToneCount {
    fn default() -> Self {
        Self::RECOMMENDED_DEFAULT
    }
}

impl TryFrom<u8> for ToneCount {
    type Error = ToneCountError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ToneCount> for u8 {
    fn from(value: ToneCount) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ToneCountError {
    #[error(
        "tone count must be between {} and {} (got {0})",
        ToneCount::MIN,
        ToneCount::MAX
    )]
    OutOfRange(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_values_in_range() {
        assert_eq!(ToneCount::new(2).unwrap().get(), 2);
        assert_eq!(ToneCount::new(16).unwrap().get(), 16);
        assert_eq!(ToneCount::default().get(), 5);
    }

    #[test]
    fn rejects_values_out_of_range() {
        assert_eq!(ToneCount::new(1), Err(ToneCountError::OutOfRange(1)));
        assert_eq!(ToneCount::new(17), Err(ToneCountError::OutOfRange(17)));
        assert_eq!(ToneCount::new(0), Err(ToneCountError::OutOfRange(0)));
    }

    #[test]
    fn round_trips_through_serde() {
        let tones = ToneCount::new(9).unwrap();
        let json = serde_json::to_string(&tones).unwrap();
        assert_eq!(json, "9");
        let back: ToneCount = serde_json::from_str(&json).unwrap();
        assert_eq!(back, tones);
    }

    #[test]
    fn rejects_out_of_range_through_serde() {
        let err = serde_json::from_str::<ToneCount>("17");
        assert!(err.is_err());
    }
}
