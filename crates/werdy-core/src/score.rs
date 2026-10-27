use std::collections::BTreeMap;

use crate::{Deed, DeedId, Mark, MarkEntry, WerdyDay};

/// Total mark fractions, due deed count, and rounded percent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Score {
    /// Sum of mark fractions for due deeds.
    pub earned: f64,
    /// Number of due deeds across included finished days.
    pub due: usize,
    /// Rounded percent, or none when no deed was due.
    pub percent: Option<u8>,
}

/// Scores deeds on supplied days before today. Today and later days are skipped.
pub fn score(
    deeds: &[Deed],
    marks: &BTreeMap<(DeedId, WerdyDay), MarkEntry>,
    days: &[WerdyDay],
    today: WerdyDay,
) -> Score {
    let mut earned = 0.0;
    let mut due = 0;
    for day in days {
        if *day >= today {
            continue;
        }
        for deed in deeds {
            if deed.is_due_on(*day) {
                due += 1;
                earned += marks
                    .get(&(deed.id, *day))
                    .map_or(0.0, |entry| entry.mark.value());
            }
        }
    }

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
) -> TodaySummary {
    let mut summary = TodaySummary {
        done: 0,
        partial: 0,
        open: 0,
        missed: 0,
    };
    for deed in deeds {
        if !deed.is_due_on(today) {
            continue;
        }
        match marks.get(&(deed.id, today)).map(|entry| entry.mark) {
            Some(Mark::Done) => summary.done += 1,
            Some(Mark::Partial(_)) => summary.partial += 1,
            Some(Mark::Missed) => summary.missed += 1,
            None => summary.open += 1,
        }
    }
    summary
}

fn rounded_percent(earned: f64, due: usize) -> u8 {
    let percent = earned / due as f64 * 100.0;
    // A few float steps protect exact halves without changing nearby values.
    let tolerance = 4.0 * f64::EPSILON * percent;
    (percent + tolerance).round() as u8
}
