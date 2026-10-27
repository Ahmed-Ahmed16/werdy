use crate::{Error, WerdyDay};

const LATE_AFTER_DAYS: i64 = 2;

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

/// Mark and the werdy day when the owner entered it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkEntry {
    /// The owner's mark for a due deed.
    pub mark: Mark,
    /// Werdy day when the owner entered the mark.
    pub entered_on: WerdyDay,
}

impl MarkEntry {
    /// Records a mark and its entry day.
    pub fn new(mark: Mark, entered_on: WerdyDay) -> Self {
        Self { mark, entered_on }
    }

    /// Says whether the mark was entered more than two days after its werdy day.
    pub fn is_late_for(self, marked_day: WerdyDay) -> bool {
        self.entered_on.days_since(marked_day) > LATE_AFTER_DAYS
    }
}
