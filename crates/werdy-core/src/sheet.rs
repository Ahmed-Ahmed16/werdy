//! The daily sheet: due deeds for one werdy day, grouped by time slot.

use std::collections::BTreeMap;

use crate::{CalendarSettings, Deed, DeedId, Fraction, Mark, MarkEntry, SlotKey, Target, WerdyDay};

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
}

/// Due deeds in one time slot.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetSlot {
    /// Time slot key.
    pub slot: SlotKey,
    /// Rows in the order the deeds were given.
    pub rows: Vec<SheetRow>,
    /// True when every deed in this slot is worth one, so the slot can fold.
    pub folded: bool,
}

/// Due deeds for one werdy day, grouped by time slot. Empty slots are left out.
#[derive(Clone, Debug, PartialEq)]
pub struct DailySheet {
    /// Day this sheet shows.
    pub day: WerdyDay,
    /// Slots in sheet order.
    pub slots: Vec<SheetSlot>,
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
    let mut groups: BTreeMap<(usize, SlotKey), (Vec<SheetRow>, bool)> = BTreeMap::new();
    for deed in deeds {
        let Some(rule) = deed.due_rule_on(day, &calendar) else {
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
        group.0.push(SheetRow {
            deed_id: deed.id,
            mark,
            added_later: entry.is_some_and(|entry| entry.is_late_for(day)),
            target: rule.target.clone(),
        });
        group.1 &= done;
    }
    DailySheet {
        day,
        slots: groups
            .into_iter()
            .map(|((_, slot), (rows, folded))| SheetSlot { slot, rows, folded })
            .collect(),
    }
}
