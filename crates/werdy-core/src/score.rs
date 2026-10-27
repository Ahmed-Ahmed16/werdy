use std::collections::BTreeMap;

use crate::mark::PARTS_PER_DEED;
use crate::{CalendarSettings, DayNote, Deed, DeedId, Mark, MarkEntry, WerdyDay};

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

/// Sum of deed values in exact parts.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ScoreValue(u128);

impl ScoreValue {
    /// Zero earned value.
    pub const ZERO: Self = Self(0);

    /// Returns the exact number of trillionth parts.
    pub fn parts(self) -> u128 {
        self.0
    }

    /// Returns an approximate value for display.
    pub fn as_f64(self) -> f64 {
        self.0 as f64 / PARTS_PER_DEED as f64
    }

    pub(crate) fn add_parts(&mut self, parts: u64) {
        self.0 += u128::from(parts);
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
pub fn score(records: &OwnerRecords<'_>, days: &[WerdyDay], today: WerdyDay) -> Score {
    let mut earned = ScoreValue::ZERO;
    let mut due = 0;
    for day in days {
        if *day >= today {
            continue;
        }
        for deed in records.deeds {
            if let Some(rule) = deed.due_rule_on(*day, records.calendar) {
                due += 1;
                earned.add_parts(due_value(records, deed, *day, rule));
            }
        }
    }
    score_from_totals(earned, due)
}

pub(crate) fn due_value(
    records: &OwnerRecords<'_>,
    deed: &Deed,
    day: WerdyDay,
    rule: &crate::DeedRule,
) -> u64 {
    let credit = records
        .day_notes
        .get(&day)
        .map_or(0, |note| note.credit().parts());
    let mark_value = records
        .marks
        .get(&(deed.id, day))
        .map_or(0, |entry| entry.mark.parts(rule));
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
            Some(Mark::Done) => summary.done += 1,
            Some(Mark::Partial(_)) => summary.partial += 1,
            Some(Mark::Missed) => summary.missed += 1,
            Some(mark @ Mark::Count(_)) => match mark.parts(rule) {
                0 => summary.missed += 1,
                PARTS_PER_DEED => summary.done += 1,
                _ => summary.partial += 1,
            },
            None => summary.open += 1,
        }
    }
    summary
}

fn rounded_percent(earned: ScoreValue, due: usize) -> u8 {
    let denominator = due as u128 * u128::from(PARTS_PER_DEED);
    ((earned.parts() * 100 + denominator / 2) / denominator) as u8
}
