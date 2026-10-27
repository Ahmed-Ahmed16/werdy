//! Pure deed scoring and counts for the current werdy day.

#![forbid(unsafe_code)]

/// Error for an invalid day, mark, or deed change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Calendar day is invalid or outside the supported day count.
    InvalidDate,
    /// Fraction is not a finite value from zero to one.
    FractionOutOfRange,
    /// Target amount must be at least one.
    InvalidTargetAmount,
    /// Target unit key is empty.
    EmptyTargetUnit,
    /// Rule change starts before the deed was created.
    RuleBeforeCreation,
    /// Rule change starts before the latest rule version.
    RuleBeforeLatest,
    /// Archive starts before the deed was created.
    ArchiveBeforeCreation,
    /// Deed was already archived.
    AlreadyArchived,
    /// Day note text is empty and no credit was chosen.
    EmptyDayNote,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::InvalidDate => "day is invalid or outside the supported range",
            Self::FractionOutOfRange => "fraction is not from zero to one",
            Self::InvalidTargetAmount => "target amount must be at least one",
            Self::EmptyTargetUnit => "target unit is empty",
            Self::RuleBeforeCreation => "rule change starts before the deed was created",
            Self::RuleBeforeLatest => "rule change starts before the latest rule",
            Self::ArchiveBeforeCreation => "archive starts before the deed was created",
            Self::AlreadyArchived => "deed was already archived",
            Self::EmptyDayNote => "day note is empty and has no credit",
        };
        formatter.write_str(text)
    }
}

impl std::error::Error for Error {}

mod consistency;
mod day;
mod deed;
mod mark;
mod note;
mod report;
mod score;
mod sheet;
#[cfg(feature = "serde")]
mod stored;

pub use consistency::{Consistency, DEFAULT_CONSISTENCY_WINDOW, consistency, kept_days};
pub use day::{Weekday, WerdyDay};
pub use deed::{CategoryKey, Deed, DeedId, DeedRule, Schedule, SlotKey, Target, delete_deed};
pub use mark::{Fraction, Mark, MarkEntry};
pub use note::{DayNote, ExcuseCredit};
pub use report::{Band, MissedDeed, Period, Report, ScoreBands, SeriesEntry, build_report};
pub use score::{OwnerRecords, Score, TodaySummary, score, today_summary};
pub use sheet::{DailySheet, SheetRow, SheetSlot, daily_sheet};
