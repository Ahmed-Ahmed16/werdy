use crate::{DeedRule, Error, WerdyDay};

const LATE_AFTER_DAYS: i64 = 2;
pub(crate) const PARTS_PER_DEED: u64 = 1_000_000_000_000;

pub(crate) fn is_late_for(entered_on: WerdyDay, day: WerdyDay) -> bool {
    entered_on.days_since(day) > LATE_AFTER_DAYS
}

/// Value from zero to one inclusive, stored to twelve decimal places.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fraction(u64);

impl Fraction {
    /// Creates a checked fraction, rounded half up to twelve decimal places.
    pub fn new(value: f64) -> Result<Self, Error> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(Error::FractionOutOfRange);
        }
        Ok(Self((value * PARTS_PER_DEED as f64).round() as u64))
    }

    /// Returns the stored fraction.
    pub fn value(self) -> f64 {
        self.0 as f64 / PARTS_PER_DEED as f64
    }

    pub(crate) fn parts(self) -> u64 {
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
        Self::Partial(Fraction(PARTS_PER_DEED / 2))
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

    // Count over target rounds half up once when converted to parts.
    pub(crate) fn parts(self, rule: &DeedRule) -> u64 {
        match self {
            Self::Done => PARTS_PER_DEED,
            Self::Partial(fraction) => fraction.parts(),
            Self::Missed | Self::Count(0) => 0,
            Self::Count(count) => match &rule.target {
                Some(target) => {
                    let amount = u128::from(target.amount());
                    let numerator =
                        u128::from(count.min(target.amount())) * u128::from(PARTS_PER_DEED);
                    ((numerator + amount / 2) / amount) as u64
                }
                None => PARTS_PER_DEED,
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
