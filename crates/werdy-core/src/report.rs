use std::collections::{BTreeMap, BTreeSet};

use crate::{
    CategoryKey, DeedId, Error, Fraction, OwnerRecords, Score, ScoreValue, WerdyDay, score,
};

const THREE_DAYS: i64 = 3;
const WEEK_DAYS: i64 = 7;
// ___: Confirm the rolling month length with the owner.
const MONTH_DAYS: i64 = 30;
// ___: Confirm the rolling six month length with the owner.
const SIX_MONTH_DAYS: i64 = 182;
const YEAR_DAYS: i64 = 365;

/// Rolling report period ending before today.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Period {
    /// Last three finished werdy days.
    ThreeDays,
    /// Last seven finished werdy days.
    Week,
    /// Last thirty finished werdy days.
    Month,
    /// Last 182 finished werdy days.
    SixMonths,
    /// Last 365 finished werdy days.
    Year,
}

impl Period {
    fn length(self) -> i64 {
        match self {
            Self::ThreeDays => THREE_DAYS,
            Self::Week => WEEK_DAYS,
            Self::Month => MONTH_DAYS,
            Self::SixMonths => SIX_MONTH_DAYS,
            Self::Year => YEAR_DAYS,
        }
    }

    /// Returns days in oldest to newest order. Step one moves one whole period back.
    /// At the day count limit, only representable days are returned.
    pub fn days(self, today: WerdyDay, steps_back: usize) -> Vec<WerdyDay> {
        let Ok(steps_back) = i64::try_from(steps_back) else {
            return Vec::new();
        };
        let Some(first_offset) = steps_back
            .checked_mul(self.length())
            .and_then(|offset| offset.checked_add(1))
        else {
            return Vec::new();
        };
        (0..self.length())
            .rev()
            .filter_map(|index| {
                first_offset
                    .checked_add(index)
                    .and_then(|offset| today.checked_sub_days(offset))
            })
            .collect()
    }
}

/// Score thresholds used by report cells.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScoreBands {
    /// Percent at or above which a score is good.
    pub good: u8,
    /// Percent at or above which a score is middle.
    pub mid: u8,
}

impl Default for ScoreBands {
    fn default() -> Self {
        Self { good: 80, mid: 50 }
    }
}

impl ScoreBands {
    /// Maps a percent to its configured band.
    pub fn band(self, percent: u8) -> Band {
        if percent >= self.good {
            Band::Good
        } else if percent >= self.mid {
            Band::Mid
        } else {
            Band::Low
        }
    }
}

/// Colour band for a score with due deeds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Band {
    /// At or above the good threshold.
    Good,
    /// At or above the middle threshold.
    Mid,
    /// Below the middle threshold.
    Low,
}

/// A deed's value on one day when it was due.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeedDay {
    /// Finished werdy day.
    pub day: WerdyDay,
    /// Value after any day credit.
    pub value: Fraction,
    /// A mark or day note added more than two days after this day.
    pub added_later: bool,
}

/// One sentence chosen from report facts. The caller supplies translated text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportLead {
    /// No deed was due.
    Empty,
    /// All due deeds have full value.
    AllDone,
    /// Deed with the greatest lost value.
    MostMissed(DeedId),
}

/// Selects the plain report lead without language or display logic.
pub fn report_lead(report: &Report) -> ReportLead {
    if report.overall.due == 0 {
        ReportLead::Empty
    } else if let Some(missed) = report.missed.first() {
        ReportLead::MostMissed(missed.deed_id)
    } else {
        ReportLead::AllDone
    }
}

/// One day or Gregorian month in report series.
#[derive(Clone, Debug, PartialEq)]
pub enum SeriesEntry {
    /// One finished werdy day.
    Day {
        /// Day label.
        day: WerdyDay,
        /// Score for this day.
        score: Score,
        /// None when no deed was due.
        band: Option<Band>,
    },
    /// All period days in one Gregorian month.
    Month {
        /// Gregorian year.
        year: i32,
        /// Gregorian month from one to twelve.
        month: u8,
        /// Score for included days in this month.
        score: Score,
        /// None when no deed was due.
        band: Option<Band>,
    },
}

/// Due deed with unfinished days in this period.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MissedDeed {
    /// Stable deed identity.
    pub deed_id: DeedId,
    /// Due days worth less than one after any credit.
    pub days_not_done: usize,
    /// Sum of value lost across due days.
    pub lost: ScoreValue,
}

/// Scores and history for one rolling period.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// Thresholds used for this report's score bands.
    pub bands: ScoreBands,
    /// Finished days included in this period.
    pub days: Vec<WerdyDay>,
    /// Score across all due deeds.
    pub overall: Score,
    /// Daily or monthly score entries.
    pub series: Vec<SeriesEntry>,
    /// Scores under each dated category key.
    pub by_category: BTreeMap<CategoryKey, Score>,
    /// Deeds with lost value, sorted by lost value then deed id.
    pub missed: Vec<MissedDeed>,
    /// Score for each deed due in this period.
    pub per_deed: BTreeMap<DeedId, Score>,
    /// Values only on days each deed was due.
    pub per_deed_days: BTreeMap<DeedId, Vec<DeedDay>>,
    /// Deeds never due in this period.
    pub not_due: Vec<DeedId>,
    /// Days with a note or excuse credit.
    pub noted_days: BTreeSet<WerdyDay>,
}

/// Builds scores for a rolling period ending yesterday.
/// Returns `ScoreOverflow` if any exact total or due count cannot fit.
pub fn build_report(
    records: &OwnerRecords<'_>,
    period: Period,
    steps_back: usize,
    today: WerdyDay,
    bands: ScoreBands,
) -> Result<Report, Error> {
    let days = period.days(today, steps_back);
    let overall = score(records, &days, today)?;
    let series = match period {
        Period::ThreeDays | Period::Week | Period::Month => {
            build_day_series(records, &days, today, bands)?
        }
        Period::SixMonths | Period::Year => build_month_series(records, &days, today, bands)?,
    };
    let by_category = category_scores(records, &days)?;
    let (missed, per_deed) = deed_summaries(records, &days, today)?;
    let (per_deed_days, not_due) = deed_day_history(records, &days);
    let noted_days = days
        .iter()
        .filter(|day| records.day_notes.contains_key(day))
        .copied()
        .collect();
    Ok(Report {
        bands,
        days,
        overall,
        series,
        by_category,
        missed,
        per_deed,
        per_deed_days,
        not_due,
        noted_days,
    })
}

fn deed_day_history(
    records: &OwnerRecords<'_>,
    days: &[WerdyDay],
) -> (BTreeMap<DeedId, Vec<DeedDay>>, Vec<DeedId>) {
    let mut history = BTreeMap::new();
    let mut not_due = Vec::new();
    for deed in records.deeds {
        let due_days: Vec<_> = days
            .iter()
            .filter_map(|day| {
                deed.due_rule_on(*day, &records.calendar)
                    .map(|rule| DeedDay {
                        day: *day,
                        value: crate::score::due_value(records, deed, *day, rule),
                        added_later: records
                            .marks
                            .get(&(deed.id, *day))
                            .is_some_and(|entry| entry.is_late_for(*day))
                            || records
                                .day_notes
                                .get(day)
                                .is_some_and(|note| note.is_late_for(*day)),
                    })
            })
            .collect();
        if due_days.is_empty() {
            not_due.push(deed.id);
        } else {
            history.insert(deed.id, due_days);
        }
    }
    (history, not_due)
}

fn build_day_series(
    records: &OwnerRecords<'_>,
    days: &[WerdyDay],
    today: WerdyDay,
    bands: ScoreBands,
) -> Result<Vec<SeriesEntry>, Error> {
    days.iter()
        .map(|day| {
            let day_score = score(records, &[*day], today)?;
            Ok(SeriesEntry::Day {
                day: *day,
                score: day_score,
                band: day_score.percent.map(|percent| bands.band(percent)),
            })
        })
        .collect()
}

fn build_month_series(
    records: &OwnerRecords<'_>,
    days: &[WerdyDay],
    today: WerdyDay,
    bands: ScoreBands,
) -> Result<Vec<SeriesEntry>, Error> {
    let mut buckets: BTreeMap<(i32, u8), Vec<WerdyDay>> = BTreeMap::new();
    for day in days {
        buckets.entry(day.year_month()).or_default().push(*day);
    }
    buckets
        .into_iter()
        .map(|((year, month), bucket_days)| {
            let month_score = score(records, &bucket_days, today)?;
            Ok(SeriesEntry::Month {
                year,
                month,
                score: month_score,
                band: month_score.percent.map(|percent| bands.band(percent)),
            })
        })
        .collect()
}

fn category_scores(
    records: &OwnerRecords<'_>,
    days: &[WerdyDay],
) -> Result<BTreeMap<CategoryKey, Score>, Error> {
    let mut totals: BTreeMap<CategoryKey, (ScoreValue, usize)> = BTreeMap::new();
    for day in days {
        for deed in records.deeds {
            if let Some(rule) = deed.due_rule_on(*day, &records.calendar) {
                let value = crate::score::due_value(records, deed, *day, rule);
                let category = totals.entry(rule.category.clone()).or_default();
                category.0.add_fraction(value)?;
                category.1 = category.1.checked_add(1).ok_or(Error::ScoreOverflow)?;
            }
        }
    }
    Ok(totals
        .into_iter()
        .map(|(key, (earned, due))| (key, crate::score::score_from_totals(earned, due)))
        .collect())
}

fn deed_summaries(
    records: &OwnerRecords<'_>,
    days: &[WerdyDay],
    today: WerdyDay,
) -> Result<(Vec<MissedDeed>, BTreeMap<DeedId, Score>), Error> {
    let mut missed = Vec::new();
    let mut per_deed = BTreeMap::new();
    for deed in records.deeds {
        let deed_score = score(
            &OwnerRecords {
                deeds: std::slice::from_ref(deed),
                marks: records.marks,
                day_notes: records.day_notes,
                calendar: records.calendar.clone(),
            },
            days,
            today,
        )?;
        if deed_score.due == 0 {
            continue;
        }
        per_deed.insert(deed.id, deed_score);
        if let Some(missed_deed) = missed_deed(records, deed, days)? {
            missed.push(missed_deed);
        }
    }
    missed.sort_by(|left, right| {
        right
            .lost
            .cmp(&left.lost)
            .then_with(|| left.deed_id.cmp(&right.deed_id))
    });
    Ok((missed, per_deed))
}

fn missed_deed(
    records: &OwnerRecords<'_>,
    deed: &crate::Deed,
    days: &[WerdyDay],
) -> Result<Option<MissedDeed>, Error> {
    let mut days_not_done = 0usize;
    let mut lost = ScoreValue::ZERO;
    for day in days {
        if let Some(rule) = deed.due_rule_on(*day, &records.calendar) {
            let value = crate::score::due_value(records, deed, *day, rule);
            if value < Fraction::ONE {
                days_not_done = days_not_done.checked_add(1).ok_or(Error::ScoreOverflow)?;
                lost.add_fraction(value.complement())?;
            }
        }
    }
    Ok((days_not_done > 0).then_some(MissedDeed {
        deed_id: deed.id,
        days_not_done,
        lost,
    }))
}
