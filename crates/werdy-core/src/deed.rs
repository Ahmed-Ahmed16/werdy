use std::collections::{BTreeMap, BTreeSet};

use crate::{Error, Mark, Weekday, WerdyDay};

/// Rule that says which calendar days a deed is due.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Schedule {
    /// Due every day.
    EveryDay,
    /// Due on the listed weekdays.
    Weekdays(BTreeSet<Weekday>),
}

impl Schedule {
    /// Says whether the schedule includes this day.
    pub fn includes(&self, day: WerdyDay) -> bool {
        match self {
            Self::EveryDay => true,
            Self::Weekdays(weekdays) => weekdays.contains(&day.weekday()),
        }
    }
}

// Hijri date ranges belong to a later ticket.

/// Stable identity for a deed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DeedId(pub u64);

/// Fields that may change for a deed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeedRule {
    /// Days when this deed is due.
    pub schedule: Schedule,
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

    /// Says whether this deed is due on this day.
    pub fn is_due_on(&self, day: WerdyDay) -> bool {
        self.archived_from.is_none_or(|archived| day < archived)
            && self
                .rule_on(day)
                .is_some_and(|rule| rule.schedule.includes(day))
    }
}

/// Removes a deed and all its marks from caller owned collections.
pub fn delete_deed(
    deeds: &mut Vec<Deed>,
    marks: &mut BTreeMap<(DeedId, WerdyDay), Mark>,
    id: DeedId,
) {
    deeds.retain(|deed| deed.id != id);
    marks.retain(|(deed_id, _), _| *deed_id != id);
}
