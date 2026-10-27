//! Quiet counts of due days and days with a kept record. Neither count is a streak.

use crate::{Deed, DeedId, ExcuseCredit, OwnerRecords, WerdyDay, score};

/// Number of recent due days to count when the caller uses the default window.
pub const DEFAULT_CONSISTENCY_WINDOW: usize = 7;

/// Full value days among recent due days. This is not a streak.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Consistency {
    /// Due days worth one after any day credit.
    pub done: usize,
    /// Due days found before today, up to the chosen window.
    pub of: usize,
}

/// Counts full value days in the deed's last `window` due days before today.
/// A missed day lowers the count but never resets it.
pub fn consistency(
    records: &OwnerRecords<'_>,
    deed_id: DeedId,
    today: WerdyDay,
    window: usize,
) -> Consistency {
    let mut tally = Consistency { done: 0, of: 0 };
    let Some(deed) = records.deeds.iter().find(|deed| deed.id == deed_id) else {
        return tally;
    };
    let Some(mut day) = today.checked_sub_days(1) else {
        return tally;
    };

    while day >= deed.created_on() && tally.of < window {
        if let Some(rule) = deed.due_rule_on(day) {
            tally.of = tally.of.saturating_add(1);
            if score::due_value(records, deed, day, rule) == 1.0 {
                tally.done = tally.done.saturating_add(1);
            }
        }
        let Some(previous) = day.checked_sub_days(1) else {
            break;
        };
        day = previous;
    }
    tally
}

/// Counts finished days with due deeds all marked or covered by day credit.
/// Gaps do not reset this milestone count.
pub fn kept_days(records: &OwnerRecords<'_>, today: WerdyDay) -> usize {
    let Some(first_day) = records.deeds.iter().map(Deed::created_on).min() else {
        return 0;
    };
    let Some(mut day) = today.checked_sub_days(1) else {
        return 0;
    };
    let mut kept = 0usize;

    while day >= first_day {
        if day_is_kept(records, day) {
            kept = kept.saturating_add(1);
        }
        let Some(previous) = day.checked_sub_days(1) else {
            break;
        };
        day = previous;
    }
    kept
}

fn day_is_kept(records: &OwnerRecords<'_>, day: WerdyDay) -> bool {
    let has_credit = records
        .day_notes
        .get(&day)
        .is_some_and(|note| note.credit() != ExcuseCredit::None);
    let mut has_due_deed = false;
    for deed in records.deeds {
        if deed.is_due_on(day) {
            has_due_deed = true;
            if !has_credit && !records.marks.contains_key(&(deed.id, day)) {
                return false;
            }
        }
    }
    has_due_deed
}
