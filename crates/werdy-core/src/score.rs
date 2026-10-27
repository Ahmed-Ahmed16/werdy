use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::mark::{compare_rationals, gcd};
use crate::{CalendarSettings, DayNote, Deed, DeedId, Error, Fraction, MarkEntry, WerdyDay};

/// Owner's deeds, marks, and day notes used by a score.
pub struct OwnerRecords<'a> {
    /// Deeds and their dated rules.
    pub deeds: &'a [Deed],
    /// Marks keyed by deed and werdy day.
    pub marks: &'a BTreeMap<(DeedId, WerdyDay), MarkEntry>,
    /// Day notes keyed by werdy day.
    pub day_notes: &'a BTreeMap<WerdyDay, DayNote>,
    /// Calendar settings, such as the Hijri offset.
    pub calendar: CalendarSettings,
}

/// Sum of deed values as a reduced exact fraction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScoreValue {
    numerator: u128,
    denominator: u128,
}

impl Default for ScoreValue {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Ord for ScoreValue {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_rationals(
            self.numerator,
            self.denominator,
            other.numerator,
            other.denominator,
        )
    }
}

impl PartialOrd for ScoreValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl ScoreValue {
    /// Zero earned value.
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    /// Returns the exact numerator.
    pub fn numerator(self) -> u128 {
        self.numerator
    }

    /// Returns the exact denominator.
    pub fn denominator(self) -> u128 {
        self.denominator
    }

    /// Returns an approximate value for display.
    pub fn as_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    pub(crate) fn add_fraction(&mut self, value: Fraction) -> Result<(), Error> {
        let shared = gcd(self.denominator, value.denominator());
        let left_scale = value.denominator() / shared;
        let right_scale = self.denominator / shared;
        let numerator = self
            .numerator
            .checked_mul(left_scale)
            .and_then(|left| {
                value
                    .numerator()
                    .checked_mul(right_scale)
                    .and_then(|right| left.checked_add(right))
            })
            .ok_or(Error::ScoreOverflow)?;
        let denominator = self
            .denominator
            .checked_mul(left_scale)
            .ok_or(Error::ScoreOverflow)?;
        let divisor = gcd(numerator, denominator);
        self.numerator = numerator / divisor;
        self.denominator = denominator / divisor;
        Ok(())
    }
}

/// Total earned value, due deed count, and rounded percent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Score {
    /// Sum of mark values after any day credit for due deeds.
    pub earned: ScoreValue,
    /// Number of due deeds across included finished days.
    pub due: usize,
    /// Rounded percent, or none when no deed was due.
    pub percent: Option<u8>,
}

/// Scores due deeds with any day credit on supplied days before today.
/// Today and later days are skipped.
/// Returns `ScoreOverflow` if the exact total or due count cannot fit.
pub fn score(
    records: &OwnerRecords<'_>,
    days: &[WerdyDay],
    today: WerdyDay,
) -> Result<Score, Error> {
    let mut earned = ScoreValue::ZERO;
    let mut due = 0usize;
    for day in days {
        if *day >= today {
            continue;
        }
        for deed in records.deeds {
            if let Some(rule) = deed.due_rule_on(*day, records.calendar) {
                due = due.checked_add(1).ok_or(Error::ScoreOverflow)?;
                earned.add_fraction(due_value(records, deed, *day, rule))?;
            }
        }
    }
    Ok(score_from_totals(earned, due))
}

pub(crate) fn due_value(
    records: &OwnerRecords<'_>,
    deed: &Deed,
    day: WerdyDay,
    rule: &crate::DeedRule,
) -> Fraction {
    let credit = records
        .day_notes
        .get(&day)
        .map_or(Fraction::ZERO, |note| note.credit().exact_value());
    let mark_value = records
        .marks
        .get(&(deed.id, day))
        .map_or(Fraction::ZERO, |entry| entry.mark.exact_value(rule));
    mark_value.max(credit)
}

pub(crate) fn score_from_totals(earned: ScoreValue, due: usize) -> Score {
    Score {
        earned,
        due,
        percent: (due > 0).then(|| rounded_percent(earned, due)),
    }
}

/// Counts due deeds on the current werdy day without a percent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TodaySummary {
    /// Due deeds marked Done.
    pub done: usize,
    /// Due deeds marked Partial.
    pub partial: usize,
    /// Due deeds with no mark yet.
    pub open: usize,
    /// Due deeds marked Missed by the owner.
    pub missed: usize,
}

/// Counts marks and open deeds due today.
pub fn today_summary(
    deeds: &[Deed],
    marks: &BTreeMap<(DeedId, WerdyDay), MarkEntry>,
    today: WerdyDay,
    calendar: CalendarSettings,
) -> TodaySummary {
    let mut summary = TodaySummary {
        done: 0,
        partial: 0,
        open: 0,
        missed: 0,
    };
    for deed in deeds {
        let Some(rule) = deed.due_rule_on(today, calendar) else {
            continue;
        };
        match marks.get(&(deed.id, today)).map(|entry| entry.mark) {
            Some(mark) => match mark.exact_value(rule) {
                Fraction::ZERO => summary.missed += 1,
                Fraction::ONE => summary.done += 1,
                _ => summary.partial += 1,
            },
            None => summary.open += 1,
        }
    }
    summary
}

fn rounded_percent(earned: ScoreValue, due: usize) -> u8 {
    for percent in (1u8..=100).rev() {
        let threshold = (u128::from(percent) * 2 - 1) * due as u128;
        if compare_rationals(earned.numerator, earned.denominator, threshold, 200) != Ordering::Less
        {
            return percent;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_overflow_returns_error_without_changing_total() -> Result<(), Error> {
        let mut total = ScoreValue::ZERO;
        total.add_fraction(Fraction::from_ratio(1, u128::MAX)?)?;
        let before = total;
        assert_eq!(
            total.add_fraction(Fraction::from_ratio(1, u128::MAX - 1)?),
            Err(Error::ScoreOverflow)
        );
        assert_eq!(total, before);
        Ok(())
    }
}
