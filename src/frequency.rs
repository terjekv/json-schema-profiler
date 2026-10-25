use serde::Serialize;

use crate::ProfileError;

/// An exact whole percentage in 1..=100. Comparisons use integer arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Frequency(u8);

impl Frequency {
    /// # Errors
    /// Rejects zero and percentages above 100.
    pub fn percent(value: u8) -> Result<Self, ProfileError> {
        if !(1..=100).contains(&value) {
            return Err(ProfileError::InvalidOptions(
                "frequency must be between 1 and 100 percent".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_percent(self) -> u8 {
        self.0
    }

    pub(crate) fn reached(self, count: u64, total: u64) -> bool {
        total > 0 && u128::from(count) * 100 >= u128::from(total) * u128::from(self.0)
    }

    pub(crate) fn at_most(self, count: u64, total: u64) -> bool {
        total > 0 && u128::from(count) * 100 <= u128::from(total) * u128::from(self.0)
    }
}
