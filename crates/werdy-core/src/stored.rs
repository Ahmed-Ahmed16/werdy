//! Checked shapes for loading stored data. Each one goes through the same
//! rules as the normal constructors, so bad data returns an error.

use crate::deed::RuleVersion;
use crate::{DayNote, Deed, DeedId, Error, ExcuseCredit, Target, WerdyDay};

#[derive(serde::Deserialize)]
pub(crate) struct TargetData {
    amount: u32,
    unit: String,
}

impl TryFrom<TargetData> for Target {
    type Error = Error;

    fn try_from(data: TargetData) -> Result<Self, Error> {
        Self::new(data.amount, &data.unit)
    }
}

#[derive(serde::Deserialize)]
pub(crate) struct DayNoteData {
    text: String,
    credit: ExcuseCredit,
    entered_on: WerdyDay,
}

impl TryFrom<DayNoteData> for DayNote {
    type Error = Error;

    fn try_from(data: DayNoteData) -> Result<Self, Error> {
        Self::new(&data.text, data.credit, data.entered_on)
    }
}

#[derive(serde::Deserialize)]
pub(crate) struct DeedData {
    id: DeedId,
    first_rule: RuleVersion,
    later_rules: Vec<RuleVersion>,
    archived_from: Option<WerdyDay>,
}

impl TryFrom<DeedData> for Deed {
    type Error = Error;

    /// Rebuilds the deed through `change_rule` and `archive`, so stored
    /// history obeys the same order rules as history made in the app.
    fn try_from(data: DeedData) -> Result<Self, Error> {
        let mut deed = Deed::new(data.id, data.first_rule.from_day, data.first_rule.rule);
        let mut previous = deed.created_on();
        for version in data.later_rules {
            if version.from_day <= previous {
                return Err(Error::RuleBeforeLatest);
            }
            previous = version.from_day;
            deed.change_rule(version.from_day, version.rule)?;
        }
        if let Some(day) = data.archived_from {
            deed.archive(day)?;
        }
        Ok(deed)
    }
}
