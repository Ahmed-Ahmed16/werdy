use crate::Error;

/// Value from zero to one inclusive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fraction(f64);

impl Fraction {
    /// Creates a checked fraction.
    pub fn new(value: f64) -> Result<Self, Error> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(Error::FractionOutOfRange);
        }
        Ok(Self(value))
    }

    /// Returns the stored fraction.
    pub fn value(self) -> f64 {
        self.0
    }
}

/// Owner's record for one deed on one werdy day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mark {
    /// Deed was done in full.
    Done,
    /// Deed was done in part.
    Partial(Fraction),
    /// Deed was missed.
    Missed,
}

impl Mark {
    /// Returns the fraction earned by this mark.
    pub fn value(self) -> f64 {
        match self {
            Self::Done => 1.0,
            Self::Partial(fraction) => fraction.value(),
            Self::Missed => 0.0,
        }
    }
}
