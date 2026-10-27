use crate::{DeedRule, Error, Target, WerdyDay};
use std::cmp::Ordering;

const LATE_AFTER_DAYS: i64 = 2;
const DECIMAL_PLACES: u128 = 1_000_000_000_000;

pub(crate) fn is_late_for(entered_on: WerdyDay, day: WerdyDay) -> bool {
    entered_on.days_since(day) > LATE_AFTER_DAYS
}

/// Exact value from zero to one inclusive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Fraction {
    numerator: u128,
    denominator: u128,
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Fraction {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FractionVisitor;

        impl<'de> serde::de::Visitor<'de> for FractionVisitor {
            type Value = Fraction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an exact fraction or a legacy decimal")
            }

            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                map: M,
            ) -> Result<Self::Value, M::Error> {
                #[derive(serde::Deserialize)]
                struct Parts {
                    numerator: u128,
                    denominator: u128,
                }
                let parts: Parts = serde::Deserialize::deserialize(
                    serde::de::value::MapAccessDeserializer::new(map),
                )?;
                Fraction::from_ratio(parts.numerator, parts.denominator)
                    .map_err(serde::de::Error::custom)
            }

            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Fraction::new(value).map_err(E::custom)
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                self.visit_f64(value as f64)
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                self.visit_f64(value as f64)
            }
        }

        deserializer.deserialize_any(FractionVisitor)
    }
}

impl Fraction {
    /// Zero value.
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };
    /// Half value.
    pub const HALF: Self = Self {
        numerator: 1,
        denominator: 2,
    };
    /// Full value.
    pub const ONE: Self = Self {
        numerator: 1,
        denominator: 1,
    };

    /// Converts a float once to a decimal fraction with twelve places.
    /// Rounds the decimal numerator half up. Use `from_ratio` for exact input.
    pub fn new(value: f64) -> Result<Self, Error> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(Error::FractionOutOfRange);
        }
        Self::from_ratio(
            (value * DECIMAL_PLACES as f64).round() as u128,
            DECIMAL_PLACES,
        )
    }

    /// Creates an exact fraction. Denominator must be positive and value at most one.
    pub fn from_ratio(numerator: u128, denominator: u128) -> Result<Self, Error> {
        if denominator == 0 || numerator > denominator {
            return Err(Error::FractionOutOfRange);
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    /// Returns an approximate value for display.
    pub fn value(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    /// Returns the exact numerator.
    pub fn numerator(self) -> u128 {
        self.numerator
    }

    /// Returns the exact denominator.
    pub fn denominator(self) -> u128 {
        self.denominator
    }

    pub(crate) fn complement(self) -> Self {
        let numerator = self.denominator - self.numerator;
        let divisor = gcd(numerator, self.denominator);
        Self {
            numerator: numerator / divisor,
            denominator: self.denominator / divisor,
        }
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_rationals(
            self.numerator,
            self.denominator,
            other.numerator,
            other.denominator,
        )
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(crate) fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left
}

pub(crate) fn compare_rationals(
    mut left_num: u128,
    mut left_den: u128,
    mut right_num: u128,
    mut right_den: u128,
) -> Ordering {
    let mut reversed = false;
    loop {
        let (left_whole, left_rest) = (left_num / left_den, left_num % left_den);
        let (right_whole, right_rest) = (right_num / right_den, right_num % right_den);
        let order = left_whole.cmp(&right_whole);
        if order != Ordering::Equal {
            return if reversed { order.reverse() } else { order };
        }
        if left_rest == 0 || right_rest == 0 {
            let order = left_rest.cmp(&right_rest);
            return if reversed { order.reverse() } else { order };
        }
        (left_num, left_den) = (left_den, left_rest);
        (right_num, right_den) = (right_den, right_rest);
        reversed = !reversed;
    }
}

impl TryFrom<f64> for Fraction {
    type Error = Error;

    fn try_from(value: f64) -> Result<Self, Error> {
        Self::new(value)
    }
}

impl From<Fraction> for f64 {
    fn from(fraction: Fraction) -> Self {
        fraction.value()
    }
}

/// Owner's record for one deed on one werdy day.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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
        Self::Partial(Fraction::HALF)
    }

    /// Returns approximate value for display under the rule in force on the marked day.
    /// Count without a target is full if positive, or zero otherwise.
    pub fn value(self, rule: &DeedRule) -> f64 {
        self.exact_value(rule).value()
    }

    pub(crate) fn exact_value(self, rule: &DeedRule) -> Fraction {
        match self {
            Self::Done => Fraction::ONE,
            Self::Partial(fraction) => fraction,
            Self::Missed | Self::Count(0) => Fraction::ZERO,
            Self::Count(count) => match &rule.target {
                Some(target) => {
                    let numerator = u128::from(count.min(target.amount()));
                    let denominator = u128::from(target.amount());
                    let divisor = gcd(numerator, denominator);
                    Fraction {
                        numerator: numerator / divisor,
                        denominator: denominator / divisor,
                    }
                }
                None => Fraction::ONE,
            },
        }
    }
}

/// Converts a mark when a deed switches between button and count input.
pub fn convert_mark_for_target(mark: Mark, old: Option<&Target>, new: Option<&Target>) -> Mark {
    match (old, new) {
        (None, Some(target)) => match mark {
            Mark::Done | Mark::Count(1..) => Mark::Count(target.amount()),
            Mark::Partial(fraction) if fraction == Fraction::ZERO => Mark::Missed,
            Mark::Partial(fraction) => {
                let count = (fraction.value() * f64::from(target.amount())).round() as u32;
                Mark::Count(count.clamp(1, target.amount()))
            }
            Mark::Missed | Mark::Count(0) => Mark::Missed,
        },
        (Some(target), None) => match mark {
            Mark::Count(0) => Mark::Missed,
            Mark::Count(count) if count >= target.amount() => Mark::Done,
            // Button input has one partial value, so any positive short count becomes half.
            Mark::Count(_) => Mark::partial_without_target(),
            other => other,
        },
        _ => mark,
    }
}

/// Mark and the werdy day when the owner entered it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
