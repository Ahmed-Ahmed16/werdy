//! Section counts, chip selection and fold choices for the daily sheet.

use std::collections::BTreeMap;

use werdy_core::{DailySheet, SheetRow, SheetSlot, SlotKey, WerdyDay};

#[derive(Clone, Copy, PartialEq)]
pub struct SlotProgress {
    pub marked: usize,
    pub total: usize,
    pub complete: bool,
}

pub struct SectionView {
    pub slot: SheetSlot,
    pub progress: SlotProgress,
    pub current: bool,
}

/// Chips follow visible sections in sheet order. Counts still describe the day
/// when the owner filters the rows to unmarked deeds.
pub fn section_views(
    shown: DailySheet,
    full: Option<&DailySheet>,
    current: Option<&str>,
) -> Vec<SectionView> {
    shown
        .slots
        .into_iter()
        .filter(|slot| !slot.rows.is_empty())
        .map(|slot| {
            let counted_slot = full
                .and_then(|sheet| sheet.slots.iter().find(|other| other.slot == slot.slot))
                .unwrap_or(&slot);
            let (marked, total) = section_count(&counted_slot.rows);
            SectionView {
                progress: SlotProgress {
                    marked,
                    total,
                    complete: counted_slot.complete,
                },
                current: current == Some(slot.slot.0.as_str()),
                slot,
            }
        })
        .collect()
}

/// Marked deeds and counted deeds in a section. A miss counts as marked
/// because the person has dealt with it. An unmarked when it comes up deed is
/// not counted (decision 0047, see `SheetRow::counts_in_section`).
fn section_count(rows: &[SheetRow]) -> (usize, usize) {
    let marked = rows.iter().filter(|row| row.mark.is_some()).count();
    let counted = rows.iter().filter(|row| row.counts_in_section()).count();
    (marked, counted)
}

/// Manual choices for each viewed date during the current werdy day.
pub struct FoldChoices {
    today: WerdyDay,
    choices: BTreeMap<(WerdyDay, SlotKey), bool>,
}

impl FoldChoices {
    pub fn new(today: WerdyDay) -> Self {
        Self {
            today,
            choices: BTreeMap::new(),
        }
    }

    pub fn is_for(&self, today: WerdyDay) -> bool {
        self.today == today
    }

    pub fn reset_for(&mut self, today: WerdyDay) {
        if !self.is_for(today) {
            *self = Self::new(today);
        }
    }

    /// A manual choice wins for the day. Otherwise a slot with every mark
    /// entered folds on its own.
    pub fn open(
        &self,
        today: WerdyDay,
        viewed: WerdyDay,
        key: &SlotKey,
        all_entered: bool,
    ) -> bool {
        if !self.is_for(today) {
            return !all_entered;
        }
        self.choices
            .get(&(viewed, key.clone()))
            .copied()
            .unwrap_or(!all_entered)
    }

    pub fn set<'a>(
        &mut self,
        today: WerdyDay,
        viewed: WerdyDay,
        keys: impl IntoIterator<Item = &'a SlotKey>,
        open: bool,
    ) {
        self.reset_for(today);
        for key in keys {
            self.choices.insert((viewed, key.clone()), open);
        }
    }
}

/// The fold button opens all sections only when every section is folded.
pub fn fold_all_opens(open: &[bool]) -> bool {
    !open.is_empty() && open.iter().all(|open| !open)
}

/// For the first jump of the day, use the wanted slot, the next visible
/// section or the last one. Sections missing from the order sort last.
pub fn nearest_slot<'a>(
    shown: &'a [SlotKey],
    order: &[SlotKey],
    wanted: &str,
) -> Option<&'a SlotKey> {
    let position = |key: &SlotKey| {
        order
            .iter()
            .position(|slot| slot == key)
            .unwrap_or(order.len())
    };
    let wanted_index = order.iter().position(|slot| slot.0 == wanted).unwrap_or(0);
    shown
        .iter()
        .find(|slot| slot.0 == wanted)
        .or_else(|| shown.iter().find(|slot| position(slot) >= wanted_index))
        .or_else(|| shown.last())
}

#[cfg(test)]
mod tests {
    use werdy_core::{DeedId, Mark, RowTag};

    use super::*;

    fn row(id: u64, mark: Option<Mark>) -> SheetRow {
        SheetRow {
            deed_id: DeedId(id),
            mark,
            added_later: false,
            target: None,
            tag: None,
        }
    }

    #[test]
    fn the_chip_strip_keeps_day_counts_in_the_unmarked_view() -> Result<(), werdy_core::Error> {
        let day = WerdyDay::new(2026, 11, 26)?;
        let full = DailySheet {
            day,
            slots: vec![
                SheetSlot {
                    slot: SlotKey("asr".into()),
                    rows: vec![row(1, Some(Mark::Missed)), row(2, None)],
                    complete: false,
                    all_entered: false,
                },
                SheetSlot {
                    slot: SlotKey("quran".into()),
                    rows: vec![row(3, Some(Mark::Done))],
                    complete: true,
                    all_entered: true,
                },
            ],
        };
        let shown = full.clone().unmarked_only();
        let sections = section_views(shown, Some(&full), Some("asr"));
        let chips: Vec<_> = sections
            .iter()
            .map(|section| {
                (
                    section.slot.slot.0.as_str(),
                    section.progress.marked,
                    section.progress.total,
                    section.current,
                )
            })
            .collect();
        assert_eq!(chips, [("asr", 1, 2, true)]);
        assert_eq!(
            section_views(full, None, Some("isha"))
                .iter()
                .filter(|section| section.current)
                .count(),
            0
        );
        Ok(())
    }

    #[test]
    fn a_new_werdy_day_discards_fold_choices_even_when_viewing_the_same_past_day()
    -> Result<(), werdy_core::Error> {
        let today = WerdyDay::new(2026, 11, 26)?;
        let tomorrow = WerdyDay::new(2026, 11, 27)?;
        let past = WerdyDay::new(2026, 11, 25)?;
        let fajr = SlotKey("fajr".into());
        let asr = SlotKey("asr".into());
        let mut choices = FoldChoices::new(today);

        choices.set(today, past, [&fajr], false);
        choices.set(today, past, [&asr], true);
        assert!(!choices.open(today, past, &fajr, false));
        assert!(choices.open(today, past, &asr, true));
        assert!(choices.open(today, today, &fajr, false));

        choices.reset_for(tomorrow);
        assert!(choices.open(tomorrow, past, &fajr, false));
        assert!(!choices.open(tomorrow, past, &asr, true));
        choices.reset_for(today);
        assert!(choices.open(today, past, &fajr, false));
        Ok(())
    }

    #[test]
    fn every_mark_counts_including_misses_and_zero_counts() {
        let rows = [
            row(1, None),
            row(2, Some(Mark::Done)),
            row(3, Some(Mark::Missed)),
            row(4, Some(Mark::Count(0))),
            row(5, Some(Mark::partial_without_target())),
            row(6, Some(Mark::Count(120))),
        ];
        assert_eq!(section_count(&rows), (5, 6));
    }

    #[test]
    fn an_unmarked_when_it_comes_up_deed_is_left_out_of_the_section_count() {
        let comes_up = |id, mark| SheetRow {
            tag: Some(RowTag::WhenItComesUp),
            ..row(id, mark)
        };
        let rows = [
            row(1, Some(Mark::Done)),
            row(2, None),
            comes_up(3, None),
            comes_up(4, Some(Mark::Missed)),
        ];
        assert_eq!(section_count(&rows), (2, 3));
    }

    #[test]
    fn the_fold_all_button_opens_only_when_every_slot_is_folded() {
        assert!(!fold_all_opens(&[true, false, true]));
        assert!(!fold_all_opens(&[true]));
        assert!(fold_all_opens(&[false, false]));
        // With no slots there is nothing to open.
        assert!(!fold_all_opens(&[]));
    }

    fn keys(names: &[&str]) -> Vec<SlotKey> {
        names
            .iter()
            .map(|name| SlotKey((*name).to_owned()))
            .collect()
    }

    #[test]
    fn the_initial_jump_uses_the_wanted_section_or_the_next_one_with_deeds() {
        let order = keys(&[
            "fajr", "morning", "dhuhr", "asr", "maghrib", "isha", "night",
        ]);
        let shown = keys(&["fajr", "dhuhr", "isha"]);
        assert_eq!(nearest_slot(&shown, &order, "dhuhr"), Some(&shown[1]));
        assert_eq!(nearest_slot(&shown, &order, "asr"), Some(&shown[2]));
        assert_eq!(nearest_slot(&shown, &order, "morning"), Some(&shown[1]));
        // Past the last slot with deeds, the last one stays current.
        assert_eq!(nearest_slot(&shown, &order, "night"), Some(&shown[2]));
        assert_eq!(nearest_slot(&[], &order, "night"), None);
    }
}
