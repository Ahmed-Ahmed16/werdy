//! Repeatable sample history for the separate demo store.

use werdy_core::{DayNote, Deed, ExcuseCredit, Mark, MarkEntry, WerdyDay};
use werdy_data::Catalog;
use werdy_store::{DeedName, OwnerData, Settings};

use crate::i18n::{Language, Texts};

// Change these values together when the sample history needs a new shape.
const HISTORY_DAYS: usize = 120;
const SHORT_HISTORY_DAYS: usize = 10;
const ARCHIVED_DAYS_AGO: usize = 20;
const GAP_START: usize = 20;
const GAP_END: usize = 24;
const DEED_KEYS: [&str; 12] = [
    "fajr_on_time",
    "dhuhr_on_time",
    "asr_on_time",
    "maghrib_on_time",
    "isha_on_time",
    "morning_adhkar",
    "evening_adhkar",
    "tahlil",
    "quran_reading",
    "fasting_monday_thursday",
    "sleep_adhkar",
    "istighfar",
];
const SHORT_KEY: &str = "sleep_adhkar";
const ARCHIVED_KEY: &str = "istighfar";
const OFTEN_MISSED_KEY: &str = "evening_adhkar";
const MISSED_INTERVAL: usize = 17;
const PARTIAL_INTERVAL: usize = 9;
const UNMARKED_WINDOW: usize = 30;
const UNMARKED_INTERVAL: usize = 11;
const UNMARKED_DEED_INTERVAL: u64 = 3;
const OFTEN_MISSED_DAYS: std::ops::RangeInclusive<usize> = 7..=13;
const LATE_OFFSETS: [usize; 3] = [8, 13, 35];
const LATE_DELAY_DAYS: usize = 4;
const NOTES: [(usize, ExcuseCredit, &str); 2] = [
    (4, ExcuseCredit::Half, "demo-note-half"),
    (16, ExcuseCredit::Full, "demo-note-full"),
];

fn days_before(mut day: WerdyDay, count: usize) -> Result<WerdyDay, werdy_core::Error> {
    for _ in 0..count {
        day = day.previous()?;
    }
    Ok(day)
}

fn days_after(mut day: WerdyDay, count: usize) -> Result<WerdyDay, werdy_core::Error> {
    for _ in 0..count {
        day = day.next()?;
    }
    Ok(day)
}

/// Builds sample deeds and finished days without reading the browser or clock.
pub fn demo_records(today: WerdyDay) -> Result<OwnerData, Box<dyn std::error::Error>> {
    let catalog = crate::editor_data::catalog()?;
    let mut data = OwnerData {
        first_run_complete: true,
        settings: demo_settings()?,
        ..OwnerData::default()
    };
    data.chosen_sets.insert("five_prayers".to_owned());
    add_demo_deeds(today, catalog, &mut data)?;
    add_demo_marks(today, catalog, &mut data)?;
    add_demo_notes(today, &mut data)?;
    Ok(data)
}

fn add_demo_deeds(
    today: WerdyDay,
    catalog: &Catalog,
    data: &mut OwnerData,
) -> Result<(), Box<dyn std::error::Error>> {
    for key in DEED_KEYS {
        let catalog_deed = catalog
            .deed(key)
            .ok_or("demo deed is missing from catalog")?;
        let rule = crate::starter::rule(catalog_deed).ok_or("demo deed has invalid rule")?;
        let created = days_before(
            today,
            if key == SHORT_KEY {
                SHORT_HISTORY_DAYS
            } else {
                HISTORY_DAYS
            },
        )?;
        let id = data.next_deed_id();
        data.last_deed_id = id.0;
        let mut deed = Deed::new(id, created, rule);
        if key == ARCHIVED_KEY {
            deed.archive(days_before(today, ARCHIVED_DAYS_AGO)?)?;
        }
        data.names
            .insert(id, DeedName::Key(catalog_deed.name.clone()));
        data.starter_keys.insert(id, key.to_owned());
        data.deeds.push(deed);
    }
    Ok(())
}

fn add_demo_marks(
    today: WerdyDay,
    catalog: &Catalog,
    data: &mut OwnerData,
) -> Result<(), Box<dyn std::error::Error>> {
    for offset in (1..=HISTORY_DAYS).rev() {
        if (GAP_START..=GAP_END).contains(&offset) {
            continue;
        }
        let day = days_before(today, offset)?;
        for deed in &data.deeds {
            if !deed.is_due_on(day, &data.settings.calendar) {
                continue;
            }
            if offset < UNMARKED_WINDOW
                && offset.is_multiple_of(UNMARKED_INTERVAL)
                && deed.id.0.is_multiple_of(UNMARKED_DEED_INTERVAL)
            {
                continue;
            }
            let key = data
                .starter_keys
                .get(&deed.id)
                .ok_or("demo deed has no key")?;
            let target = catalog.deed(key).and_then(|deed| deed.target.as_ref());
            let mark = demo_mark(offset, deed.id.0 as usize, key, target);
            let entered_on = if LATE_OFFSETS.contains(&offset) && key == OFTEN_MISSED_KEY {
                days_after(day, LATE_DELAY_DAYS)?
            } else {
                day
            };
            data.marks
                .insert((deed.id, day), MarkEntry::new(mark, entered_on));
        }
    }
    Ok(())
}

fn demo_mark(
    offset: usize,
    deed_number: usize,
    key: &str,
    target: Option<&werdy_data::Target>,
) -> Mark {
    if key == OFTEN_MISSED_KEY && OFTEN_MISSED_DAYS.contains(&offset)
        || (offset + deed_number).is_multiple_of(MISSED_INTERVAL)
    {
        Mark::Missed
    } else if (offset + deed_number).is_multiple_of(PARTIAL_INTERVAL) {
        target.map_or_else(Mark::partial_without_target, |target| {
            Mark::Count(target.amount / 2)
        })
    } else {
        target.map_or(Mark::Done, |target| Mark::Count(target.amount))
    }
}

fn add_demo_notes(today: WerdyDay, data: &mut OwnerData) -> Result<(), Box<dyn std::error::Error>> {
    let texts = Texts::load();
    for (offset, credit, key) in NOTES {
        let day = days_before(today, offset)?;
        let note = texts.get(Language::Arabic, key, &[]);
        data.day_notes
            .insert(day, DayNote::new(&note, credit, day)?);
    }
    Ok(())
}

pub fn demo_settings() -> Result<Settings, Box<dyn std::error::Error>> {
    let city = crate::location::cities()?
        .into_iter()
        .find(|city| city.key == "cairo")
        .ok_or("Cairo is missing from city data")?;
    Ok(Settings {
        location: Some(city.location().ok_or("Cairo has invalid coordinates")?),
        prayer_method: crate::location::method_for_country(&city.country)?,
        ..Settings::default()
    })
}

#[cfg(test)]
mod tests {
    use super::{ARCHIVED_KEY, GAP_END, GAP_START, SHORT_KEY, days_before, demo_records};
    use werdy_core::{Period, ScoreBands, WerdyDay, build_report, kept_days, milestone_to_show};
    use werdy_store::{MemoryStore, PrayerMethod, Store, Theme, from_json, to_json};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn today() -> Result<WerdyDay, werdy_core::Error> {
        WerdyDay::new(2026, 10, 28)
    }

    #[test]
    fn demo_owner_records_survive_store_round_trip() -> TestResult {
        let sample = demo_records(today()?)?;
        assert!(sample.first_run_complete);
        assert_eq!(sample.settings.language, "ar");
        assert_eq!(sample.settings.theme, Theme::System);
        assert_eq!(
            sample.settings.prayer_method,
            PrayerMethod::EgyptianGeneralAuthority
        );
        assert_eq!(
            sample
                .settings
                .location
                .as_ref()
                .and_then(|place| place.city_key.as_deref()),
            Some("cairo")
        );
        assert_eq!(from_json(&to_json(&sample)?)?, sample);
        let store = MemoryStore::default();
        store.save(&sample)?;
        assert_eq!(store.load()?, Some(sample));
        Ok(())
    }

    #[test]
    fn demo_week_month_and_six_month_reports_have_due_deeds_and_loss() -> TestResult {
        let today = today()?;
        let sample = demo_records(today)?;
        for period in [Period::Week, Period::Month, Period::SixMonths] {
            let report = build_report(&sample.records(), period, 0, today, ScoreBands::default())?;
            assert!(report.overall.due > 0);
            assert!(matches!(report.overall.percent, Some(1..=99)));
            assert!(!report.missed.is_empty());
        }
        assert_eq!(
            sample
                .marks
                .iter()
                .filter(|((_, day), entry)| entry.is_late_for(*day))
                .count(),
            3
        );
        for credit in [
            werdy_core::ExcuseCredit::Half,
            werdy_core::ExcuseCredit::Full,
        ] {
            assert!(
                sample
                    .day_notes
                    .values()
                    .any(|note| note.credit() == credit && !note.text().is_empty())
            );
        }
        let kept = kept_days(&sample.records(), today);
        assert!(kept >= 30);
        assert_eq!(
            milestone_to_show(kept, &[7, 30], &sample.dismissed_milestones),
            Some(30)
        );
        assert!(sample.dismissed_milestones.is_empty());
        assert!(sample.last_welcome_day.is_none());
        Ok(())
    }

    #[test]
    fn demo_archived_deed_and_added_deed_keep_their_own_history() -> TestResult {
        let today = today()?;
        let sample = demo_records(today)?;
        let archived = sample
            .deeds
            .iter()
            .find(|deed| {
                sample
                    .starter_keys
                    .get(&deed.id)
                    .is_some_and(|key| key == ARCHIVED_KEY)
            })
            .ok_or("archived deed missing")?;
        let archive_day = archived.archived_from().ok_or("archive day missing")?;
        assert!(
            sample
                .marks
                .keys()
                .any(|(id, day)| *id == archived.id && *day < archive_day)
        );
        assert!(
            !sample
                .marks
                .keys()
                .any(|(id, day)| *id == archived.id && *day >= archive_day)
        );
        let short = sample
            .deeds
            .iter()
            .find(|deed| {
                sample
                    .starter_keys
                    .get(&deed.id)
                    .is_some_and(|key| key == SHORT_KEY)
            })
            .ok_or("short deed missing")?;
        assert_eq!(short.created_on(), days_before(today, 10)?);
        assert!(sample.marks.keys().all(|(_, day)| *day < today));
        for offset in GAP_START..=GAP_END {
            let day = days_before(today, offset)?;
            assert!(
                sample
                    .marks
                    .keys()
                    .all(|(_, marked_day)| *marked_day != day)
            );
        }
        Ok(())
    }
}
