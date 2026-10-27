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
    /// Stored exact score has a zero denominator.
    InvalidScoreValue,
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
    /// Deed is not archived.
    NotArchived,
    /// Restore day comes before the archive day.
    RestoreBeforeArchive,
    /// Day note text is empty and no credit was chosen.
    EmptyDayNote,
    /// Hijri offset is outside minus one to plus one day.
    InvalidHijriOffset,
    /// Hijri offset change starts before the latest change.
    OffsetBeforeLatest,
    /// Hijri range has a month outside 1 to 12 or days outside 1 to 30, or ends before it starts.
    InvalidHijriRange,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::InvalidDate => "day is invalid or outside the supported range",
            Self::FractionOutOfRange => "fraction is not from zero to one",
            Self::ScoreOverflow => "exact score is too large",
            Self::InvalidScoreValue => "exact score denominator must be positive",
            Self::InvalidTargetAmount => "target amount must be at least one",
            Self::EmptyTargetUnit => "target unit is empty",
            Self::RuleBeforeCreation => "rule change starts before the deed was created",
            Self::RuleBeforeLatest => "rule change starts before the latest rule",
            Self::ArchiveBeforeCreation => "archive starts before the deed was created",
            Self::AlreadyArchived => "deed was already archived",
            Self::NotArchived => "deed is not archived",
            Self::RestoreBeforeArchive => "restore starts before archive",
            Self::EmptyDayNote => "day note is empty and has no credit",
            Self::InvalidHijriOffset => "Hijri offset must be from minus one to plus one day",
            Self::OffsetBeforeLatest => "Hijri offset change starts before the latest change",
            Self::InvalidHijriRange => "Hijri range is invalid",
        };
        formatter.write_str(text)
    }
}

impl std::error::Error for Error {}

mod consistency;
mod day;
mod deed;
mod hijri;
mod mark;
mod note;
mod report;
mod score;
mod sheet;
#[cfg(feature = "serde")]
mod stored;

pub use consistency::{Consistency, DEFAULT_CONSISTENCY_WINDOW, consistency, kept_days};
pub use day::{Weekday, WerdyDay};
pub use deed::{
    CategoryKey, Deed, DeedId, DeedRule, Schedule, SlotKey, Target, active_deed_count,
    archived_deed_count, decrease_target_amount, delete_deed, increase_target_amount,
};
pub use hijri::{CalendarSettings, HijriDate, HijriOffset, HijriRange};
pub use mark::{Fraction, Mark, MarkEntry, convert_mark_for_target};
pub use note::{DayNote, ExcuseCredit};
pub use report::{Band, MissedDeed, Period, Report, ScoreBands, SeriesEntry, build_report};
pub use score::{
    OwnerRecords, Score, ScoreValue, TodaySummary, finished_day_summary, score, today_summary,
};
pub use sheet::{DailySheet, SheetRow, SheetSlot, daily_sheet};
