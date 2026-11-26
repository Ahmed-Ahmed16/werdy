//! The daily sheet: due deeds for one werdy day, grouped by time slot.

use std::collections::BTreeMap;

use crate::{
    CalendarSettings, Deed, DeedId, Fraction, Mark, MarkEntry, Schedule, SlotKey, Target, WerdyDay,
};

/// One due deed on the sheet with its mark for that day.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetRow {
    /// Stable deed identity.
    pub deed_id: DeedId,
    /// The owner's mark for that day, if any.
    pub mark: Option<Mark>,
    /// Whether the mark was entered more than two days after this day.
    pub added_later: bool,
    /// Target in force that day, if the deed uses counts.
    pub target: Option<Target>,
    /// Quiet tag that explains the deed's schedule on this row, if any.
    pub tag: Option<RowTag>,
}

impl SheetRow {
    /// Says whether the row counts in its section's count and folding. An
    /// unmarked when it comes up deed does not (decisions 0046 and 0047).
    pub fn counts_in_section(&self) -> bool {
        self.mark.is_some() || self.tag != Some(RowTag::WhenItComesUp)
    }
}

/// Quiet tag drawn on a sheet row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowTag {
    /// A once a week deed with no mark yet this week (decision 0047).
    ThisWeek,
    /// A when it comes up deed, due only on a day it has a mark (decision 0047).
    WhenItComesUp,
}

/// Due deeds in one time slot.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetSlot {
    /// Time slot key.
    pub slot: SlotKey,
    /// Rows in the order the deeds were given.
    pub rows: Vec<SheetRow>,
    /// True when the slot has counted deeds and every one earned full credit.
    pub complete: bool,
    /// True when every counted deed in this slot has a mark for the day, so
    /// the slot folds by itself. See `SheetRow::counts_in_section`.
    pub all_entered: bool,
}

/// Due deeds for one werdy day, grouped by time slot. Empty slots are left out.
#[derive(Clone, Debug, PartialEq)]
pub struct DailySheet {
    /// Day this sheet shows.
    pub day: WerdyDay,
    /// Slots in sheet order.
    pub slots: Vec<SheetSlot>,
}

impl DailySheet {
    /// Keeps only counted rows with no mark yet and drops slots left empty.
    /// On today the rows kept match the open count of `today_summary`.
    pub fn unmarked_only(mut self) -> Self {
        for slot in &mut self.slots {
            slot.rows
                .retain(|row| row.mark.is_none() && row.counts_in_section());
        }
        self.slots.retain(|slot| !slot.rows.is_empty());
        self
    }
}

/// Builds the sheet for a day. Slots follow `slot_order`; slots missing from it
/// come last in key order, so no due deed is ever hidden.
pub fn daily_sheet(
    deeds: &[Deed],
    marks: &BTreeMap<(DeedId, WerdyDay), MarkEntry>,
    day: WerdyDay,
    slot_order: &[SlotKey],
    calendar: CalendarSettings,
) -> DailySheet {
    // Rows, and whether every counted row so far earned full credit.
    let mut groups: BTreeMap<(usize, SlotKey), (Vec<SheetRow>, bool)> = BTreeMap::new();
    for deed in deeds {
        let Some((rule, _)) = deed.sheet_rule_on(day, &calendar, marks) else {
            continue;
        };
        let entry = marks.get(&(deed.id, day));
        let mark = entry.map(|entry| entry.mark);
        let done = mark.is_some_and(|mark| mark.exact_value(rule) == Fraction::ONE);
        let position = slot_order
            .iter()
            .position(|slot| *slot == rule.slot)
            .unwrap_or(slot_order.len());
        let group = groups
            .entry((position, rule.slot.clone()))
            .or_insert_with(|| (Vec::new(), true));
        let row = SheetRow {
            deed_id: deed.id,
            mark,
            added_later: entry.is_some_and(|entry| entry.is_late_for(day)),
            target: rule.target.clone(),
            tag: row_tag(&rule.schedule, mark.is_some()),
        };
        if row.counts_in_section() {
            group.1 &= done;
        }
        group.0.push(row);
    }
    DailySheet {
        day,
        slots: groups
            .into_iter()
            .map(|((_, slot), (rows, all_done))| {
                let has_counted = rows.iter().any(SheetRow::counts_in_section);
                SheetSlot {
                    slot,
                    all_entered: rows
                        .iter()
                        .filter(|row| row.counts_in_section())
                        .all(|row| row.mark.is_some()),
                    complete: all_done && has_counted,
                    rows,
                }
            })
            .collect(),
    }
}

fn row_tag(schedule: &Schedule, marked: bool) -> Option<RowTag> {
    match schedule {
        Schedule::OnceAWeek if !marked => Some(RowTag::ThisWeek),
        Schedule::WhenItComesUp => Some(RowTag::WhenItComesUp),
        _ => None,
    }
}
