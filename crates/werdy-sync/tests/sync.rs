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
