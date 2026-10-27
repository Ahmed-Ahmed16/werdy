//! Pure deed scoring and counts for the current werdy day.

#![forbid(unsafe_code)]

/// Error for invalid input, deed changes, or score overflow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Calendar day is invalid or outside the supported day count.
    InvalidDate,
    /// Fraction is outside zero to one, has zero denominator, or has nonfinite float input.
    FractionOutOfRange,
    /// Exact score cannot fit within u128 values.
    ScoreOverflow,
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
    /// Hijri offset is outside minus one to plus one day.
    InvalidHijriOffset,
    /// Hijri range has a month outside 1 to 12 or days outside 1 to 30, or ends before it starts.
    InvalidHijriRange,
}

mod consistency;
mod day;
mod deed;
mod hijri;
mod mark;
mod note;
mod report;
mod score;

pub use consistency::{Consistency, DEFAULT_CONSISTENCY_WINDOW, consistency, kept_days};
pub use day::{Weekday, WerdyDay};
pub use deed::{CategoryKey, Deed, DeedId, DeedRule, Schedule, Target, delete_deed};
pub use hijri::{CalendarSettings, HijriDate, HijriOffset, HijriRange};
pub use mark::{Fraction, Mark, MarkEntry};
pub use note::{DayNote, ExcuseCredit};
pub use report::{Band, MissedDeed, Period, Report, ScoreBands, SeriesEntry, build_report};
pub use score::{OwnerRecords, Score, ScoreValue, TodaySummary, score, today_summary};
