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
    /// Due on one day of each Saturday to Friday week: the first day it has a
    /// mark, or Friday when the week has none (decision 0047).
    OnceAWeek,
    /// Due only on a day it has a mark. Unmarked, it is never Missed (decision 0047).
    WhenItComesUp,
}

impl Schedule {
    /// Says whether a calendar schedule includes this day under the owner's
    /// calendar settings. A schedule whose due day follows the owner's marks,
    /// such as once a week, includes no day here; `Deed::status_on` decides it.
    fn includes(&self, day: WerdyDay, calendar: &CalendarSettings) -> bool {
        match self {
            Self::EveryDay => true,
            Self::Weekdays(weekdays) => weekdays.contains(&day.weekday()),
            Self::Hijri(range) => day
                .hijri(calendar.offset_on(day))
                .is_ok_and(|date| range.contains(date)),
            Self::OnceAWeek | Self::WhenItComesUp => false,
        }
    }
}

/// Marks keyed by deed and werdy day.
pub type Marks = BTreeMap<(DeedId, WerdyDay), MarkEntry>;

/// How a deed stands on one werdy day.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DayStatus {
    /// Not on that day's sheet and not scored that day.
    NotDue,
    /// On the sheet and scored that day. Left unmarked, the finished day counts it as Missed.
    Due,
    /// A once a week deed with no mark yet this week, on a day before Friday.
    /// It is on the sheet and open, but it is not scored that day and is never Missed.
    OpenThisWeek,
    /// A when it comes up deed with no mark that day. It is on the sheet, but
    /// it is not scored, not open in today's line, and never Missed.
    IfItComesUp,
}

impl DayStatus {
    /// Says whether the deed shows on that day's sheet.
    pub fn is_on_sheet(self) -> bool {
        self != Self::NotDue
    }

    /// Says whether the owner is expected to mark the deed that day, so an
    /// unmarked one is open in today's line and in its section's count.
    pub fn is_expected(self) -> bool {
        matches!(self, Self::Due | Self::OpenThisWeek)
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

    /// Says whether this deed is due on this day under the owner's calendar
    /// settings. Marks matter for a once a week deed, whose due day is the
    /// first day it is marked that week.
    pub fn is_due_on(&self, day: WerdyDay, calendar: &CalendarSettings, marks: &Marks) -> bool {
        self.status_on(day, calendar, marks) == DayStatus::Due
    }

    /// Says how this deed stands on this day: due, open this week, or not due.
    pub fn status_on(
        &self,
        day: WerdyDay,
        calendar: &CalendarSettings,
        marks: &Marks,
    ) -> DayStatus {
        let Some(rule) = self.active_rule_on(day) else {
            return DayStatus::NotDue;
        };
        match &rule.schedule {
            Schedule::OnceAWeek => self.weekly_status_on(day, marks),
            Schedule::WhenItComesUp if marks.contains_key(&(self.id, day)) => DayStatus::Due,
            Schedule::WhenItComesUp => DayStatus::IfItComesUp,
            schedule if schedule.includes(day, calendar) => DayStatus::Due,
            _ => DayStatus::NotDue,
        }
    }

    /// Returns the rule if the deed is due on this day.
    pub(crate) fn due_rule_on(
        &self,
        day: WerdyDay,
        calendar: &CalendarSettings,
        marks: &Marks,
    ) -> Option<&DeedRule> {
        self.sheet_rule_on(day, calendar, marks)
            .filter(|(_, status)| *status == DayStatus::Due)
            .map(|(rule, _)| rule)
    }

    /// Returns the rule and status if the deed shows on this day's sheet.
    pub(crate) fn sheet_rule_on(
        &self,
        day: WerdyDay,
        calendar: &CalendarSettings,
        marks: &Marks,
    ) -> Option<(&DeedRule, DayStatus)> {
        let status = self.status_on(day, calendar, marks);
        if !status.is_on_sheet() {
            return None;
        }
        self.rule_on(day).map(|rule| (rule, status))
    }

    fn active_rule_on(&self, day: WerdyDay) -> Option<&DeedRule> {
        if self.is_archived_on(day) {
            return None;
        }
        self.rule_on(day)
    }

    /// The week's due day is the first day with a mark while the deed was once
    /// a week and active, or Friday when there is none.
    fn weekly_status_on(&self, day: WerdyDay, marks: &Marks) -> DayStatus {
        let since_saturday = day.days_since_saturday();
        let first_marked = (0..7)
            .filter_map(|offset| day.checked_add_days(offset - since_saturday))
            .find(|week_day| {
                marks.contains_key(&(self.id, *week_day))
                    && self
                        .active_rule_on(*week_day)
                        .is_some_and(|rule| rule.schedule == Schedule::OnceAWeek)
            });
        match first_marked {
            Some(due_day) if due_day == day => DayStatus::Due,
            Some(_) => DayStatus::NotDue,
            None if since_saturday == 6 => DayStatus::Due,
            None => DayStatus::OpenThisWeek,
        }
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
