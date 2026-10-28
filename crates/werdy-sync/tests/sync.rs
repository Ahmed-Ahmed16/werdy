#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use werdy_core::{
    CategoryKey, DayNote, Deed, DeedId, DeedRule, ExcuseCredit, Mark, MarkEntry, Schedule, SlotKey,
    WerdyDay,
};
use werdy_store::{DeedName, OwnerData};
use werdy_sync::{Backend, Batch, Change, FakeBackend, Pending, Sync, SyncError};

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
        if let Change::Deed { id, row: Some(_) } = &row.pending.change {
            assert!(partial.changes.iter().any(|row|
                matches!(&row.pending.change, Change::Rule { id: rule_id, .. } if rule_id == id)));
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
        row.pending.change,
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
        let cursor = accepted.stamp.sequence;
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
