//! Loads werdy's starter sets from `data/starter-sets.toml` and checks them.
//!
//! The data file lists the known time slots, categories, Hijri ranges and
//! units, then every deed once, then each set as a list of deed keys. Two
//! sets that share a deed name the same deed key, so adding both sets adds
//! the deed once. Names are Fluent message keys, looked up in
//! `locales/<lang>/starter-sets.ftl`.

use std::collections::BTreeSet;
use std::fmt;

use serde::Deserialize;

const BUILTIN_DATA: &str = include_str!("../../../data/starter-sets.toml");
const BUILTIN_AR: &str = include_str!("../../../locales/ar/starter-sets.ftl");
const BUILTIN_EN: &str = include_str!("../../../locales/en/starter-sets.ftl");

/// Everything in one starter sets file, after it passed the checks.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    /// Known time slot keys, in sheet order.
    pub slots: Vec<String>,
    /// Known category keys.
    pub categories: Vec<String>,
    /// Known Hijri range keys.
    pub hijri_ranges: Vec<String>,
    /// Known target units.
    pub units: Vec<Unit>,
    /// Every deed, each once.
    pub deeds: Vec<Deed>,
    /// The starter sets, in the order they are offered.
    pub sets: Vec<StarterSet>,
}

/// A target unit such as pages or times.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    /// Stable unit key stored on a target.
    pub key: String,
    /// Fluent key for the unit's name.
    pub name: String,
}

/// One deed a starter set can add.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Deed {
    /// Stable deed key, unique in the file.
    pub key: String,
    /// Fluent key for the deed's name.
    pub name: String,
    /// Time slot key.
    pub slot: String,
    /// Category key.
    pub category: String,
    /// Days the deed is due.
    pub schedule: Schedule,
    /// Optional daily target.
    #[serde(default)]
    pub target: Option<Target>,
}

/// Rule for the days a deed is due.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Schedule {
    /// Due every day. Written as `"every_day"`.
    EveryDay,
    /// Due on the listed weekdays. Written as `{ weekdays = ["friday"] }`.
    Weekdays(Vec<Weekday>),
    /// Due inside a Hijri range. Written as `{ hijri = "ramadan" }`.
    Hijri(String),
}

/// Day of the week, written in lower case in the file.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// Daily amount with a unit key.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// Positive daily amount.
    pub amount: u32,
    /// Unit key from the known units.
    pub unit: String,
}

/// A named list of deeds that fills a sheet in one tap.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StarterSet {
    /// Stable set key, unique in the file.
    pub key: String,
    /// Fluent key for the set's name.
    pub name: String,
    /// Deed keys in the order they are added.
    pub deeds: Vec<String>,
}

/// What can be wrong with a starter sets file or its names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// The text is not valid TOML or does not match the format.
    Parse(String),
    /// A key appears twice. `kind` says which list it is in.
    DuplicateKey {
        /// The list, such as "deed" or "set".
        kind: &'static str,
        /// The repeated key.
        key: String,
    },
    /// Two entries share one Fluent name key.
    DuplicateName {
        /// The repeated name key.
        name: String,
    },
    /// A deed uses a slot that is not in the known list.
    UnknownSlot {
        /// The deed key.
        deed: String,
        /// The unknown slot.
        slot: String,
    },
    /// A deed uses a category that is not in the known list.
    UnknownCategory {
        /// The deed key.
        deed: String,
        /// The unknown category.
        category: String,
    },
    /// A deed uses a Hijri range that is not in the known list.
    UnknownHijriRange {
        /// The deed key.
        deed: String,
        /// The unknown range.
        range: String,
    },
    /// A deed target uses a unit that is not in the known list.
    UnknownUnit {
        /// The deed key.
        deed: String,
        /// The unknown unit.
        unit: String,
    },
    /// A deed target has an amount of zero.
    ZeroTarget {
        /// The deed key.
        deed: String,
    },
    /// A weekday schedule lists no days.
    NoWeekdays {
        /// The deed key.
        deed: String,
    },
    /// A set lists no deeds.
    EmptySet {
        /// The set key.
        set: String,
    },
    /// A set lists a deed key that is not defined.
    UnknownDeedInSet {
        /// The set key.
        set: String,
        /// The unknown deed key.
        deed: String,
    },
    /// A set lists the same deed twice.
    DeedTwiceInSet {
        /// The set key.
        set: String,
        /// The repeated deed key.
        deed: String,
    },
    /// A name key has no message in one locale file.
    MissingName {
        /// The locale, such as "ar" or "en".
        locale: &'static str,
        /// The missing name key.
        name: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "starter sets file does not parse: {message}"),
            Self::DuplicateKey { kind, key } => write!(f, "{kind} key {key} appears twice"),
            Self::DuplicateName { name } => write!(f, "name key {name} appears twice"),
            Self::UnknownSlot { deed, slot } => write!(f, "deed {deed} uses unknown slot {slot}"),
            Self::UnknownCategory { deed, category } => {
                write!(f, "deed {deed} uses unknown category {category}")
            }
            Self::UnknownHijriRange { deed, range } => {
                write!(f, "deed {deed} uses unknown Hijri range {range}")
            }
            Self::UnknownUnit { deed, unit } => write!(f, "deed {deed} uses unknown unit {unit}"),
            Self::ZeroTarget { deed } => write!(f, "deed {deed} has a target of zero"),
            Self::NoWeekdays { deed } => write!(f, "deed {deed} lists no weekdays"),
            Self::EmptySet { set } => write!(f, "set {set} lists no deeds"),
            Self::UnknownDeedInSet { set, deed } => {
                write!(f, "set {set} lists unknown deed {deed}")
            }
            Self::DeedTwiceInSet { set, deed } => write!(f, "set {set} lists deed {deed} twice"),
            Self::MissingName { locale, name } => write!(f, "locale {locale} has no {name}"),
        }
    }
}

impl std::error::Error for Error {}

impl Catalog {
    /// Finds a deed by key.
    pub fn deed(&self, key: &str) -> Option<&Deed> {
        self.deeds.iter().find(|deed| deed.key == key)
    }

    /// Finds a set by key.
    pub fn set(&self, key: &str) -> Option<&StarterSet> {
        self.sets.iter().find(|set| set.key == key)
    }

    /// Returns the deeds of a set, in set order. A checked catalog knows them all.
    pub fn deeds_in(&self, set: &StarterSet) -> Vec<&Deed> {
        set.deeds.iter().filter_map(|key| self.deed(key)).collect()
    }

    /// Every Fluent name key the file uses.
    pub fn name_keys(&self) -> impl Iterator<Item = &str> {
        let units = self.units.iter().map(|unit| unit.name.as_str());
        let deeds = self.deeds.iter().map(|deed| deed.name.as_str());
        let sets = self.sets.iter().map(|set| set.name.as_str());
        units.chain(deeds).chain(sets)
    }
}

/// Parses a starter sets file and checks its keys. Names are not checked here.
pub fn parse(text: &str) -> Result<Catalog, Error> {
    let catalog: Catalog = toml::from_str(text).map_err(|error| Error::Parse(error.to_string()))?;
    check_keys(&catalog)?;
    Ok(catalog)
}

/// Parses a starter sets file and checks that both locales name everything.
pub fn load(text: &str, ar_ftl: &str, en_ftl: &str) -> Result<Catalog, Error> {
    let catalog = parse(text)?;
    check_names(&catalog, "ar", ar_ftl)?;
    check_names(&catalog, "en", en_ftl)?;
    Ok(catalog)
}

/// Loads the starter sets shipped with werdy.
pub fn builtin() -> Result<Catalog, Error> {
    load(BUILTIN_DATA, BUILTIN_AR, BUILTIN_EN)
}

fn check_keys(catalog: &Catalog) -> Result<(), Error> {
    unique("slot", catalog.slots.iter().map(String::as_str))?;
    unique("category", catalog.categories.iter().map(String::as_str))?;
    unique(
        "hijri range",
        catalog.hijri_ranges.iter().map(String::as_str),
    )?;
    unique("unit", catalog.units.iter().map(|unit| unit.key.as_str()))?;
    unique("deed", catalog.deeds.iter().map(|deed| deed.key.as_str()))?;
    unique("set", catalog.sets.iter().map(|set| set.key.as_str()))?;

    let mut names = BTreeSet::new();
    for name in catalog.name_keys() {
        if !names.insert(name) {
            return Err(Error::DuplicateName { name: name.into() });
        }
    }

    for deed in &catalog.deeds {
        check_deed(catalog, deed)?;
    }
    for set in &catalog.sets {
        check_set(catalog, set)?;
    }
    Ok(())
}

fn unique<'a>(kind: &'static str, keys: impl Iterator<Item = &'a str>) -> Result<(), Error> {
    let mut seen = BTreeSet::new();
    for key in keys {
        if !seen.insert(key) {
            return Err(Error::DuplicateKey {
                kind,
                key: key.into(),
            });
        }
    }
    Ok(())
}

fn check_deed(catalog: &Catalog, deed: &Deed) -> Result<(), Error> {
    let name = || deed.key.clone();
    if !catalog.slots.contains(&deed.slot) {
        return Err(Error::UnknownSlot {
            deed: name(),
            slot: deed.slot.clone(),
        });
    }
    if !catalog.categories.contains(&deed.category) {
        return Err(Error::UnknownCategory {
            deed: name(),
            category: deed.category.clone(),
        });
    }
    match &deed.schedule {
        Schedule::EveryDay => {}
        Schedule::Weekdays(days) if days.is_empty() => {
            return Err(Error::NoWeekdays { deed: name() });
        }
        Schedule::Weekdays(_) => {}
        Schedule::Hijri(range) if !catalog.hijri_ranges.contains(range) => {
            return Err(Error::UnknownHijriRange {
                deed: name(),
                range: range.clone(),
            });
        }
        Schedule::Hijri(_) => {}
    }
    if let Some(target) = &deed.target {
        if target.amount == 0 {
            return Err(Error::ZeroTarget { deed: name() });
        }
        if !catalog.units.iter().any(|unit| unit.key == target.unit) {
            return Err(Error::UnknownUnit {
                deed: name(),
                unit: target.unit.clone(),
            });
        }
    }
    Ok(())
}

fn check_set(catalog: &Catalog, set: &StarterSet) -> Result<(), Error> {
    if set.deeds.is_empty() {
        return Err(Error::EmptySet {
            set: set.key.clone(),
        });
    }
    let mut seen = BTreeSet::new();
    for key in &set.deeds {
        if catalog.deed(key).is_none() {
            return Err(Error::UnknownDeedInSet {
                set: set.key.clone(),
                deed: key.clone(),
            });
        }
        if !seen.insert(key) {
            return Err(Error::DeedTwiceInSet {
                set: set.key.clone(),
                deed: key.clone(),
            });
        }
    }
    Ok(())
}

fn check_names(catalog: &Catalog, locale: &'static str, ftl: &str) -> Result<(), Error> {
    let defined = message_ids(ftl);
    match catalog.name_keys().find(|name| !defined.contains(name)) {
        Some(name) => Err(Error::MissingName {
            locale,
            name: name.into(),
        }),
        None => Ok(()),
    }
}

/// Reads the message ids from Fluent text. A message starts at the first
/// column with its id and an equals sign. Comments, blank lines and indented
/// lines are skipped. This is enough to check keys; it does not parse Fluent.
fn message_ids(ftl: &str) -> BTreeSet<&str> {
    ftl.lines()
        .filter(|line| !line.starts_with([' ', '\t', '#']))
        .filter_map(|line| line.split_once('='))
        .map(|(id, _)| id.trim())
        .filter(|id| !id.is_empty())
        .collect()
}
