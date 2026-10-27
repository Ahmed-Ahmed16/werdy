//! Pure deed scoring and counts for the current werdy day.

#![forbid(unsafe_code)]

/// Error for an invalid day, mark, or deed change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Calendar day is invalid or outside the supported day count.
    InvalidDate,
    /// Fraction is not a finite value from zero to one.
    FractionOutOfRange,
    /// Rule change starts before the deed was created.
    RuleBeforeCreation,
    /// Rule change starts before the latest rule version.
    RuleBeforeLatest,
    /// Archive starts before the deed was created.
    ArchiveBeforeCreation,
    /// Deed was already archived.
    AlreadyArchived,
}

mod day;
mod deed;
mod mark;
mod score;

pub use day::{Weekday, WerdyDay};
pub use deed::{Deed, DeedId, DeedRule, Schedule, delete_deed};
pub use mark::{Fraction, Mark, MarkEntry};
pub use score::{Score, TodaySummary, score, today_summary};
