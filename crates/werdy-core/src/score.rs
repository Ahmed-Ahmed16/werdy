use std::collections::BTreeMap;

use crate::{Deed, DeedId, Mark, WerdyDay};

/// Total mark fractions, due deed count, and rounded percent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Score {
    /// Sum of mark fractions for due deeds.
    pub earned: f64,
    /// Number of due deeds across the given days.
    pub due: usize,
    /// Rounded percent, or none when no deed was due.
    pub percent: Option<u8>,
}

/// Scores deeds over the supplied finished werdy days.
pub fn score(
    deeds: &[Deed],
    marks: &BTreeMap<(DeedId, WerdyDay), Mark>,
    days: &[WerdyDay],
) -> Score {
    let mut earned = 0.0;
    let mut due = 0;
    for day in days {
        for deed in deeds {
            if deed.is_due_on(*day) {
                due += 1;
                earned += marks.get(&(deed.id, *day)).map_or(0.0, |mark| mark.value());
            }
        }
    }

    Score {
        earned,
        due,
        percent: (due > 0).then(|| rounded_percent(earned, due)),
    }
}

fn rounded_percent(earned: f64, due: usize) -> u8 {
    let percent = earned / due as f64 * 100.0;
    // A few float steps protect exact halves without changing nearby values.
    let tolerance = 4.0 * f64::EPSILON * percent;
    (percent + tolerance).round() as u8
}
