use crate::{DeedRule, Error, WerdyDay};

const LATE_AFTER_DAYS: i64 = 2;

pub(crate) fn is_late_for(entered_on: WerdyDay, day: WerdyDay) -> bool {
    entered_on.days_since(day) > LATE_AFTER_DAYS
}

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
    /// Raw count entered for this deed and werdy day.
    Count(u32),
}

impl Mark {
    /// Returns the fixed half mark for a deed without a target.
    pub fn partial_without_target() -> Self {
        Self::Partial(Fraction(0.5))
    }

    /// Returns value under the rule in force on the marked day.
    /// Count without a target is full if positive, or zero otherwise.
    pub fn value(self, rule: &DeedRule) -> f64 {
        match self {
            Self::Done => 1.0,
            Self::Partial(fraction) => fraction.value(),
            Self::Missed => 0.0,
            Self::Count(0) => 0.0,
            Self::Count(count) => match &rule.target {
                Some(target) => count.min(target.amount()) as f64 / target.amount() as f64,
                None => 1.0,
            },
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
        is_late_for(self.entered_on, marked_day)
    }
}
