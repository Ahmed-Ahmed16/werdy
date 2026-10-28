#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use werdy_core::{
    CategoryKey, DayNote, Deed, DeedId, DeedRule, ExcuseCredit, Mark, MarkEntry, Period, Schedule,
    ScoreBands, SlotKey, Target, WerdyDay, build_report,
};
use werdy_store::{DeedName, FileStore, OwnerData, Store, StoreError, Theme, to_json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct TestFolder(PathBuf);

impl TestFolder {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        static NEXT_FOLDER: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "werdy-file-store-{}-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
            NEXT_FOLDER.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for TestFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn sample() -> Result<OwnerData, Box<dyn std::error::Error>> {
    let start = WerdyDay::new(2026, 10, 1)?;
    let later = WerdyDay::new(2026, 10, 15)?;
    let initial_rule = DeedRule {
        schedule: Schedule::EveryDay,
        target: None,
        category: CategoryKey::new("worship"),
        slot: SlotKey::new("morning"),
    };
    let mut reading = Deed::new(DeedId(1), start, initial_rule.clone());
    reading.change_rule(
        later,
        DeedRule {
            target: Some(Target::new(5, "page")?),
            ..initial_rule
        },
    )?;
    let mut owner = OwnerData {
        first_run_complete: true,
        deeds: vec![reading],
        last_deed_id: 1,
        names: [(DeedId(1), DeedName::Text("قراءة القرآن".to_owned()))].into(),
        ..OwnerData::default()
    };
    owner.settings.theme = Theme::Dark;
    let mut day = start;
    for step in 0..20 {
        let mark = match step % 4 {
            0 => Mark::Done,
            1 => Mark::partial_without_target(),
            2 => Mark::Missed,
            _ => Mark::Count(3),
        };
        owner.set_mark(DeedId(1), day, Some(MarkEntry::new(mark, day)), step + 1);
        day = day.next()?;
    }
    owner.set_day_note(
        later,
        Some(DayNote::new("ill", ExcuseCredit::Half, later)?),
        30,
    );
    Ok(owner)
}

#[test]
fn missing_file_loads_none() -> TestResult {
    let folder = TestFolder::new()?;
    assert_eq!(FileStore::new(folder.0.join("werdy.json")).load()?, None);
    Ok(())
}

#[test]
fn saved_records_and_reports_survive_reopening() -> TestResult {
    let folder = TestFolder::new()?;
    let path = folder.0.join("private").join("werdy.json");
    let owner = sample()?;
    FileStore::new(path.clone()).save(&owner)?;

    assert_eq!(fs::read_to_string(&path)?, to_json(&owner)?);
    let loaded = FileStore::new(path)
        .load()?
        .ok_or("missing saved records")?;
    assert_eq!(loaded, owner);
    let today = WerdyDay::new(2026, 10, 21)?;
    for period in [Period::ThreeDays, Period::Week, Period::Month] {
        assert_eq!(
            build_report(&loaded.records(), period, 0, today, ScoreBands::default()),
            build_report(&owner.records(), period, 0, today, ScoreBands::default()),
        );
    }
    Ok(())
}

#[test]
fn corrupt_file_returns_error() -> TestResult {
    let folder = TestFolder::new()?;
    let path = folder.0.join("werdy.json");
    fs::write(&path, "{broken")?;
    assert!(matches!(
        FileStore::new(path).load(),
        Err(StoreError::Corrupt(_))
    ));
    Ok(())
}

#[test]
fn saving_twice_replaces_records_without_temporary_files() -> TestResult {
    let folder = TestFolder::new()?;
    let store = FileStore::new(folder.0.join("werdy.json"));
    store.save(&sample()?)?;
    let latest = OwnerData::default();
    store.save(&latest)?;
    assert_eq!(store.load()?, Some(latest));
    let files = fs::read_dir(&folder.0)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(files, vec![std::ffi::OsString::from("werdy.json")]);
    Ok(())
}

#[test]
fn unreadable_path_returns_backend_error_text() -> TestResult {
    let folder = TestFolder::new()?;
    let expected = fs::read_to_string(&folder.0)
        .err()
        .ok_or("expected read error")?;
    assert_eq!(
        FileStore::new(folder.0.clone()).load(),
        Err(StoreError::Backend(expected.to_string()))
    );
    Ok(())
}

#[test]
fn blocked_parent_returns_backend_error() -> TestResult {
    let folder = TestFolder::new()?;
    let parent = folder.0.join("blocked");
    fs::write(&parent, "file instead of folder")?;
    assert!(matches!(
        FileStore::new(parent.join("werdy.json")).save(&sample()?),
        Err(StoreError::Backend(_))
    ));
    Ok(())
}

#[test]
fn failed_temporary_write_keeps_previous_records() -> TestResult {
    let folder = TestFolder::new()?;
    let store = FileStore::new(folder.0.join("werdy.json"));
    let original = sample()?;
    store.save(&original)?;
    fs::create_dir(folder.0.join("werdy.json.tmp"))?;
    assert!(matches!(
        store.save(&OwnerData::default()),
        Err(StoreError::Backend(_))
    ));
    assert_eq!(store.load()?, Some(original));
    Ok(())
}
