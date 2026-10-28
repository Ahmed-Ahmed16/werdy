use std::collections::{BTreeMap, BTreeSet};

use crate::{CalendarSettings, Error, HijriRange, MarkEntry, Weekday, WerdyDay};

/// Rule that says which calendar days a deed is due.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Schedule {
    /// Due every day.
    EveryDay,
    /// Due on the listed weekdays.
    Weekdays(BTreeSet<Weekday>),
    /// Due on the Hijri days of this range.
    Hijri(HijriRange),
}

impl Schedule {
    /// Says whether the schedule includes this day under the owner's calendar settings.
    pub fn includes(&self, day: WerdyDay, calendar: &CalendarSettings) -> bool {
        match self {
            Self::EveryDay => true,
            Self::Weekdays(weekdays) => weekdays.contains(&day.weekday()),
            Self::Hijri(range) => day
                .hijri(calendar.offset_on(day))
                .is_ok_and(|date| range.contains(date)),
        }
    }
}

/// Stable identity for a deed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct DeedId(pub u64);

/// Key for a category stored as deed data.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct CategoryKey(pub String);

impl CategoryKey {
    /// Creates a category key from stored text.
    pub fn new(key: &str) -> Self {
        Self(key.to_owned())
    }
}

/// Key for a time slot stored as deed data, such as `fajr` or `any_time`.
/// The order of slots on the sheet is data too, given by the caller.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct SlotKey(pub String);

impl SlotKey {
    /// Creates a time slot key from stored text.
    pub fn new(key: &str) -> Self {
        Self(key.to_owned())
    }
}

/// Positive daily amount with a short unit key, not translated UI text.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "crate::stored::TargetData"))]
pub struct Target {
    amount: u32,
    unit: String,
}

impl Target {
    /// Creates a target with a positive amount and a nonempty unit key.
    pub fn new(amount: u32, unit: &str) -> Result<Self, Error> {
        if amount == 0 {
            return Err(Error::InvalidTargetAmount);
        }
        if unit.is_empty() {
            return Err(Error::EmptyTargetUnit);
        }
        Ok(Self {
            amount,
            unit: unit.to_owned(),
        })
    }

    /// Returns the daily amount.
    pub fn amount(&self) -> u32 {
        self.amount
    }

    /// Returns the unit key for later Fluent lookup.
    pub fn unit(&self) -> &str {
        &self.unit
    }
}

/// Raises a target amount by one without overflow.
pub fn increase_target_amount(amount: u32) -> u32 {
    amount.saturating_add(1)
}

/// Lowers a target amount by one while keeping it positive.
pub fn decrease_target_amount(amount: u32) -> u32 {
    amount.saturating_sub(1).max(1)
}

/// Fields that may change for a deed.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeedRule {
    /// Days when this deed is due.
    pub schedule: Schedule,
    /// Daily target, if this deed uses counts.
    pub target: Option<Target>,
    /// Category in force for this rule version.
    pub category: CategoryKey,
    /// Time slot in force for this rule version.
    pub slot: SlotKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct RuleVersion {
    pub(crate) from_day: WerdyDay,
    pub(crate) rule: DeedRule,
}

/// Deed with dated rules and optional archive day.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "crate::stored::DeedData"))]
pub struct Deed {
    /// Stable deed identity.
    pub id: DeedId,
    pub(crate) first_rule: RuleVersion,
    pub(crate) later_rules: Vec<RuleVersion>,
    pub(crate) archived_from: Option<WerdyDay>,
    pub(crate) past_archives: Vec<(WerdyDay, WerdyDay)>,
}

impl Deed {
    /// Dated rules in ascending day order, for lossless sync and export.
    pub fn rule_versions(&self) -> impl Iterator<Item = (WerdyDay, &DeedRule)> {
        std::iter::once(&self.first_rule)
            .chain(&self.later_rules)
            .map(|version| (version.from_day, &version.rule))
    }

    /// Closed archive periods. The end day is active again.
    pub fn past_archives(&self) -> &[(WerdyDay, WerdyDay)] {
        &self.past_archives
    }

    /// Creates a deed with its first rule from its creation day.
    pub fn new(id: DeedId, created_on: WerdyDay, rule: DeedRule) -> Self {
        Self {
            id,
            first_rule: RuleVersion {
                from_day: created_on,
                rule,
            },
            later_rules: Vec::new(),
            archived_from: None,
            past_archives: Vec::new(),
        }
    }

    /// Returns the werdy day this deed was created.
    pub fn created_on(&self) -> WerdyDay {
        self.first_rule.from_day
    }

    /// Returns the day archiving starts, if any.
    pub fn archived_from(&self) -> Option<WerdyDay> {
        self.archived_from
    }

    /// Says whether an archive period covers this day.
    pub fn is_archived_on(&self, day: WerdyDay) -> bool {
        self.archived_from.is_some_and(|archived| day >= archived)
            || self
                .past_archives
                .iter()
                .any(|(start, end)| *start <= day && day < *end)
    }

    /// Changes the rule from this day onward. Replaces an edit on the same day.
    /// Errors if the day precedes creation or the latest rule.
    pub fn change_rule(&mut self, from_day: WerdyDay, rule: DeedRule) -> Result<(), Error> {
        if from_day < self.created_on() {
            return Err(Error::RuleBeforeCreation);
        }
        let latest_day = self
            .later_rules
            .last()
            .map_or(self.created_on(), |version| version.from_day);
        if from_day < latest_day {
            return Err(Error::RuleBeforeLatest);
        }
        if from_day == latest_day {
            self.replace_latest_rule(rule);
        } else {
            self.later_rules.push(RuleVersion { from_day, rule });
        }
        Ok(())
    }

    fn replace_latest_rule(&mut self, rule: DeedRule) {
        if let Some(latest) = self.later_rules.last_mut() {
            latest.rule = rule;
        } else {
            self.first_rule.rule = rule;
        }
    }

    /// Stops this deed being due from this day.
    /// Errors before creation or if the deed is already archived.
    pub fn archive(&mut self, from_day: WerdyDay) -> Result<(), Error> {
        if from_day < self.created_on() {
            return Err(Error::ArchiveBeforeCreation);
        }
        if self.archived_from.is_some() {
            return Err(Error::AlreadyArchived);
        }
        if self
            .past_archives
            .last()
            .is_some_and(|(_, end)| from_day < *end)
        {
            return Err(Error::ArchiveBeforeCreation);
        }
        self.archived_from = Some(from_day);
        Ok(())
    }

    /// Ends an archive period today. Earlier days keep their due status.
    pub fn restore(&mut self, from_day: WerdyDay) -> Result<(), Error> {
        let start = self.archived_from.ok_or(Error::NotArchived)?;
        if from_day < start {
            return Err(Error::RestoreBeforeArchive);
        }
        if from_day > start {
            self.past_archives.push((start, from_day));
        }
        self.archived_from = None;
        Ok(())
    }

    /// Returns the rule in force on this day, if the deed existed.
    pub fn rule_on(&self, day: WerdyDay) -> Option<&DeedRule> {
        if day < self.created_on() {
            return None;
        }
        self.later_rules
            .iter()
            .rev()
            .find(|version| version.from_day <= day)
            .map(|version| &version.rule)
            .or(Some(&self.first_rule.rule))
    }

    /// Says whether this deed is due on this day under the owner's calendar settings.
    pub fn is_due_on(&self, day: WerdyDay, calendar: &CalendarSettings) -> bool {
        self.due_rule_on(day, calendar).is_some()
    }

    /// Returns the rule if the deed is due on this day.
    pub(crate) fn due_rule_on(
        &self,
        day: WerdyDay,
        calendar: &CalendarSettings,
    ) -> Option<&DeedRule> {
        if self.is_archived_on(day) {
            return None;
        }
        self.rule_on(day)
            .filter(|rule| rule.schedule.includes(day, calendar))
    }
}

/// Removes a deed and all its marks from caller owned collections.
pub fn delete_deed(
    deeds: &mut Vec<Deed>,
    marks: &mut BTreeMap<(DeedId, WerdyDay), MarkEntry>,
    id: DeedId,
) {
    deeds.retain(|deed| deed.id != id);
    marks.retain(|(deed_id, _), _| *deed_id != id);
}

/// Number of deeds active on this day, including deeds not due today.
pub fn active_deed_count(deeds: &[Deed], day: WerdyDay) -> usize {
    deeds
        .iter()
        .filter(|deed| deed.created_on() <= day && !deed.is_archived_on(day))
        .count()
}

/// Number of deeds archived on this day.
pub fn archived_deed_count(deeds: &[Deed], day: WerdyDay) -> usize {
    deeds.iter().filter(|deed| deed.is_archived_on(day)).count()
}
