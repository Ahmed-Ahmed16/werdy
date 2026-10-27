use std::collections::{BTreeMap, BTreeSet};

use crate::{CalendarSettings, Error, HijriRange, MarkEntry, Weekday, WerdyDay};

/// Rule that says which calendar days a deed is due.
#[derive(Clone, Debug, Eq, PartialEq)]
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
pub struct DeedId(pub u64);

/// Key for a category stored as deed data.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CategoryKey(pub String);

impl CategoryKey {
    /// Creates a category key from stored text.
    pub fn new(key: &str) -> Self {
        Self(key.to_owned())
    }
}

/// Positive daily amount with a short unit key, not translated UI text.
#[derive(Clone, Debug, Eq, PartialEq)]
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

/// Fields that may change for a deed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeedRule {
    /// Days when this deed is due.
    pub schedule: Schedule,
    /// Daily target, if this deed uses counts.
    pub target: Option<Target>,
    /// Category in force for this rule version.
    pub category: CategoryKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuleVersion {
    from_day: WerdyDay,
    rule: DeedRule,
}

/// Deed with dated rules and optional archive day.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deed {
    /// Stable deed identity.
    pub id: DeedId,
    first_rule: RuleVersion,
    later_rules: Vec<RuleVersion>,
    archived_from: Option<WerdyDay>,
}

impl Deed {
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
        self.archived_from = Some(from_day);
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
        if self.archived_from.is_some_and(|archived| day >= archived) {
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
