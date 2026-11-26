#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use werdy_core::{
    CategoryKey, DayNote, Deed, DeedId, DeedRule, ExcuseCredit, Mark, MarkEntry, Schedule, SlotKey,
    WerdyDay,
};
use werdy_store::{DeedName, OwnerData};
use werdy_sync::{Backend, Batch, BatchRow, Change, FakeBackend, Pending, Sync, SyncError};

// These tests use in memory backends whose futures finish on the first poll.
fn run<F: Future>(future: F) -> F::Output {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test backend unexpectedly suspended"),
    }
}

fn day() -> WerdyDay {
    WerdyDay::new(2026, 10, 28).unwrap()
}

fn rule(slot: &str) -> DeedRule {
    DeedRule {
        schedule: Schedule::EveryDay,
        target: None,
        category: CategoryKey::new("prayer"),
        slot: SlotKey::new(slot),
    }
}

fn history(name: &str) -> OwnerData {
    let mut records = OwnerData {
        last_deed_id: 1,
        ..OwnerData::default()
    };
    records
        .deeds
        .push(Deed::new(DeedId(1), day(), rule("fajr")));
    records
        .names
        .insert(DeedId(1), DeedName::Text(name.to_owned()));
    records.set_mark(DeedId(1), day(), Some(MarkEntry::new(Mark::Done, day())), 1);
    records
}

fn sync(engine: &mut Sync, backend: &mut impl Backend, records: &mut OwnerData) {
    run(engine.sync(backend, records, || 1_000)).unwrap();
}

fn wire_row(sequence: u64, change: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "pending": {"install": "source", "operation": sequence, "change": change},
        "stamp": {"received_at": 1000, "sequence": sequence}
    })
}

fn wire_page(cursor: u64, changes: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({"cursor": cursor, "changes": changes})
}

struct WirePull(serde_json::Value);

impl Backend for WirePull {
    async fn pull(&mut self, _: u64) -> Result<Batch, SyncError> {
        serde_json::from_value(self.0.clone())
            .map_err(|error| SyncError::Invalid(error.to_string()))
    }

    async fn push(&mut self, changes: &[Pending]) -> Result<Batch, SyncError> {
        assert!(changes.is_empty(), "unchanged records must not push rows");
        Ok(serde_json::from_value(wire_page(self.0["cursor"].as_u64().unwrap(), vec![])).unwrap())
    }

    async fn server_time(&mut self) -> Result<u64, SyncError> {
        Ok(1_000)
    }
}

fn wire_history() -> Vec<serde_json::Value> {
    let mut backend = FakeBackend::new(1_000);
    let mut records = history("shared");
    records.chosen_sets.insert("starter".to_owned());
    sync(
        &mut Sync::new("source").unwrap(),
        &mut backend,
        &mut records,
    );
    let batch = serde_json::to_value(run(backend.pull(0)).unwrap()).unwrap();
    batch["changes"].as_array().unwrap().clone()
}

fn wire_rule(rows: &[serde_json::Value]) -> serde_json::Value {
    rows.iter()
        .find_map(|row| row["pending"]["change"].get("Rule"))
        .unwrap()
        .clone()
}

#[test]
fn held_deed_ids_cover_headers_rules_and_marks() {
    let mut source = FakeBackend::new(1_000);
    for install in ["rule_copy", "header_copy", "mark_copy", "readable_copy"] {
        sync(
            &mut Sync::new(install).unwrap(),
            &mut source,
            &mut history(install),
        );
    }
    let batch = run(source.pull(0)).unwrap();
    let cursor = batch.cursor;
    let mut rows = serde_json::to_value(batch).unwrap()["changes"]
        .as_array()
        .unwrap()
        .clone();
    for (offset, (install, tag, pointer, unreadable)) in [
        (
            "rule_copy",
            "Rule",
            "/Rule/rule/schedule",
            serde_json::json!("some_future_kind"),
        ),
        (
            "header_copy",
            "Deed",
            "/Deed/row/name",
            serde_json::json!({"FutureName": "name"}),
        ),
        (
            "mark_copy",
            "Mark",
            "/Mark/entry",
            serde_json::json!({"FutureEntry": {}}),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut change = rows
            .iter()
            .map(|row| &row["pending"]["change"])
            .find(|change| change[tag]["id"]["install"] == install)
            .unwrap()
            .clone();
        *change.pointer_mut(pointer).unwrap() = unreadable;
        rows.push(wire_row(cursor + offset as u64 + 1, change));
    }

    let mut engine = Sync::new("reader").unwrap();
    let mut records = OwnerData::default();
    sync(
        &mut engine,
        &mut WirePull(wire_page(cursor + 3, rows)),
        &mut records,
    );
    assert_eq!(records.deeds.len(), 4);
    let expected = records
        .deed_keys
        .iter()
        .filter_map(|(id, key)| (key.install != "readable_copy").then_some(*id))
        .collect();
    assert_eq!(engine.held_deed_ids(), expected);
}

#[test]
fn unreadable_creation_rule_defers_deed_without_losing_readable_rows() {
    let mut rows = wire_history();
    let mut later = wire_rule(&rows);
    for row in &mut rows {
        if let Some(rule) = row["pending"]["change"].get_mut("Rule") {
            rule["rule"]["schedule"] = "some_future_kind".into();
        }
    }
    let mut engine = Sync::new("reader").unwrap();
    let mut records = OwnerData::default();
    let first_cursor = rows.len() as u64;
    sync(
        &mut engine,
        &mut WirePull(wire_page(first_cursor, rows)),
        &mut records,
    );
    assert!(records.deeds.is_empty());
    assert!(records.marks.is_empty());
    assert!(records.chosen_sets.contains("starter"));

    later["from_day"] = serde_json::to_value(day().next().unwrap()).unwrap();
    let cursor = first_cursor + 2;
    sync(
        &mut engine,
        &mut WirePull(wire_page(
            cursor,
            vec![
                wire_row(cursor - 1, serde_json::json!({"Rule": later})),
                wire_row(cursor, serde_json::json!({"Setting": {"Language": "en"}})),
            ],
        )),
        &mut records,
    );
    let state = serde_json::to_value(&engine).unwrap();
    assert_eq!(state["cursor"], cursor);
    assert!(records.deeds.is_empty());
    assert!(records.marks.is_empty());
    assert_eq!(records.settings.language, "en");
    engine.capture(&records).unwrap();
    assert!(engine.outbox().is_empty());
}

#[test]
fn unreadable_updates_keep_readable_rows_and_snapshot_has_no_tombstones() {
    let rows = wire_history();
    let cursor = rows.len() as u64;
    let mut engine = Sync::new("reader").unwrap();
    let mut records = OwnerData::default();
    sync(
        &mut engine,
        &mut WirePull(wire_page(cursor, rows.clone())),
        &mut records,
    );
    let before = records.clone();
    let mut future_rule = wire_rule(&rows);
    future_rule["rule"]["schedule"] = "some_future_kind".into();
    let mut future_mark = rows
        .iter()
        .find_map(|row| row["pending"]["change"].get("Mark"))
        .unwrap()
        .clone();
    future_mark["entry"] = serde_json::json!({"FutureEntry": {}});
    sync(
        &mut engine,
        &mut WirePull(wire_page(
            cursor + 5,
            vec![
                wire_row(cursor + 1, serde_json::json!({"Rule": future_rule})),
                wire_row(cursor + 2, serde_json::json!({"FutureKind": {}})),
                wire_row(
                    cursor + 3,
                    serde_json::json!({"Setting": {"FutureSetting": true}}),
                ),
                wire_row(cursor + 4, serde_json::json!({"Mark": future_mark})),
                wire_row(
                    cursor + 5,
                    serde_json::json!({"ChosenSet": {"key": "starter", "chosen": "some_future_kind"}}),
                ),
            ],
        )),
        &mut records,
    );
    assert_eq!(records, before);
    engine.capture(&records).unwrap();
    assert!(engine.outbox().is_empty());
    let state = serde_json::to_value(&engine).unwrap();
    assert_eq!(state["held"].as_array().unwrap().len(), 5);
    for change in state["observed"].as_array().unwrap() {
        assert!(!change.get("Deed").is_some_and(|deed| deed["row"].is_null()));
        assert!(
            !change
                .get("Mark")
                .is_some_and(|mark| mark["entry"].is_null())
        );
        assert!(
            !change
                .get("ChosenSet")
                .is_some_and(|set| set["chosen"] == false)
        );
    }
}

#[test]
fn held_rows_round_trip_and_retry_with_stamp_ordering() {
    let legacy = serde_json::to_value(Sync::new("reader").unwrap()).unwrap();
    let mut legacy = legacy.as_object().unwrap().clone();
    legacy.remove("held");
    let mut engine: Sync = serde_json::from_value(legacy.into()).unwrap();
    let mut records = OwnerData::default();
    let future = wire_row(2, serde_json::json!({"FutureKind": {}}));
    sync(
        &mut engine,
        &mut WirePull(wire_page(
            3,
            vec![
                wire_row(1, serde_json::json!({"Setting": {"FutureSetting": true}})),
                future.clone(),
                wire_row(3, serde_json::json!({"Setting": {"Language": "en"}})),
            ],
        )),
        &mut records,
    );
    assert_eq!(records.settings.language, "en");
    let checkpoint = serde_json::to_string(&engine).unwrap();
    let mut saved: serde_json::Value = serde_json::from_str(&checkpoint).unwrap();
    assert_eq!(saved["held"][1], future);
    let mut loaded: Sync = serde_json::from_str(&checkpoint).unwrap();
    assert_eq!(serde_json::to_value(&loaded).unwrap(), saved);
    saved["held"][0]["pending"]["change"] = serde_json::json!({"Setting": {"Language": "fr"}});
    saved["held"][1]["pending"]["change"] = serde_json::json!({"Setting": {"Theme": "dark"}});
    loaded = serde_json::from_value(saved).unwrap();
    sync(
        &mut loaded,
        &mut WirePull(wire_page(3, vec![])),
        &mut records,
    );
    assert_eq!(records.settings.language, "en");
    assert_eq!(records.settings.theme, werdy_store::Theme::Dark);
    assert_eq!(
        serde_json::to_value(&loaded).unwrap()["held"],
        serde_json::json!([])
    );
    loaded.capture(&records).unwrap();
    assert!(loaded.outbox().is_empty());
}

#[test]
fn background_sync_keeps_held_header_and_its_readable_dependents() {
    let mut rows = wire_history();
    for row in &mut rows {
        if let Some(deed) = row["pending"]["change"].get_mut("Deed") {
            deed["row"]["name"] = serde_json::json!({"FutureName": "shared"});
        }
    }
    let cursor = rows.len() as u64;
    let mut foreground = Sync::new("reader").unwrap();
    let mut background = foreground.clone();
    let mut records = OwnerData::default();
    sync(
        &mut background,
        &mut WirePull(wire_page(cursor, rows)),
        &mut records.clone(),
    );
    foreground.finish_from(&background, &mut records).unwrap();
    let mut saved = serde_json::to_value(&foreground).unwrap();
    assert_eq!(saved["cursor"], cursor);
    assert!(records.deeds.is_empty());
    assert!(records.marks.is_empty());
    saved["held"][0]["pending"]["change"]["Deed"]["row"]["name"] =
        serde_json::json!({"text": "shared"});
    foreground = serde_json::from_value(saved).unwrap();
    sync(
        &mut foreground,
        &mut WirePull(wire_page(cursor, vec![])),
        &mut records,
    );
    assert_eq!(records.deeds.len(), 1);
    assert_eq!(records.marks.len(), 1);
    foreground.capture(&records).unwrap();
    assert!(foreground.outbox().is_empty());
}

#[test]
fn watch_applies_readable_rows_around_an_unreadable_row() {
    let mut view = werdy_sync::WatchView::new().unwrap();
    let page = wire_page(
        3,
        vec![
            wire_row(1, serde_json::json!({"Setting": {"Language": "en"}})),
            wire_row(2, serde_json::json!({"FutureKind": {}})),
            wire_row(
                3,
                serde_json::json!({"ChosenSet": {"key": "starter", "chosen": true}}),
            ),
        ],
    );
    view.apply(serde_json::from_value(page).unwrap()).unwrap();
    assert_eq!(view.cursor(), 3);
    assert_eq!(view.records().settings.language, "en");
    assert!(view.records().chosen_sets.contains("starter"));
}

#[test]
fn readable_rule_before_creation_is_rejected_even_when_the_creation_rule_is_held() {
    let mut rows = wire_history();
    let mut early = wire_rule(&rows);
    for row in &mut rows {
        if let Some(rule) = row["pending"]["change"].get_mut("Rule") {
            rule["rule"]["schedule"] = "some_future_kind".into();
        }
    }
    early["from_day"] = serde_json::to_value(day().previous().unwrap()).unwrap();
    let cursor = rows.len() as u64 + 1;
    rows.push(wire_row(cursor, serde_json::json!({"Rule": early})));
    let mut view = werdy_sync::WatchView::new().unwrap();
    assert!(
        view.apply(serde_json::from_value(wire_page(cursor, rows)).unwrap())
            .is_err()
    );
    assert_eq!(view.cursor(), 0);
}

#[test]
fn unreadable_change_does_not_bypass_envelope_or_stream_validation() {
    let row = wire_row(1, serde_json::json!({"FutureKind": {}}));
    let mut extra = wire_page(1, vec![row.clone()]);
    extra["extra"] = true.into();
    assert!(serde_json::from_value::<Batch>(extra).is_err());
    for (field, value) in [("install", 42.into()), ("operation", "wrong".into())] {
        let mut malformed = row.clone();
        malformed["pending"][field] = value;
        assert!(serde_json::from_value::<Batch>(wire_page(1, vec![malformed])).is_err());
    }
    let mut malformed = row.clone();
    malformed["stamp"] = "wrong".into();
    assert!(serde_json::from_value::<Batch>(wire_page(1, vec![malformed])).is_err());

    let mut view = werdy_sync::WatchView::new().unwrap();
    let mut bad_install = row.clone();
    bad_install["pending"]["install"] = "invalid install".into();
    let mut zero_operation = row.clone();
    zero_operation["pending"]["operation"] = 0.into();
    let next_row = wire_row(2, row["pending"]["change"].clone());
    let mut earlier = next_row.clone();
    earlier["stamp"]["received_at"] = 999.into();
    for page in [
        wire_page(1, vec![bad_install]),
        wire_page(1, vec![zero_operation]),
        wire_page(2, vec![row.clone()]),
        wire_page(2, vec![next_row, row.clone()]),
    ] {
        assert!(view.apply(serde_json::from_value(page).unwrap()).is_err());
        assert_eq!(view.cursor(), 0);
    }
    view.apply(serde_json::from_value(wire_page(1, vec![row])).unwrap())
        .unwrap();
    assert!(
        view.apply(serde_json::from_value(wire_page(2, vec![earlier])).unwrap())
            .is_err()
    );
    assert_eq!(view.cursor(), 1);
}

#[test]
fn first_sign_in_keeps_both_histories_despite_local_id_collision() {
    let mut backend = FakeBackend::new(1_000);
    let mut first = Sync::new("first").unwrap();
    let mut account = history("account deed");
    let yesterday = day().previous().unwrap();
    account.set_day_note(
        yesterday,
        Some(DayNote::new("account note", ExcuseCredit::None, yesterday).unwrap()),
        10,
    );
    sync(&mut first, &mut backend, &mut account);

    let mut second = Sync::new("second").unwrap();
    let mut local = history("offline deed");
    local.set_day_note(
        day(),
        Some(DayNote::new("offline note", ExcuseCredit::Half, day()).unwrap()),
        20,
    );
    sync(&mut second, &mut backend, &mut local);
    sync(&mut first, &mut backend, &mut account);

    for records in [&account, &local] {
        assert_eq!(records.deeds.len(), 2);
        assert_eq!(records.marks.len(), 2);
        assert_eq!(records.day_notes.len(), 2);
        assert!(
            records
                .names
                .values()
                .any(|name| name == &DeedName::Text("offline deed".to_owned()))
        );
        assert!(
            records
                .names
                .values()
                .any(|name| name == &DeedName::Text("account deed".to_owned()))
        );
    }
}

#[test]
fn two_phones_converge_and_keep_different_rule_edits() {
    let mut backend = FakeBackend::new(1_000);
    let mut first = Sync::new("first").unwrap();
    let mut account = history("shared");
    let tomorrow = day().next().unwrap();
    let later = tomorrow.next().unwrap();
    account.deeds[0]
        .change_rule(tomorrow, rule("morning"))
        .unwrap();
    sync(&mut first, &mut backend, &mut account);
    let mut second = Sync::new("second").unwrap();
    let mut other = OwnerData::default();
    sync(&mut second, &mut backend, &mut other);

    account.deeds[0]
        .change_rule(tomorrow, rule("noon"))
        .unwrap();
    other.deeds[0].change_rule(later, rule("evening")).unwrap();
    account.settings.language = "en".to_owned();
    other.settings.theme = werdy_store::Theme::Dark;
    sync(&mut first, &mut backend, &mut account);
    sync(&mut second, &mut backend, &mut other);
    sync(&mut first, &mut backend, &mut account);

    assert_eq!(account, other);
    assert_eq!(
        account.deeds[0].rule_on(tomorrow).unwrap().slot,
        SlotKey::new("noon")
    );
    assert_eq!(
        account.deeds[0].rule_on(later).unwrap().slot,
        SlotKey::new("evening")
    );
    assert_eq!(account.settings.language, "en");
    assert_eq!(account.settings.theme, werdy_store::Theme::Dark);
}

#[test]
fn a_deed_moved_into_a_group_reaches_the_other_phone_in_that_group() {
    let mut backend = FakeBackend::new(1_000);
    let mut first = Sync::new("first").unwrap();
    let mut account = history("shared");
    sync(&mut first, &mut backend, &mut account);
    let tomorrow = day().next().unwrap();
    account.deeds[0]
        .change_rule(tomorrow, rule("heart_and_character"))
        .unwrap();
    sync(&mut first, &mut backend, &mut account);

    let mut second = Sync::new("second").unwrap();
    let mut other = OwnerData::default();
    sync(&mut second, &mut backend, &mut other);

    assert_eq!(
        other.deeds[0].rule_on(day()).unwrap().slot,
        SlotKey::new("fajr")
    );
    assert_eq!(
        other.deeds[0].rule_on(tomorrow).unwrap().slot,
        SlotKey::new("heart_and_character")
    );
}

#[test]
fn later_server_receipt_wins_even_with_earlier_phone_time() {
    let mut backend = FakeBackend::new(1_000);
    let mut first = Sync::new("first").unwrap();
    let mut account = history("shared");
    sync(&mut first, &mut backend, &mut account);
    let mut second = Sync::new("second").unwrap();
    let mut other = OwnerData::default();
    sync(&mut second, &mut backend, &mut other);
    account.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::Missed, day())),
        9_999_999,
    );
    other.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::partial_without_target(), day())),
        1,
    );
    backend.set_time(2_000);
    sync(&mut first, &mut backend, &mut account);
    backend.set_time(3_000);
    sync(&mut second, &mut backend, &mut other);
    sync(&mut first, &mut backend, &mut account);
    assert_eq!(account.marks, other.marks);
    assert_eq!(
        account.marks[&(DeedId(1), day())].mark,
        Mark::partial_without_target()
    );
    assert_eq!(account.mark_revisions[&(DeedId(1), day())], 3_000);
}

#[test]
fn years_wrong_clock_cannot_reassert_old_mark_and_reports_offset() {
    let mut backend = FakeBackend::new(1_000);
    let mut first = Sync::new("first").unwrap();
    let mut account = history("shared");
    account.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::Done, day())),
        u64::MAX,
    );
    let offset = run(first.sync(&mut backend, &mut account, || 2_000_000_000_000)).unwrap();
    assert_eq!(offset.milliseconds, 1_000 - 2_000_000_000_000_i128);
    let mut second = Sync::new("second").unwrap();
    let mut other = OwnerData::default();
    sync(&mut second, &mut backend, &mut other);
    other.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::Missed, day())),
        0,
    );
    sync(&mut second, &mut backend, &mut other);
    sync(&mut first, &mut backend, &mut account);
    assert_eq!(account.marks[&(DeedId(1), day())].mark, Mark::Missed);
    assert!(first.outbox().is_empty());
}

#[test]
fn offline_queue_survives_json_restart_and_pushes_once() {
    let mut engine = Sync::new("offline").unwrap();
    let mut records = history("offline");
    engine.capture(&records).unwrap();
    let saved = serde_json::to_string(&engine).unwrap();
    let mut restored: Sync = serde_json::from_str(&saved).unwrap();
    assert!(!restored.outbox().is_empty());
    let mut backend = FakeBackend::new(100);
    sync(&mut restored, &mut backend, &mut records);
    let cursor = run(backend.pull(0)).unwrap().cursor;
    sync(&mut restored, &mut backend, &mut records);
    assert_eq!(run(backend.pull(0)).unwrap().cursor, cursor);
    assert!(restored.outbox().is_empty());
    let mut reader = Sync::new("reader").unwrap();
    let mut remote = OwnerData::default();
    sync(&mut reader, &mut backend, &mut remote);
    assert_eq!(remote.marks, records.marks);
}

#[test]
fn fresh_phone_keeps_account_preferences_and_device_location() {
    let mut backend = FakeBackend::new(100);
    let mut first = Sync::new("first").unwrap();
    let mut account = history("shared");
    account.settings.language = "en".to_owned();
    account.chosen_sets.insert("prayers".to_owned());
    account.settings.prayer_method = werdy_store::PrayerMethod::EgyptianGeneralAuthority;
    sync(&mut first, &mut backend, &mut account);
    let mut second = Sync::new("second").unwrap();
    let mut other = OwnerData::default();
    let location = werdy_store::Location::new(None, None, 30.0, 31.0).unwrap();
    other.settings.location = Some(location.clone());
    other.settings.prayer_method = werdy_store::PrayerMethod::UmmAlQura;
    sync(&mut second, &mut backend, &mut other);
    assert_eq!(other.settings.language, "en");
    assert_eq!(other.settings.location, Some(location));
    assert!(other.chosen_sets.contains("prayers"));
    assert_eq!(account.settings.location, None);

    let local_id = other.next_deed_id();
    other.deeds.push(Deed::new(local_id, day(), rule("night")));
    other.last_deed_id = local_id.0;
    other.chosen_sets.clear();
    sync(&mut second, &mut backend, &mut other);
    sync(&mut first, &mut backend, &mut account);
    assert_eq!(account.deeds.len(), 2);
    assert!(account.chosen_sets.is_empty());
    assert_eq!(other.deeds.len(), 2);
    assert_eq!(
        account.settings.prayer_method,
        werdy_store::PrayerMethod::EgyptianGeneralAuthority
    );
    assert_eq!(
        other.settings.prayer_method,
        werdy_store::PrayerMethod::UmmAlQura
    );
}

#[test]
fn clock_sample_reports_midpoint_and_rejects_backwards_phone_jump() {
    let mut backend = FakeBackend::new(1_000);
    let mut engine = Sync::new("clock").unwrap();
    let mut records = OwnerData::default();
    let mut samples = [800, 900].into_iter();
    let offset = run(engine.sync(&mut backend, &mut records, || samples.next().unwrap())).unwrap();
    assert_eq!(offset.milliseconds, 150);
    assert_eq!(offset.round_trip_ms, 100);

    records.settings.language = "en".to_owned();
    let mut samples = [900, 800].into_iter();
    assert!(run(engine.sync(&mut backend, &mut records, || samples.next().unwrap())).is_err());
    assert!(!engine.outbox().is_empty());
    sync(&mut engine, &mut backend, &mut records);
    assert!(engine.outbox().is_empty());
}

#[test]
fn failed_push_keeps_queue_and_records() {
    let mut backend = FakeBackend::new(100);
    backend.fail_next_push();
    let mut engine = Sync::new("offline").unwrap();
    let mut records = history("offline");
    engine.capture(&records).unwrap();
    let queue = engine.outbox().to_vec();
    let before = records.clone();
    assert!(run(engine.sync(&mut backend, &mut records, || 0)).is_err());
    assert_eq!(records, before);
    assert_eq!(engine.outbox(), queue);
    assert_eq!(run(backend.pull(0)).unwrap().cursor, 0);
    sync(&mut engine, &mut backend, &mut records);
    assert!(engine.outbox().is_empty());
}

#[derive(Clone, Copy)]
enum Fault {
    LostReply,
    SuspendPush,
    MissingPull,
    UnknownDeed,
    BadStamp,
    BadArchive,
    UnreadableHeader,
}

struct FaultyBackend {
    inner: FakeBackend,
    fault: Option<Fault>,
}
impl Backend for FaultyBackend {
    async fn push(&mut self, changes: &[Pending]) -> Result<Batch, SyncError> {
        let mut batch = self.inner.push(changes).await?;
        if matches!(self.fault, Some(Fault::SuspendPush)) {
            std::future::pending::<()>().await;
        }
        if matches!(self.fault, Some(Fault::LostReply)) {
            self.fault = None;
            return Err(SyncError::Backend("reply lost after commit".to_owned()));
        }
        if matches!(self.fault, Some(Fault::BadStamp)) {
            batch.cursor = u64::MAX;
        }
        Ok(batch)
    }

    async fn pull(&mut self, cursor: u64) -> Result<Batch, SyncError> {
        let mut batch = self.inner.pull(cursor).await?;
        match self.fault {
            Some(Fault::MissingPull) => batch.changes.clear(),
            Some(Fault::UnknownDeed) => {
                if let Some(row) = batch
                    .changes
                    .iter_mut()
                    .filter_map(|row| match row {
                        BatchRow::Readable(row) => Some(row),
                        BatchRow::Held(_) => None,
                    })
                    .find(|row| matches!(row.pending.change, Change::Mark { .. }))
                    && let Change::Mark { id, .. } = &mut row.pending.change
                {
                    id.sequence = 999;
                }
            }
            Some(Fault::BadArchive) => {
                if let Some(row) = batch
                    .changes
                    .iter_mut()
                    .filter_map(|row| match row {
                        BatchRow::Readable(row) => Some(row),
                        BatchRow::Held(_) => None,
                    })
                    .find(|row| matches!(row.pending.change, Change::Deed { .. }))
                    && let Change::Deed {
                        row: Some(header), ..
                    } = &mut row.pending.change
                {
                    header
                        .past_archives
                        .push((day(), day().previous().unwrap()));
                }
            }
            Some(Fault::UnreadableHeader) => {
                for row in &mut batch.changes {
                    if matches!(
                        row.accepted().map(|row| &row.pending.change),
                        Some(Change::Deed { row: Some(_), .. })
                    ) {
                        let mut raw = serde_json::to_value(&*row).unwrap();
                        raw["pending"]["change"]["Deed"]["row"]["name"] =
                            serde_json::json!({"FutureName": "first"});
                        *row = serde_json::from_value(raw).unwrap();
                    }
                }
            }
            _ => {}
        }
        Ok(batch)
    }

    async fn server_time(&mut self) -> Result<u64, SyncError> {
        self.inner.server_time().await
    }
}

#[test]
fn malformed_backend_data_returns_error_without_changing_records_or_queue() {
    for fault in [
        Fault::MissingPull,
        Fault::UnknownDeed,
        Fault::BadStamp,
        Fault::BadArchive,
    ] {
        let mut backend = FaultyBackend {
            inner: FakeBackend::new(100),
            fault: Some(fault),
        };
        let mut seed = history("seed");
        sync(
            &mut Sync::new("seed").unwrap(),
            &mut backend.inner,
            &mut seed,
        );
        let mut engine = Sync::new("offline").unwrap();
        let mut records = OwnerData::default();
        engine.capture(&records).unwrap();
        let queue = engine.outbox().to_vec();
        let before = records.clone();
        assert!(run(engine.sync(&mut backend, &mut records, || 0)).is_err());
        assert_eq!(records, before);
        assert_eq!(engine.outbox(), queue);
        backend.fault = None;
        sync(&mut engine, &mut backend, &mut records);
        assert!(engine.outbox().is_empty());
    }
}

#[test]
fn lost_push_reply_retry_keeps_original_order_after_another_phone_edits() {
    let mut backend = FaultyBackend {
        inner: FakeBackend::new(100),
        fault: None,
    };
    let mut engine = Sync::new("first").unwrap();
    let mut records = history("shared");
    sync(&mut engine, &mut backend, &mut records);
    let mut second = Sync::new("second").unwrap();
    let mut other = OwnerData::default();
    sync(&mut second, &mut backend, &mut other);
    records.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::Missed, day())),
        10,
    );
    backend.fault = Some(Fault::LostReply);
    assert!(run(engine.sync(&mut backend, &mut records, || 0)).is_err());
    other.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::partial_without_target(), day())),
        1,
    );
    sync(&mut second, &mut backend, &mut other);
    let saved = serde_json::to_string(&engine).unwrap();
    let mut restored: Sync = serde_json::from_str(&saved).unwrap();
    sync(&mut restored, &mut backend, &mut records);
    assert_eq!(records.marks, other.marks);
    assert_eq!(
        records.marks[&(DeedId(1), day())].mark,
        Mark::partial_without_target()
    );
}

#[test]
fn clears_deletions_and_archive_history_survive_sync_and_stale_phone() {
    let mut backend = FakeBackend::new(100);
    let mut engine = Sync::new("first").unwrap();
    let mut records = history("shared");
    let tomorrow = day().next().unwrap();
    records.deeds[0].archive(day()).unwrap();
    records.deeds[0].restore(tomorrow).unwrap();
    records.set_day_note(
        day(),
        Some(DayNote::new("ill", ExcuseCredit::Full, day()).unwrap()),
        1,
    );
    sync(&mut engine, &mut backend, &mut records);
    let mut second = Sync::new("second").unwrap();
    let mut other = OwnerData::default();
    sync(&mut second, &mut backend, &mut other);
    assert!(other.deeds[0].is_archived_on(day()));
    assert!(!other.deeds[0].is_archived_on(tomorrow));
    records.set_mark(DeedId(1), day(), None, 2);
    records.set_day_note(day(), None, 2);
    sync(&mut engine, &mut backend, &mut records);
    sync(&mut second, &mut backend, &mut other);
    assert!(other.marks.is_empty());
    assert!(other.day_notes.is_empty());
    records.deeds.clear();
    records.names.clear();
    records.deleted_deeds.insert(DeedId(1));
    sync(&mut engine, &mut backend, &mut records);
    sync(&mut second, &mut backend, &mut other);
    assert!(other.deeds.is_empty());
    assert!(other.deleted_deeds.contains(&DeedId(1)));
}

#[test]
fn a_mark_cleared_during_sync_keeps_its_local_time() {
    let mut backend = FakeBackend::new(1_000);
    let mut engine = Sync::new("first").unwrap();
    let mut records = history("shared");
    sync(&mut engine, &mut backend, &mut records);

    let mut background = engine.clone();
    let mut background_records = records.clone();
    let clear_time = 3_000;
    records.set_mark(DeedId(1), day(), None, clear_time);
    engine.capture(&records).unwrap();

    sync(&mut background, &mut backend, &mut background_records);
    engine.finish_from(&background, &mut records).unwrap();

    assert_eq!(
        records.cleared_marks.get(&(DeedId(1), day())),
        Some(&clear_time)
    );
}

#[test]
fn exhausted_local_ids_return_error_instead_of_aliasing_remote_deed() {
    let mut backend = FakeBackend::new(100);
    let mut seed = history("account");
    sync(&mut Sync::new("seed").unwrap(), &mut backend, &mut seed);
    let mut records = OwnerData::default();
    records
        .deeds
        .push(Deed::new(DeedId(u64::MAX), day(), rule("night")));
    let before = records.clone();
    let mut engine = Sync::new("full").unwrap();
    assert_eq!(
        run(engine.sync(&mut backend, &mut records, || 0)),
        Err(SyncError::Exhausted)
    );
    assert_eq!(records, before);
    assert!(!engine.outbox().is_empty());
}

#[test]
fn cancelled_suspended_push_keeps_queue_and_retries_without_duplicates() {
    let mut backend = FaultyBackend {
        inner: FakeBackend::new(100),
        fault: Some(Fault::SuspendPush),
    };
    let mut records = history("offline");
    let mut engine = Sync::new("cancelled").unwrap();
    {
        let mut future = pin!(engine.sync(&mut backend, &mut records, || 0));
        assert!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
    }
    assert!(!engine.outbox().is_empty());
    let cursor = run(backend.inner.pull(0)).unwrap().cursor;
    backend.fault = None;
    sync(&mut engine, &mut backend, &mut records);
    assert_eq!(run(backend.inner.pull(0)).unwrap().cursor, cursor);
    assert!(engine.outbox().is_empty());
}

struct FailSecondBatch {
    inner: FakeBackend,
    pushes: usize,
}
impl Backend for FailSecondBatch {
    async fn push(&mut self, changes: &[Pending]) -> Result<Batch, SyncError> {
        self.pushes += 1;
        if self.pushes == 2 {
            return Err(SyncError::Backend("second batch failed".into()));
        }
        self.inner.push(changes).await
    }
    async fn pull(&mut self, cursor: u64) -> Result<Batch, SyncError> {
        self.inner.pull(cursor).await
    }
    async fn server_time(&mut self) -> Result<u64, SyncError> {
        self.inner.server_time().await
    }
}

#[test]
fn later_batch_failure_keeps_whole_outbox_and_retry_deduplicates_earlier_batches() {
    let mut backend = FailSecondBatch {
        inner: FakeBackend::new(1000),
        pushes: 0,
    };
    let mut engine = Sync::new("batch").unwrap();
    let mut records = history("large import");
    let mut marked = day();
    for _ in 0..80 {
        records.set_mark(
            DeedId(1),
            marked,
            Some(MarkEntry::new(Mark::Done, marked)),
            1,
        );
        marked = marked.next().unwrap();
    }
    engine.capture(&records).unwrap();
    let pending = engine.outbox().to_vec();
    let before = records.clone();
    assert!(run(engine.sync(&mut backend, &mut records, || 1000)).is_err());
    assert_eq!(engine.outbox(), pending);
    assert_eq!(records, before);
    assert_eq!(run(backend.inner.pull(0)).unwrap().cursor, 32);
    sync(&mut engine, &mut backend, &mut records);
    assert_eq!(
        run(backend.inner.pull(0)).unwrap().cursor,
        pending.len() as u64
    );
    assert!(engine.outbox().is_empty());
}

#[test]
fn failed_second_push_never_leaves_a_deed_header_without_its_rules() {
    let mut records = OwnerData::default();
    records.set_day_note(
        day(),
        Some(DayNote::new("note", ExcuseCredit::None, day()).unwrap()),
        1,
    );
    let mut engine = Sync::new("grouped").unwrap();
    engine.capture(&records).unwrap();
    // The prior note puts a header at operation 32.
    for id in 1..=15 {
        records
            .deeds
            .push(Deed::new(DeedId(id), day(), rule("fajr")));
    }
    records
        .deeds
        .push(Deed::new(DeedId(16), day(), rule("night")));
    let mut backend = FailSecondBatch {
        inner: FakeBackend::new(1000),
        pushes: 0,
    };
    assert!(run(engine.sync(&mut backend, &mut records, || 1000)).is_err());
    let partial = run(backend.inner.pull(0)).unwrap();
    for row in &partial.changes {
        let row = row.accepted().unwrap();
        if let Change::Deed { id, row: Some(_) } = &row.pending.change {
            assert!(partial.changes.iter().any(|row|
                matches!(&row.accepted().unwrap().pending.change, Change::Rule { id: rule_id, .. } if rule_id == id)));
        }
    }
    let mut fresh = OwnerData::default();
    sync(
        &mut Sync::new("fresh").unwrap(),
        &mut backend.inner,
        &mut fresh,
    );
    assert!(!fresh.deeds.is_empty());
}

#[test]
fn incomplete_header_and_marks_wait_for_later_rule_without_stalling_cursor() {
    let mut source = Sync::new("source").unwrap();
    source.capture(&history("shared")).unwrap();
    let header = source
        .outbox()
        .iter()
        .find(|p| matches!(p.change, Change::Deed { .. }))
        .unwrap()
        .clone();
    let rule = source
        .outbox()
        .iter()
        .find(|p| matches!(p.change, Change::Rule { .. }))
        .unwrap()
        .clone();
    let mark = source
        .outbox()
        .iter()
        .find(|p| matches!(p.change, Change::Mark { .. }))
        .unwrap()
        .clone();
    let mut backend = FakeBackend::new(1000);
    run(backend.push(&[header, mark])).unwrap();
    let mut engine = Sync::new("reader").unwrap();
    let mut records = OwnerData::default();
    sync(&mut engine, &mut backend, &mut records);
    assert!(records.deeds.is_empty());
    assert!(records.deed_keys.is_empty());
    assert!(records.marks.is_empty());
    // A restart must preserve the deferred header and its cursor.
    let mut engine: Sync = serde_json::from_str(&serde_json::to_string(&engine).unwrap()).unwrap();
    run(backend.push(&[rule])).unwrap();
    sync(&mut engine, &mut backend, &mut records);
    assert_eq!(records.deeds.len(), 1);
    assert_eq!(records.deed_keys.len(), 1);
    assert_eq!(records.marks.len(), 1);
}

#[test]
fn a_deed_with_more_than_32_rules_is_sent_as_one_atomic_group() {
    let mut records = history("many rules");
    let mut next = day();
    for _ in 0..40 {
        next = next.next().unwrap();
        records.deeds[0].change_rule(next, rule("night")).unwrap();
        records.set_day_note(
            next,
            Some(DayNote::new("note", ExcuseCredit::None, next).unwrap()),
            1,
        );
    }
    let mut backend = FailSecondBatch {
        inner: FakeBackend::new(1000),
        pushes: 0,
    };
    let mut engine = Sync::new("large_deed").unwrap();
    assert!(run(engine.sync(&mut backend, &mut records, || 1000)).is_err());
    let partial = run(backend.inner.pull(0)).unwrap();
    assert_eq!(
        partial
            .changes
            .iter()
            .map(|row| row.accepted().unwrap())
            .filter(|row| matches!(row.pending.change, Change::Rule { .. }))
            .count(),
        41
    );
    let mut fresh = OwnerData::default();
    sync(
        &mut Sync::new("reader").unwrap(),
        &mut backend.inner,
        &mut fresh,
    );
    assert_eq!(fresh.deeds[0].rule_versions().count(), 41);
}

#[test]
fn oversized_text_and_keys_stay_local_without_blocking_capture() {
    let mut cases = Vec::new();
    let mut note = history("valid");
    note.set_day_note(
        day(),
        Some(DayNote::new(&"ن".repeat(4097), ExcuseCredit::None, day()).unwrap()),
        1,
    );
    cases.push((note, "note text exceeds 8192 bytes"));
    cases.push((history(&"ن".repeat(513)), "deed name exceeds 1024 bytes"));
    let mut keyed_name = history("valid");
    keyed_name
        .names
        .insert(DeedId(1), DeedName::Key(String::new()));
    cases.push((keyed_name, "deed name key must be 1 to 128 bytes"));
    let mut starter = history("valid");
    starter.starter_keys.insert(DeedId(1), "ن".repeat(65));
    cases.push((starter, "starter key must be 1 to 128 bytes"));
    let mut chosen = history("valid");
    chosen.chosen_sets.insert(String::new());
    cases.push((chosen, "chosen set key must be 1 to 128 bytes"));
    let mut unit = history("valid");
    let mut changed = rule("fajr");
    changed.target = Some(werdy_core::Target::new(1, &"ن".repeat(65)).unwrap());
    unit.deeds[0] = Deed::new(DeedId(1), day(), changed);
    cases.push((unit, "target unit key must be 1 to 128 bytes"));
    for (category, slot, reason) in [
        (
            "".into(),
            "fajr".into(),
            "category key must be 1 to 128 bytes",
        ),
        (
            "ن".repeat(65),
            "fajr".into(),
            "category key must be 1 to 128 bytes",
        ),
        (
            "prayer".into(),
            "".into(),
            "slot key must be 1 to 128 bytes",
        ),
        (
            "prayer".into(),
            "ن".repeat(65),
            "slot key must be 1 to 128 bytes",
        ),
    ] {
        let mut records = history("valid");
        let mut changed = rule(&slot);
        changed.category = CategoryKey(category);
        records.deeds[0] = Deed::new(DeedId(1), day(), changed);
        cases.push((records, reason));
    }
    for (mut records, reason) in cases {
        let mut engine = Sync::new("limits").unwrap();
        engine.capture(&records).unwrap();
        assert_eq!(
            engine.local_only_errors(),
            vec![SyncError::Invalid(reason.into())]
        );
        sync(&mut engine, &mut FakeBackend::new(1000), &mut records);
        assert!(engine.outbox().is_empty());
    }
    let mut boundary = history(&"ن".repeat(512));
    boundary.set_day_note(
        day(),
        Some(DayNote::new(&"ن".repeat(4096), ExcuseCredit::None, day()).unwrap()),
        1,
    );
    let mut changed = rule(&"ن".repeat(64));
    changed.category = CategoryKey::new("a");
    boundary.deeds[0] = Deed::new(DeedId(1), day(), changed);
    let mut engine = Sync::new("boundary").unwrap();
    engine.capture(&boundary).unwrap();
    sync(&mut engine, &mut FakeBackend::new(1000), &mut boundary);
}

#[test]
fn first_capture_keeps_oversized_note_local_and_later_shortening_syncs_it() {
    let huge = "ن".repeat(4097);
    let mut records = history("valid deed");
    records.set_day_note(
        day(),
        Some(DayNote::new(&huge, ExcuseCredit::None, day()).unwrap()),
        1,
    );
    let mut engine = Sync::new("import").unwrap();
    engine.capture(&records).unwrap();
    assert_eq!(
        engine.local_only_errors(),
        vec![SyncError::Invalid("note text exceeds 8192 bytes".into())]
    );
    assert!(!engine.outbox().is_empty());
    assert!(
        engine
            .outbox()
            .iter()
            .all(|row| !matches!(row.change, Change::Note { .. }))
    );
    let mut backend = FakeBackend::new(1000);
    sync(&mut engine, &mut backend, &mut records);
    assert_eq!(records.day_notes[&day()].text(), huge);
    let mut other = OwnerData::default();
    let mut reader = Sync::new("reader").unwrap();
    sync(&mut reader, &mut backend, &mut other);
    assert_eq!(other.deeds.len(), 1);
    assert_eq!(other.marks.len(), 1);
    assert!(other.day_notes.is_empty());
    records.set_day_note(
        day(),
        Some(DayNote::new("short", ExcuseCredit::None, day()).unwrap()),
        2,
    );
    engine.capture(&records).unwrap();
    assert!(engine.local_only_errors().is_empty());
    assert!(engine.outbox().iter().any(
        |row| matches!(&row.change, Change::Note { note: Some(note), .. } if note.text() == "short")
    ));
    sync(&mut engine, &mut backend, &mut records);
    sync(&mut reader, &mut backend, &mut other);
    assert_eq!(other.day_notes[&day()].text(), "short");
}

#[test]
fn later_oversized_note_edit_stays_local_while_other_edits_sync() {
    let mut records = history("valid deed");
    let mut engine = Sync::new("later").unwrap();
    let mut backend = FakeBackend::new(1000);
    sync(&mut engine, &mut backend, &mut records);
    let huge = "ن".repeat(4097);
    records.set_day_note(
        day(),
        Some(DayNote::new(&huge, ExcuseCredit::None, day()).unwrap()),
        2,
    );
    records.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::Missed, day())),
        2,
    );
    engine.capture(&records).unwrap();
    assert_eq!(
        engine.local_only_errors(),
        vec![SyncError::Invalid("note text exceeds 8192 bytes".into())]
    );
    sync(&mut engine, &mut backend, &mut records);
    assert_eq!(records.day_notes[&day()].text(), huge);
    let mut other = OwnerData::default();
    sync(&mut Sync::new("reader").unwrap(), &mut backend, &mut other);
    assert!(other.day_notes.is_empty());
    assert_eq!(other.marks.values().next().unwrap().mark, Mark::Missed);
}

#[test]
fn legacy_oversized_outbox_keeps_local_rows_and_does_not_block_valid_changes() {
    let mut records = history("valid");
    records.set_day_note(
        day(),
        Some(DayNote::new("small", ExcuseCredit::None, day()).unwrap()),
        1,
    );
    let mut engine = Sync::new("legacy").unwrap();
    engine.capture(&records).unwrap();
    let huge_note = "ن".repeat(4097);
    let huge_name = "ن".repeat(513);
    records.set_day_note(
        day(),
        Some(DayNote::new(&huge_note, ExcuseCredit::None, day()).unwrap()),
        2,
    );
    records
        .names
        .insert(DeedId(1), DeedName::Text(huge_name.clone()));
    // Simulate an engine written before client validation existed.
    let mut value = serde_json::to_value(&engine).unwrap();
    for field in ["outbox", "observed"] {
        for row in value[field].as_array_mut().unwrap() {
            let change = if field == "outbox" {
                &mut row["change"]
            } else {
                row
            };
            if change.get("Note").is_some() {
                change["Note"]["note"]["text"] = huge_note.clone().into();
            }
            if change.get("Deed").is_some() {
                change["Deed"]["row"]["name"] = serde_json::json!({"text":huge_name});
            }
        }
    }
    let mut engine: Sync = serde_json::from_value(value).unwrap();
    let tomorrow = day().next().unwrap();
    records.set_day_note(
        tomorrow,
        Some(DayNote::new("valid new note", ExcuseCredit::None, tomorrow).unwrap()),
        3,
    );
    let mut backend = FakeBackend::new(1000);
    sync(&mut engine, &mut backend, &mut records);
    assert!(engine.outbox().is_empty());
    assert_eq!(records.day_notes[&day()].text(), huge_note);
    assert_eq!(records.names[&DeedId(1)], DeedName::Text(huge_name));
    let stream = run(backend.pull(0)).unwrap();
    assert!(stream.changes.iter().all(|row| !matches!(
        row.accepted().unwrap().pending.change,
        Change::Deed { .. } | Change::Rule { .. } | Change::Mark { .. }
    )));
    let mut fresh = OwnerData::default();
    sync(&mut Sync::new("fresh").unwrap(), &mut backend, &mut fresh);
    assert_eq!(fresh.day_notes[&tomorrow].text(), "valid new note");
    assert!(!fresh.day_notes.contains_key(&day()));
    let mut engine: Sync = serde_json::from_str(&serde_json::to_string(&engine).unwrap()).unwrap();
    sync(&mut engine, &mut backend, &mut records);
    assert_eq!(records.day_notes[&day()].text(), huge_note);
    records
        .names
        .insert(DeedId(1), DeedName::Text("fixed".into()));
    records.set_day_note(
        day(),
        Some(DayNote::new("fixed", ExcuseCredit::None, day()).unwrap()),
        4,
    );
    sync(&mut engine, &mut backend, &mut records);
    sync(&mut Sync::new("other").unwrap(), &mut backend, &mut fresh);
    assert_eq!(fresh.deeds.len(), 1);
    assert_eq!(fresh.day_notes[&day()].text(), "fixed");
}

#[test]
fn watched_pages_preserve_late_and_excuse_signs_without_touching_owner_state() {
    let mut records = history("shared");
    records.settings.location = werdy_store::Location::new(None, None, 30.04, 31.24);
    let late = day().next().unwrap().next().unwrap().next().unwrap();
    records.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::Missed, late)),
        1,
    );
    records.set_day_note(
        day(),
        Some(DayNote::new("ill", ExcuseCredit::Half, late).unwrap()),
        1,
    );
    let mut backend = FakeBackend::new(1000);
    let mut owner = Sync::new("owner").unwrap();
    sync(&mut owner, &mut backend, &mut records);
    let before = serde_json::to_string(&owner).unwrap();
    let stream = run(backend.pull(0)).unwrap();
    let mut watched = werdy_sync::WatchView::new().unwrap();
    for accepted in stream.changes {
        let cursor = accepted.stamp().sequence;
        watched
            .apply(Batch {
                changes: vec![accepted],
                cursor,
            })
            .unwrap();
    }
    assert_eq!(watched.records().day_notes[&day()].text(), "ill");
    assert!(watched.records().day_notes[&day()].is_late_for(day()));
    assert_eq!(
        watched.records().day_notes[&day()].credit(),
        ExcuseCredit::Half
    );
    let local = watched.records().deeds[0].id;
    assert!(
        werdy_core::daily_sheet(
            &watched.records().deeds,
            &watched.records().marks,
            day(),
            &[],
            watched.records().settings.calendar.clone()
        )
        .slots
        .iter()
        .flat_map(|slot| &slot.rows)
        .any(|row| row.deed_id == local && row.added_later)
    );
    assert_eq!(serde_json::to_string(&owner).unwrap(), before);
    assert!(watched.records().settings.location.is_none());
    assert_eq!(
        werdy_core::score(&watched.records().records(), &[day()], late).unwrap(),
        werdy_core::score(&records.records(), &[day()], late).unwrap()
    );
    let cursor = watched.cursor();
    let before = watched.records().clone();
    assert!(
        watched
            .apply(Batch {
                changes: vec![],
                cursor: cursor + 1
            })
            .is_err()
    );
    assert_eq!(watched.records(), &before);
    assert_eq!(watched.cursor(), cursor);
}

#[test]
fn moving_install_keeps_deed_keys_and_renumbers_pending_changes() {
    let mut engine = Sync::new("old").unwrap();
    let mut records = history("first");
    engine.capture(&records).unwrap();
    engine.remember_deed_keys(&mut records);
    let original = records.deed_keys.clone();
    let queued = engine.outbox().to_vec();
    assert!(engine.move_install("").is_err());
    assert!(engine.move_install("old").is_err());
    assert_eq!(engine.outbox(), queued);
    engine.move_install("fresh").unwrap();
    for (index, (before, after)) in queued.iter().zip(engine.outbox()).enumerate() {
        assert_eq!(after.change, before.change);
        assert_eq!(after.operation, index as u64 + 1);
        assert_eq!(after.install, "fresh");
    }
    let second = records.next_deed_id();
    records.last_deed_id = second.0;
    records.deeds.push(Deed::new(second, day(), rule("noon")));
    engine.capture(&records).unwrap();
    engine.remember_deed_keys(&mut records);
    assert_eq!(records.deed_keys[&DeedId(1)], original[&DeedId(1)]);
    assert_eq!(records.deed_keys[&second].install, "fresh");
    assert_eq!(
        engine.outbox()[queued.len()].operation,
        queued.len() as u64 + 1
    );
}

fn recover(engine: &mut Sync, backend: &mut impl Backend, records: &mut OwnerData) {
    let Err(SyncError::Collision(collision)) = run(engine.sync(backend, records, || 1000)) else {
        panic!("expected an operation collision");
    };
    engine
        .recover_collision(&collision, "fresh", records)
        .unwrap();
}

#[test]
fn copies_with_one_install_recover_both_edits_on_one_deed() {
    let mut backend = FakeBackend::new(1000);
    let mut first = Sync::new("shared").unwrap();
    let mut records = history("original");
    sync(&mut first, &mut backend, &mut records);
    let mut second = first.clone();
    let mut other = records.clone();
    records
        .names
        .insert(DeedId(1), DeedName::Text("renamed".into()));
    other.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::Missed, day())),
        2,
    );
    sync(&mut first, &mut backend, &mut records);
    recover(&mut second, &mut backend, &mut other);
    assert_eq!(other.names, records.names);
    sync(&mut second, &mut backend, &mut other);
    let mut reader = OwnerData::default();
    sync(&mut Sync::new("reader").unwrap(), &mut backend, &mut reader);
    assert_eq!(reader.deeds.len(), 1);
    assert_eq!(reader.names, records.names);
    assert_eq!(reader.marks[&(DeedId(1), day())].mark, Mark::Missed);
}

#[test]
fn collision_after_accepted_batch_retries_only_missing_rows() {
    let mut backend = FakeBackend::new(1000);
    let mut engine = Sync::new("shared").unwrap();
    let mut records = OwnerData::default();
    let mut date = day();
    for _ in 0..33 {
        records.set_day_note(
            date,
            Some(DayNote::new("local", ExcuseCredit::None, date).unwrap()),
            1,
        );
        date = date.next().unwrap();
    }
    engine.capture(&records).unwrap();
    let mut taken = engine.outbox()[0].clone();
    taken.operation = engine.outbox()[32].operation;
    run(backend.push(&[taken])).unwrap();
    recover(&mut engine, &mut backend, &mut records);
    assert_eq!(engine.outbox().len(), 1);
    assert_eq!(run(backend.pull(0)).unwrap().cursor, 33);
    sync(&mut engine, &mut backend, &mut records);
    let log = run(backend.pull(0)).unwrap();
    assert_eq!(log.cursor, 34);
    assert_eq!(
        log.changes
            .last()
            .unwrap()
            .accepted()
            .unwrap()
            .pending
            .install,
        "fresh"
    );
    assert_eq!(records.day_notes.len(), 33);
}

#[test]
fn collision_keeps_note_and_mark_clears() {
    let mut backend = FakeBackend::new(1000);
    let mut first = Sync::new("shared").unwrap();
    let mut a = history("shared");
    sync(&mut first, &mut backend, &mut a);
    let mut second = first.clone();
    let mut b = a.clone();
    let tomorrow = day().next().unwrap();
    for (engine, records, text, mark) in [
        (&mut first, &mut a, "one", Mark::Missed),
        (&mut second, &mut b, "two", Mark::partial_without_target()),
    ] {
        for date in [day(), tomorrow] {
            records.set_day_note(
                date,
                Some(DayNote::new(text, ExcuseCredit::None, date).unwrap()),
                1,
            );
        }
        records.set_mark(DeedId(1), day(), Some(MarkEntry::new(mark, day())), 1);
        engine.capture(records).unwrap();
        records.set_day_note(day(), None, 2);
        records.set_day_note(tomorrow, None, 2);
        records.set_mark(DeedId(1), day(), None, 2);
        engine.capture(records).unwrap();
    }
    sync(&mut first, &mut backend, &mut a);
    let newer = DayNote::new("later edit", ExcuseCredit::None, tomorrow).unwrap();
    b.set_day_note(tomorrow, Some(newer.clone()), 3);
    recover(&mut second, &mut backend, &mut b);
    sync(&mut second, &mut backend, &mut b);
    let mut reader = OwnerData::default();
    sync(&mut Sync::new("reader").unwrap(), &mut backend, &mut reader);
    assert!(!reader.day_notes.contains_key(&day()));
    assert_eq!(reader.day_notes.get(&tomorrow), Some(&newer));
    assert!(reader.marks.is_empty());
}

#[test]
fn collision_keeps_two_new_deeds_with_their_own_marks() {
    let mut backend = FakeBackend::new(1000);
    let mut first = Sync::new("shared").unwrap();
    let mut second = first.clone();
    let mut a = history("first");
    let mut b = history("second");
    b.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::Missed, day())),
        1,
    );
    sync(&mut first, &mut backend, &mut a);
    recover(&mut second, &mut backend, &mut b);
    sync(&mut second, &mut backend, &mut b);
    let mut reader = OwnerData::default();
    sync(&mut Sync::new("reader").unwrap(), &mut backend, &mut reader);
    assert_eq!(reader.deeds.len(), 2);
    for (name, install, mark) in [
        ("first", "shared", Mark::Done),
        ("second", "fresh", Mark::Missed),
    ] {
        let id = reader
            .names
            .iter()
            .find(|(_, value)| **value == DeedName::Text(name.into()))
            .unwrap()
            .0;
        assert_eq!(reader.deed_keys[id].install, install);
        assert_eq!(reader.marks[&(*id, day())].mark, mark);
    }
}

#[test]
fn collision_keeps_local_deed_separate_from_held_server_header() {
    let mut backend = FaultyBackend {
        inner: FakeBackend::new(1000),
        fault: None,
    };
    let mut first = Sync::new("shared").unwrap();
    let mut second = first.clone();
    sync(&mut first, &mut backend, &mut history("first"));
    backend.fault = Some(Fault::UnreadableHeader);
    let mut records = history("second");

    recover(&mut second, &mut backend, &mut records);
    assert_eq!(records.deed_keys[&DeedId(1)].install, "fresh");
    let state = serde_json::to_value(&second).unwrap();
    assert_eq!(state["held"].as_array().unwrap().len(), 1);
    assert_eq!(second.outbox().len(), 3);
    for pending in second.outbox() {
        match &pending.change {
            Change::Deed { id, .. } | Change::Rule { id, .. } | Change::Mark { id, .. } => {
                assert_eq!(id.install, "fresh");
            }
            change => panic!("unexpected pending change {change:?}"),
        }
    }
}
