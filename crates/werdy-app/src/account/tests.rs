#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::state::AppState;
use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};
use werdy_core::{
    CategoryKey, Deed, DeedId, DeedRule, Mark, MarkEntry, Schedule as DeedSchedule, SlotKey,
    WerdyDay,
};
use werdy_sync::{Backend, Batch, Change, FakeBackend, Pending, SyncError};

fn run<F: Future>(future: F) -> F::Output {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("fake backend suspended"),
    }
}
fn session() -> Session {
    Session {
        access_token: "access".into(),
        refresh_token: "refresh".into(),
        expires_at: 5000,
        user_id: "00000000-0000-4000-8000-000000000023".into(),
        email: Some("a@example.test".into()),
    }
}
fn day() -> WerdyDay {
    WerdyDay::new(2026, 10, 28).unwrap()
}
fn deed(id: u64) -> Deed {
    Deed::new(
        DeedId(id),
        day(),
        DeedRule {
            schedule: DeedSchedule::EveryDay,
            target: None,
            category: CategoryKey::new("prayer"),
            slot: SlotKey::new("fajr"),
        },
    )
}
fn records() -> OwnerData {
    OwnerData {
        deeds: vec![deed(1)],
        last_deed_id: 1,
        ..OwnerData::default()
    }
}
fn store() -> AccountStore {
    AccountStore::open(
        Rc::new(werdy_store::MemoryStore::default()),
        Rc::new(storage::MemoryVault::default()),
        true,
    )
}
fn flight(store: &AccountStore, backend: &mut impl Backend) -> Checkpoint {
    let mut checkpoint = store.account.borrow().checkpoint.clone().unwrap();
    let mut records = from_json(&checkpoint.records).unwrap();
    run(checkpoint.engine.sync(backend, &mut records, || 1000)).unwrap();
    checkpoint.records = to_json(&records).unwrap();
    checkpoint
}

fn legacy_level_sheet() -> OwnerData {
    let rule = DeedRule {
        schedule: DeedSchedule::EveryDay,
        target: Some(werdy_core::Target::new(1, "times").unwrap()),
        category: CategoryKey::new("family"),
        slot: SlotKey::new("any_time"),
    };
    OwnerData {
        deeds: vec![Deed::new(DeedId(1), day(), rule)],
        last_deed_id: 1,
        names: [(
            DeedId(1),
            werdy_store::DeedName::Key("deed-lv-parents".into()),
        )]
        .into(),
        starter_keys: [(DeedId(1), "lv_parents".into())].into(),
        ..OwnerData::default()
    }
}

fn apply_flight(store: &AccountStore, backend: &mut impl Backend, state: &mut AppState) {
    let completed = flight(store, backend);
    let records = store.complete(&completed, 1, 2000).unwrap().unwrap();
    runtime::apply_successful_sync(state, records, store);
}

#[test]
fn first_sync_moves_the_pulled_rule_and_queues_it_for_the_next_sync() {
    let store = store();
    let mut local = legacy_level_sheet();
    local.deeds[0].archive(day().next().unwrap()).unwrap();
    store.sign_in(session(), &local).unwrap();
    let mut backend = FakeBackend::new(1000);
    let today = day().next().unwrap().next().unwrap();
    let mut phone = AppState::start(&store, today, crate::starter::slot_order());

    let initial = flight(&store, &mut backend);
    store.complete(&initial, 1, 1000).unwrap();
    let mut other_engine = Sync::new("other").unwrap();
    let mut other = OwnerData::default();
    run(other_engine.sync(&mut backend, &mut other, || 1000)).unwrap();
    let edit_day = today.previous().unwrap();
    let mut edited = other.deeds[0].rule_on(today).unwrap().clone();
    edited.target = Some(werdy_core::Target::new(9, "times").unwrap());
    edited.category = CategoryKey::new("knowledge");
    edited.schedule = DeedSchedule::Weekdays([werdy_core::Weekday::Friday].into());
    other.deeds[0]
        .change_rule(edit_day, edited.clone())
        .unwrap();
    run(other_engine.sync(&mut backend, &mut other, || 2000)).unwrap();

    apply_flight(&store, &mut backend, &mut phone);

    let mut grouped = edited.clone();
    grouped.slot = SlotKey::new("heart_and_character");
    assert_eq!(phone.data.deeds[0].rule_on(edit_day), Some(&edited));
    assert_eq!(phone.data.deeds[0].rule_on(today), Some(&grouped));
    assert!(phone.data.deeds[0].is_archived_on(today));
    assert_eq!(store.load().unwrap().unwrap().deeds, phone.data.deeds);
    assert!(store.account.borrow().checkpoint.as_ref().unwrap().engine.outbox().iter().any(
        |pending| matches!(&pending.change, Change::Rule { from_day, .. } if *from_day == today)
    ));
    let later = legacy_level_sheet();
    let new_id = other.next_deed_id();
    other.last_deed_id = new_id.0;
    other.deeds.push(Deed::new(
        new_id,
        day(),
        later.deeds[0].rule_on(day()).unwrap().clone(),
    ));
    other
        .names
        .insert(new_id, werdy_store::DeedName::Key("deed-lv-kinship".into()));
    other.starter_keys.insert(new_id, "lv_kinship".into());
    run(other_engine.sync(&mut backend, &mut other, || 3000)).unwrap();
    apply_flight(&store, &mut backend, &mut phone);
    let new_deed = phone
        .data
        .deeds
        .iter()
        .find(|deed| {
            phone
                .data
                .starter_keys
                .get(&deed.id)
                .is_some_and(|key| key == "lv_kinship")
        })
        .unwrap();
    assert_eq!(new_deed.rule_on(today).unwrap().slot.0, "any_time");
    run(other_engine.sync(&mut backend, &mut other, || 3000)).unwrap();
    assert_eq!(other.deeds[0].rule_on(today), Some(&grouped));
}

#[test]
fn first_sync_keeps_a_pulled_slot_and_a_later_move_back_to_any_time() {
    let mut backend = FakeBackend::new(1000);
    let mut other_engine = Sync::new("other").unwrap();
    let today = day().next().unwrap().next().unwrap().next().unwrap();
    let mut other = legacy_level_sheet();
    run(other_engine.sync(&mut backend, &mut other, || 1000)).unwrap();
    other_engine.remember_deed_keys(&mut other);
    let stale = other.clone();
    let mut rule = other.deeds[0].rule_on(today).unwrap().clone();
    for (edit_day, slot) in [
        (day().next().unwrap(), "heart_and_character"),
        (today.previous().unwrap(), "any_time"),
    ] {
        rule.slot = SlotKey::new(slot);
        other.deeds[0].change_rule(edit_day, rule.clone()).unwrap();
        run(other_engine.sync(&mut backend, &mut other, || 1000)).unwrap();

        let store = store();
        store.sign_in(session(), &stale).unwrap();
        let mut phone = AppState::start(&store, today, crate::starter::slot_order());
        apply_flight(&store, &mut backend, &mut phone);
        assert!(phone.data.level_group_move_handled.is_empty());
        assert_eq!(phone.data.deeds[0].rule_on(today), Some(&rule));
        assert_eq!(
            phone.data.deeds[0].rule_versions().count(),
            other.deeds[0].rule_versions().count()
        );
        assert!(
            store
                .account
                .borrow()
                .checkpoint
                .as_ref()
                .unwrap()
                .engine
                .outbox()
                .is_empty()
        );
        let cursor = run(backend.pull(0)).unwrap().cursor;
        apply_flight(&store, &mut backend, &mut phone);
        assert!(run(backend.pull(cursor)).unwrap().changes.is_empty());
    }
}

#[test]
fn debounce_keeps_one_attempt_and_does_not_delay_periodic_sync_on_success() {
    let mut schedule = Schedule::default();
    schedule.immediate();
    assert!(schedule.start(0));
    assert!(!schedule.start(1));
    schedule.saved(10);
    schedule.saved(100);
    schedule.finish(200, true);
    assert!(!schedule.start(3099));
    assert!(schedule.start(3100));
    schedule.finish(4000, true);
    assert!(!schedule.start(303999));
    assert!(schedule.start(304000));
    schedule.finish(305000, false);
    assert!(!schedule.start(334999));
    assert!(schedule.start(335000));
}

#[test]
fn continuous_saves_cannot_postpone_five_minute_sync() {
    let mut schedule = Schedule::default();
    schedule.immediate();
    assert!(schedule.start(0));
    schedule.finish(0, true);
    for now in (1000..300000).step_by(1000) {
        schedule.saved(now);
    }
    assert!(schedule.start(300000));
}

#[test]
fn first_merge_and_edits_during_sync_keep_marks_and_distinct_deeds() {
    let store = store();
    let mut local = records();
    store.inner.save(&local).unwrap();
    store.sign_in(session(), &local).unwrap();
    let mut backend = FakeBackend::new(1000);
    let first = flight(&store, &mut backend);
    store.complete(&first, 1, 1000).unwrap();
    let mut other_engine = Sync::new("other").unwrap();
    let mut other = OwnerData::default();
    run(other_engine.sync(&mut backend, &mut other, || 1000)).unwrap();
    other.deeds.push(deed(2));
    other.last_deed_id = 2;
    run(other_engine.sync(&mut backend, &mut other, || 1000)).unwrap();
    let downloaded = flight(&store, &mut backend);
    local = store.load().unwrap().unwrap();
    local.deeds.push(deed(2));
    local.last_deed_id = 2;
    local.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::Done, day())),
        999999,
    );
    store.save(&local).unwrap();
    let merged = store.complete(&downloaded, 1, 2000).unwrap().unwrap();
    assert_eq!(merged.deeds.len(), 3);
    assert_eq!(merged.marks[&(DeedId(1), day())].mark, Mark::Done);
    assert!(
        !store
            .account
            .borrow()
            .checkpoint
            .as_ref()
            .unwrap()
            .engine
            .outbox()
            .is_empty()
    );
    let next = flight(&store, &mut backend);
    store.complete(&next, 1, 3000).unwrap();
    run(other_engine.sync(&mut backend, &mut other, || 1000)).unwrap();
    assert_eq!(other.deeds.len(), 3);
    assert_eq!(other.marks.values().next().unwrap().mark, Mark::Done);
}

/// A sheet with one starter deed, made by its own install, as setup does.
fn starter_sheet(mark_day: WerdyDay, mark: Mark) -> OwnerData {
    let mut sheet = records();
    sheet.starter_keys.insert(DeedId(1), "fajr_on_time".into());
    sheet.set_mark(
        DeedId(1),
        mark_day,
        Some(MarkEntry::new(mark, mark_day)),
        1000,
    );
    sheet
}

struct HeldRuleBackend {
    inner: FakeBackend,
    edit_day: WerdyDay,
}

impl Backend for HeldRuleBackend {
    async fn pull(&mut self, cursor: u64) -> Result<Batch, SyncError> {
        let mut batch = serde_json::to_value(self.inner.pull(cursor).await?).unwrap();
        for row in batch["changes"].as_array_mut().unwrap() {
            let change = &mut row["pending"]["change"];
            if let Some(rule) = change.get_mut("Rule")
                && rule["from_day"] == serde_json::json!(self.edit_day)
            {
                rule["rule"]["schedule"] = "some_future_kind".into();
            }
        }
        Ok(serde_json::from_value(batch).unwrap())
    }

    async fn push(&mut self, changes: &[Pending]) -> Result<Batch, SyncError> {
        self.inner.push(changes).await
    }

    async fn server_time(&mut self) -> Result<u64, SyncError> {
        self.inner.server_time().await
    }
}

#[test]
fn first_sync_leaves_a_level_deed_with_a_held_rule_untouched() {
    let edit_day = day().next().unwrap();
    let today = edit_day.next().unwrap();
    let mut backend = HeldRuleBackend {
        inner: FakeBackend::new(1000),
        edit_day,
    };
    let original = legacy_level_sheet();
    let mut other_phone = original.clone();
    let mut edit = other_phone.deeds[0].rule_on(day()).unwrap().clone();
    edit.schedule = DeedSchedule::WhenItComesUp;
    other_phone.deeds[0].change_rule(edit_day, edit).unwrap();
    run(Sync::new("other_phone")
        .unwrap()
        .sync(&mut backend.inner, &mut other_phone, || 1000))
    .unwrap();

    let store = store();
    store.sign_in(session(), &OwnerData::default()).unwrap();
    let mut phone = AppState::start(&store, today, crate::starter::slot_order());
    apply_flight(&store, &mut backend, &mut phone);

    let account = store.account.borrow();
    let engine = &account.checkpoint.as_ref().unwrap().engine;
    let deed = &phone.data.deeds[0];
    assert_eq!(engine.held_deed_ids(), [deed.id].into());
    assert_eq!(deed.rule_on(today), original.deeds[0].rule_on(day()));
    assert_eq!(deed.rule_versions().count(), 1);
    assert!(engine.outbox().is_empty());
    assert!(phone.data.level_group_move_handled.is_empty());
}

#[test]
fn starter_merge_waits_for_held_rule_then_moves_it_to_the_kept_deed() {
    let edit_day = day().next().unwrap();
    let mut backend = HeldRuleBackend {
        inner: FakeBackend::new(1000),
        edit_day,
    };
    let mut kept = starter_sheet(day(), Mark::Done);
    run(Sync::new("a_kept")
        .unwrap()
        .sync(&mut backend.inner, &mut kept, || 1000))
    .unwrap();
    let mut copy = starter_sheet(day(), Mark::Done);
    let mut edit = copy.deeds[0].rule_on(day()).unwrap().clone();
    edit.schedule = DeedSchedule::WhenItComesUp;
    copy.deeds[0].change_rule(edit_day, edit.clone()).unwrap();
    run(Sync::new("b_copy")
        .unwrap()
        .sync(&mut backend.inner, &mut copy, || 1000))
    .unwrap();

    let store = store();
    store.sign_in(session(), &OwnerData::default()).unwrap();
    let first = flight(&store, &mut backend);
    let deferred = store.complete(&first, 1, 1000).unwrap().unwrap();
    assert_eq!(deferred.deeds.len(), 2);
    assert!(deferred.deleted_deeds.is_empty());
    let mut next = store.account.borrow().checkpoint.clone().unwrap();
    assert!(
        next.engine
            .outbox()
            .iter()
            .all(|pending| !matches!(pending.change, Change::Deed { row: None, .. }))
    );

    let mut saved = serde_json::to_value(&next.engine).unwrap();
    saved["held"][0]["pending"]["change"]["Rule"]["rule"]["schedule"] =
        serde_json::json!(edit.schedule);
    next.engine = serde_json::from_value(saved).unwrap();
    let mut next_records = from_json(&next.records).unwrap();
    run(next.engine.sync(&mut backend, &mut next_records, || 2000)).unwrap();
    next.records = to_json(&next_records).unwrap();
    let merged = store.complete(&next, 1, 2000).unwrap().unwrap();
    assert_eq!(merged.deeds.len(), 1);
    let kept = &merged.deeds[0];
    assert_eq!(merged.deed_keys[&kept.id].install, "a_kept");
    assert_eq!(kept.rule_on(edit_day), Some(&edit));
    assert_eq!(merged.deleted_deeds.len(), 1);
}

#[test]
fn signing_in_with_a_starter_sheet_merges_it_into_the_account_copy() {
    let mut backend = FakeBackend::new(1000);
    let yesterday = day().previous().unwrap();
    // The account already has the starter deed from another device.
    let mut web_engine = Sync::new("web").unwrap();
    let mut web = starter_sheet(yesterday, Mark::Done);
    run(web_engine.sync(&mut backend, &mut web, || 1000)).unwrap();

    // This phone was set up offline with its own copy, then signs in.
    let store = store();
    let phone = starter_sheet(day(), Mark::Missed);
    store.inner.save(&phone).unwrap();
    store.sign_in(session(), &phone).unwrap();
    let first = flight(&store, &mut backend);
    let merged = store.complete(&first, 1, 1000).unwrap().unwrap();

    assert_eq!(merged.deeds.len(), 1, "one deed per starter key");
    let kept = merged.deeds[0].id;
    assert_eq!(merged.marks[&(kept, yesterday)].mark, Mark::Done);
    assert_eq!(merged.marks[&(kept, day())].mark, Mark::Missed);

    // The repair reaches the account, so the other device sees one deed too.
    let next = flight(&store, &mut backend);
    store.complete(&next, 1, 2000).unwrap();
    run(web_engine.sync(&mut backend, &mut web, || 2000)).unwrap();
    assert_eq!(web.deeds.len(), 1);
    let web_deed = web.deeds[0].id;
    assert_eq!(web.marks.len(), 2);
    assert_eq!(web.marks[&(web_deed, yesterday)].mark, Mark::Done);
    assert_eq!(web.marks[&(web_deed, day())].mark, Mark::Missed);
}

#[test]
fn a_mark_made_during_the_first_sync_survives_the_starter_merge() {
    let mut backend = FakeBackend::new(1000);
    let yesterday = day().previous().unwrap();
    // The account's copy is older, so it is the one kept.
    let mut web_engine = Sync::new("web").unwrap();
    let mut web = OwnerData {
        deeds: vec![Deed::new(
            DeedId(1),
            yesterday,
            deed(1).rule_on(day()).unwrap().clone(),
        )],
        last_deed_id: 1,
        ..OwnerData::default()
    };
    web.starter_keys.insert(DeedId(1), "fajr_on_time".into());
    web.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::Missed, day())),
        1000,
    );
    run(web_engine.sync(&mut backend, &mut web, || 1000)).unwrap();

    let store = store();
    let phone = starter_sheet(yesterday, Mark::Done);
    store.inner.save(&phone).unwrap();
    store.sign_in(session(), &phone).unwrap();
    let first = flight(&store, &mut backend);
    // While that sync is in flight, the owner marks today on this phone.
    let mut during = store.load().unwrap().unwrap();
    during.set_mark(
        DeedId(1),
        day(),
        Some(MarkEntry::new(Mark::Done, day())),
        5000,
    );
    store.save(&during).unwrap();
    let merged = store.complete(&first, 1, 1000).unwrap().unwrap();

    assert_eq!(merged.deeds.len(), 1);
    let kept = merged.deeds[0].id;
    assert_eq!(merged.marks[&(kept, day())].mark, Mark::Done);
    let next = flight(&store, &mut backend);
    store.complete(&next, 1, 2000).unwrap();
    run(web_engine.sync(&mut backend, &mut web, || 2000)).unwrap();
    assert_eq!(web.marks[&(DeedId(1), day())].mark, Mark::Done);
}

#[test]
fn signout_clears_secrets_and_rejects_late_completion_without_losing_history() {
    let store = store();
    let records = records();
    store.sign_in(session(), &records).unwrap();
    let completed = flight(&store, &mut FakeBackend::new(1000));
    store.sign_out(&records).unwrap();
    assert!(store.account.borrow().session.is_none());
    assert!(store.account.borrow().checkpoint.is_none());
    assert!(store.vault.read(File::Session).unwrap().is_none());
    assert!(store.vault.read(File::Sync).unwrap().is_none());
    assert!(store.complete(&completed, 1, 1000).unwrap().is_none());
    let kept = store.load().unwrap().unwrap();
    assert_eq!(kept.deeds, records.deeds);
    assert!(!kept.deed_keys.is_empty());
}

#[test]
fn signing_in_again_after_signout_does_not_duplicate_existing_deeds() {
    let store = store();
    let mut backend = FakeBackend::new(1000);
    store.sign_in(session(), &records()).unwrap();
    let first = flight(&store, &mut backend);
    let kept = store.complete(&first, 1, 1000).unwrap().unwrap();
    store.sign_out(&kept).unwrap();
    store.sign_in(session(), &kept).unwrap();
    let second = flight(&store, &mut backend);
    assert_eq!(from_json(&second.records).unwrap().deeds.len(), 1);
}

#[test]
fn file_session_and_checkpoint_survive_reopen_and_replace() {
    let folder = std::env::temp_dir().join(format!(
        "werdy_account_{}",
        signin::random_hex::<16>().unwrap()
    ));
    let vault = Rc::new(storage::FileVault(folder.clone()));
    let inner = Rc::new(werdy_store::MemoryStore::default());
    let store = AccountStore::open(inner.clone(), vault.clone(), true);
    let mut records = records();
    store.sign_in(session(), &records).unwrap();
    records = store.load().unwrap().unwrap();
    records.settings.language = "en".into();
    store.save(&records).unwrap();
    let restored = AccountStore::open(inner, vault.clone(), true);
    assert!(restored.account.borrow().session.as_ref() == Some(&session()));
    assert_eq!(restored.load().unwrap().unwrap(), records);
    assert_eq!(
        restored
            .account
            .borrow()
            .checkpoint
            .as_ref()
            .unwrap()
            .install_id
            .len(),
        32
    );
    restored.sign_out(&records).unwrap();
    assert!(vault.read(File::Session).unwrap().is_none());
    // Cleanup does not decide the result: on Windows a file just written or
    // renamed can stay open for a moment (antivirus, the store), so removing
    // the folder sometimes fails there. A leftover temp folder is harmless (#139).
    let _ = std::fs::remove_dir_all(folder);
}

struct BrokenMirror;
impl Store for BrokenMirror {
    fn load(&self) -> Result<Option<OwnerData>, StoreError> {
        Ok(None)
    }
    fn save(&self, _records: &OwnerData) -> Result<(), StoreError> {
        Err(storage_error())
    }
}
#[test]
fn checkpoint_recovers_edits_when_owner_file_write_failed() {
    let vault = Rc::new(storage::MemoryVault::default());
    let store = AccountStore::open(Rc::new(BrokenMirror), vault.clone(), true);
    let mut records = records();
    assert!(store.save(&records).is_err());
    store.sign_in(session(), &records).unwrap();
    records = store.load().unwrap().unwrap();
    records.set_mark(DeedId(1), day(), Some(MarkEntry::new(Mark::Done, day())), 1);
    store.save(&records).unwrap();
    let reopened = AccountStore::open(Rc::new(BrokenMirror), vault, true);
    assert_eq!(reopened.load().unwrap().unwrap(), records);
}

#[test]
fn saving_again_after_a_mirror_failure_keeps_one_new_deed() {
    use crate::state::{EditorDraft, SaveStatus};
    use werdy_store::DeedName;

    let store = AccountStore::open(
        Rc::new(BrokenMirror),
        Rc::new(storage::MemoryVault::default()),
        true,
    );
    store.sign_in(session(), &records()).unwrap();
    let mut screen = AppState::start(&store, day(), vec![]);
    let draft = EditorDraft {
        name: "Reading".into(),
        ..EditorDraft::default()
    };
    screen.save_deed(None, &draft).unwrap();
    screen.save_to(&store);
    let first_save_status = screen.save_status;
    screen.save_to(&store);

    let checkpoint = restore(store.vault.as_ref()).unwrap().1.unwrap();
    let stored = from_json(&checkpoint.records).unwrap();
    let name = DeedName::Text(draft.name);
    assert_eq!(
        stored
            .names
            .values()
            .filter(|stored| **stored == name)
            .count(),
        1
    );
    assert_eq!(first_save_status, SaveStatus::Saved);
    assert_eq!(screen.save_status, SaveStatus::Saved);
}

#[test]
fn legacy_oversized_rows_survive_background_completion_and_restart() {
    use werdy_core::{DayNote, ExcuseCredit};
    let store = store();
    let mut records = records();
    records.set_day_note(
        day(),
        Some(DayNote::new("small", ExcuseCredit::None, day()).unwrap()),
        1,
    );
    store.sign_in(session(), &records).unwrap();
    records = store.load().unwrap().unwrap();
    let huge_note = "ن".repeat(4097);
    records.set_day_note(
        day(),
        Some(DayNote::new(&huge_note, ExcuseCredit::None, day()).unwrap()),
        2,
    );
    records
        .names
        .insert(DeedId(1), werdy_store::DeedName::Text("ن".repeat(513)));
    let mut checkpoint = store.account.borrow().checkpoint.clone().unwrap();
    let mut engine = serde_json::to_value(&checkpoint.engine).unwrap();
    // A checkpoint from before validation can contain these rows in both lists.
    for field in ["outbox", "observed"] {
        for row in engine[field].as_array_mut().unwrap() {
            let change = if field == "outbox" {
                &mut row["change"]
            } else {
                row
            };
            if change.get("Note").is_some() {
                change["Note"]["note"]["text"] = huge_note.clone().into();
            }
            if change.get("Deed").is_some() {
                change["Deed"]["row"]["name"] = serde_json::json!({"text":"ن".repeat(513)});
            }
        }
    }
    checkpoint.engine = serde_json::from_value(engine).unwrap();
    checkpoint.records = to_json(&records).unwrap();
    store.write_checkpoint(&checkpoint).unwrap();
    let restored = AccountStore::open(store.inner.clone(), store.vault.clone(), true);
    let mut backend = FakeBackend::new(1000);
    let completed = flight(&restored, &mut backend);
    let tomorrow = day().next().unwrap();
    records.set_day_note(
        tomorrow,
        Some(DayNote::new("during sync", ExcuseCredit::None, tomorrow).unwrap()),
        3,
    );
    restored.save(&records).unwrap();
    let generation = restored.account.borrow().generation;
    restored.complete(&completed, generation, 1000).unwrap();
    let reopened = AccountStore::open(store.inner.clone(), store.vault.clone(), true);
    let kept = reopened.load().unwrap().unwrap();
    assert_eq!(kept.day_notes, records.day_notes);
    assert_eq!(kept.names, records.names);
    assert_eq!(kept.deeds, records.deeds);
    assert_eq!(kept.note_revisions[&day()], records.note_revisions[&day()]);
    assert_eq!(
        reopened
            .account
            .borrow()
            .checkpoint
            .as_ref()
            .unwrap()
            .engine
            .local_only_errors()
            .len(),
        2
    );
    let completed = flight(&reopened, &mut backend);
    let generation = reopened.account.borrow().generation;
    reopened.complete(&completed, generation, 1000).unwrap();
    let mut fresh = OwnerData::default();
    run(Sync::new("fresh")
        .unwrap()
        .sync(&mut backend, &mut fresh, || 1000))
    .unwrap();
    assert!(fresh.deeds.is_empty());
    assert_eq!(fresh.day_notes[&tomorrow].text(), "during sync");
    assert!(!fresh.day_notes.contains_key(&day()));
}

#[test]
fn nonce_hashes_raw_hex_and_bridge_errors_do_not_expose_messages() {
    use sha2::{Digest, Sha256};
    let nonce = signin::Nonce::generate().unwrap();
    assert_eq!(nonce.raw.len(), 64);
    let expected: String = Sha256::digest(nonce.raw.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(nonce.hash, expected);
    assert!(matches!(
        serde_json::from_str::<signin::Reply>(r#"{"state":"error","message":"secret"}"#).unwrap(),
        signin::Reply::Error
    ));
    assert!(
        matches!(serde_json::from_str::<signin::Reply>(r#"{"state":"ok","idToken":"token"}"#).unwrap(),signin::Reply::Ok { id_token } if id_token == "token")
    );
}

#[test]
fn demo_uses_records_without_opening_or_writing_account_storage() {
    let store = AccountStore::open(
        Rc::new(werdy_store::MemoryStore::default()),
        Rc::new(storage::UnavailableVault),
        false,
    );
    let records = records();
    assert!(store.account.borrow().error.is_none());
    store.save(&records).unwrap();
    assert!(store.sign_in(session(), &records).is_err());
    assert_eq!(store.load().unwrap().unwrap(), records);
    assert!(store.account.borrow().session.is_none());
}

#[test]
fn confirmed_account_delete_signs_out_and_keeps_records_here() {
    let store = store();
    let records = records();
    store.sign_in(session(), &records).unwrap();
    let generation = store.account.borrow().generation;
    assert_eq!(store.settle_delete(Ok(()), &records, generation), Ok(()));
    assert!(store.account.borrow().session.is_none());
    assert!(store.vault.read(File::Session).unwrap().is_none());
    assert!(store.vault.read(File::Sync).unwrap().is_none());
    assert_eq!(store.load().unwrap().unwrap().deeds, records.deeds);
}

#[test]
fn failed_account_delete_keeps_session_sync_state_and_records() {
    let store = store();
    let records = records();
    store.sign_in(session(), &records).unwrap();
    let generation = store.account.borrow().generation;
    assert_eq!(
        store.settle_delete(Err(werdy_sync::SyncError::Http(500)), &records, generation),
        Err("account-delete-error")
    );
    assert!(store.account.borrow().session.is_some());
    assert!(store.vault.read(File::Session).unwrap().is_some());
    assert!(store.vault.read(File::Sync).unwrap().is_some());
    assert_eq!(store.load().unwrap().unwrap().deeds, records.deeds);
}

#[test]
fn late_delete_reply_does_not_sign_out_a_newer_session() {
    let store = store();
    let records = records();
    store.sign_in(session(), &records).unwrap();
    let old = store.account.borrow().generation;
    store.sign_out(&records).unwrap();
    store.sign_in(session(), &records).unwrap();
    assert_eq!(store.settle_delete(Ok(()), &records, old), Ok(()));
    assert!(store.account.borrow().session.is_some());
}

#[test]
fn reused_operation_shows_sync_error() {
    assert_eq!(
        runtime::error_key(&werdy_sync::SyncError::OperationReused),
        "account-sync-error"
    );
}

#[test]
fn collision_recovery_saves_new_install_and_edits_made_during_push() {
    let store = store();
    store.sign_in(session(), &records()).unwrap();
    let mut backend = FakeBackend::new(1000);
    let mut failed = store.account.borrow().checkpoint.clone().unwrap();
    let mut local = from_json(&failed.records).unwrap();
    let mut other = failed.engine.clone();
    let mut remote = local.clone();
    remote.settings.theme = werdy_store::Theme::Dark;
    local.settings.language = "en".into();
    store.save(&local).unwrap();
    failed = store.account.borrow().checkpoint.clone().unwrap();
    run(other.sync(&mut backend, &mut remote, || 1000)).unwrap();
    let Err(werdy_sync::SyncError::Collision(collision)) =
        run(failed.engine.sync(&mut backend, &mut local, || 1000))
    else {
        panic!("expected an operation collision");
    };
    let mut edited = store.load().unwrap().unwrap();
    edited.set_mark(DeedId(1), day(), Some(MarkEntry::new(Mark::Done, day())), 1);
    store.save(&edited).unwrap();
    assert!(store.recover_collision(&collision, 0).unwrap().is_none());
    assert_eq!(
        store
            .account
            .borrow()
            .checkpoint
            .as_ref()
            .unwrap()
            .install_id,
        failed.install_id
    );
    let merged = store.recover_collision(&collision, 1).unwrap().unwrap();
    assert_eq!(merged.settings.theme, remote.settings.theme);
    assert_eq!(merged.settings.language, "en");
    assert_eq!(merged.marks, edited.marks);
    let reopened = AccountStore::open(store.inner.clone(), store.vault.clone(), true);
    let saved = reopened.account.borrow().checkpoint.clone().unwrap();
    assert_ne!(saved.install_id, failed.install_id);
    assert_eq!(saved.last_sync, None);
    assert_eq!(saved.engine.outbox().len(), 2);
    assert!(
        saved
            .engine
            .outbox()
            .iter()
            .all(|p| p.install == saved.install_id)
    );
    assert_eq!(saved.records, to_json(&merged).unwrap());
    let completed = flight(&reopened, &mut backend);
    assert!(completed.engine.outbox().is_empty());
}

/// Two copies of the web app on one device: a browser tab and the home
/// screen app share one storage.
fn two_tabs() -> (AccountStore, AccountStore) {
    let inner: Rc<dyn Store> = Rc::new(werdy_store::MemoryStore::default());
    let vault: Rc<dyn Vault> = Rc::new(storage::MemoryVault::default());
    let first = AccountStore::open(inner.clone(), vault.clone(), true);
    first.sign_in(session(), &records()).unwrap();
    let second = AccountStore::open(inner, vault, true);
    (first, second)
}

fn sync_tab(tab: &AccountStore, backend: &mut FakeBackend, now: u64) -> OwnerData {
    let (_, mut checkpoint, generation) = tab.claim_flight(now).unwrap().unwrap();
    let mut records = from_json(&checkpoint.records).unwrap();
    run(checkpoint.engine.sync(backend, &mut records, || 1000)).unwrap();
    checkpoint.records = to_json(&records).unwrap();
    let merged = tab.complete(&checkpoint, generation, now).unwrap().unwrap();
    tab.release_flight();
    merged
}

#[test]
fn sharing_uses_the_session_another_tab_refreshed_without_overwriting_it() {
    let (first, second) = two_tabs();
    let (mut rotated, _, generation) = first.claim_flight(1_000).unwrap().unwrap();
    rotated.refresh_token = "rotated".into();
    runtime::save_refresh(&first, &rotated, generation).unwrap();

    let used = second.current_session(0).unwrap();
    let stored = restore(first.vault.as_ref()).unwrap().0.unwrap();
    assert_eq!(stored.refresh_token, rotated.refresh_token);
    assert_eq!(used.refresh_token, rotated.refresh_token);
}

#[test]
fn a_tab_open_before_sign_in_saves_edits_to_the_checkpoint() {
    for load_before_save in [true, false] {
        let first = store();
        let second = AccountStore::open(first.inner.clone(), first.vault.clone(), true);
        assert!(!runtime::view(&second).signed_in);
        first.sign_in(session(), &records()).unwrap();

        let mut edited = if load_before_save {
            second.load().unwrap().unwrap()
        } else {
            records()
        };
        let key = (DeedId(1), day());
        edited.set_mark(key.0, key.1, Some(MarkEntry::new(Mark::Done, day())), 1);
        second.save(&edited).unwrap();

        let checkpoint = restore(first.vault.as_ref()).unwrap().1.unwrap();
        assert_eq!(from_json(&checkpoint.records).unwrap().marks, edited.marks);
        let reopened = AccountStore::open(first.inner.clone(), first.vault.clone(), true);
        assert_eq!(reopened.load().unwrap().unwrap().marks, edited.marks);
        assert!(runtime::view(&second).signed_in);
    }
}

#[test]
fn stale_screen_keeps_a_server_clear_with_an_older_revision() {
    let (first, second) = two_tabs();
    let key = (DeedId(1), day());
    let mut marked = first.load().unwrap().unwrap();
    marked.set_mark(key.0, key.1, Some(MarkEntry::new(Mark::Done, day())), 5000);
    first.save(&marked).unwrap();
    let mut stale = AppState::start(&second, day(), vec![]);
    assert_eq!(stale.data.mark_revisions[&key], 5000);

    let mut backend = FakeBackend::new(1000);
    let mut cleared = sync_tab(&first, &mut backend, 1000);
    cleared.set_mark(key.0, key.1, None, 6000);
    first.save(&cleared).unwrap();
    backend.set_time(2000);
    let accepted = sync_tab(&first, &mut backend, 2000);
    assert_eq!(accepted.cleared_marks[&key], 2000);
    let stored = first.vault.read(File::Sync).unwrap();

    stale.reload_from(&second);

    assert_eq!(stale.data.marks.get(&key), None);
    assert_eq!(stale.data.cleared_marks[&key], 2000);
    assert_eq!(first.vault.read(File::Sync).unwrap(), stored);
}

#[test]
fn an_unsaved_mark_wins_over_an_older_stored_clear() {
    let (first, second) = two_tabs();
    let key = (DeedId(1), day());
    let mut screen = AppState::start(&second, day(), vec![]);
    screen
        .data
        .set_mark(key.0, key.1, Some(MarkEntry::new(Mark::Done, day())), 5000);
    let mut cleared = first.load().unwrap().unwrap();
    cleared.set_mark(key.0, key.1, None, 2000);
    first.save(&cleared).unwrap();

    screen.reload_from(&second);

    let saved = first.load().unwrap().unwrap();
    assert_eq!(screen.data.marks[&key].mark, Mark::Done);
    assert_eq!(saved.marks[&key].mark, Mark::Done);
    assert_eq!(saved.mark_revisions[&key], 5000);
}

#[test]
fn two_tabs_sharing_storage_never_reuse_an_operation_id() {
    let (first, second) = two_tabs();
    let mut edited = first.load().unwrap().unwrap();
    edited.settings.theme = werdy_store::Theme::Dark;
    first.save(&edited).unwrap();
    let mut other = second.load().unwrap().unwrap();
    assert_eq!(other.settings.theme, werdy_store::Theme::Dark);
    other.set_mark(DeedId(1), day(), Some(MarkEntry::new(Mark::Done, day())), 1);
    second.save(&other).unwrap();
    let mut later = first.load().unwrap().unwrap();
    assert_eq!(later.marks, other.marks);
    later.settings.language = "en".into();
    first.save(&later).unwrap();

    let saved = AccountStore::open(first.inner.clone(), first.vault.clone(), true);
    let outbox = saved.account.borrow().checkpoint.clone().unwrap();
    let operations: std::collections::BTreeSet<u64> =
        outbox.engine.outbox().iter().map(|p| p.operation).collect();
    assert_eq!(operations.len(), outbox.engine.outbox().len());

    let mut backend = FakeBackend::new(1000);
    sync_tab(&second, &mut backend, 10_000);
    let mut again = first.load().unwrap().unwrap();
    again.settings.language = "ar".into();
    first.save(&again).unwrap();
    sync_tab(&first, &mut backend, 20_000);

    let fresh = store();
    fresh.sign_in(session(), &OwnerData::default()).unwrap();
    let pulled = sync_tab(&fresh, &mut backend, 30_000);
    assert_eq!(pulled.settings.theme, werdy_store::Theme::Dark);
    assert_eq!(pulled.settings.language, "ar");
    assert_eq!(pulled.marks, other.marks);
}

#[test]
fn only_one_tab_runs_sync_until_it_releases_or_its_lease_runs_out() {
    let (first, second) = two_tabs();
    assert!(first.claim_flight(1_000).unwrap().is_some());
    assert!(second.claim_flight(2_000).unwrap().is_none());
    first.release_flight();
    assert!(second.claim_flight(3_000).unwrap().is_some());
    assert!(first.claim_flight(4_000).unwrap().is_none());
    // A tab closed mid sync never releases; its lease runs out.
    assert!(first.claim_flight(3_000 + LEASE_MS).unwrap().is_some());
    // A clock moved backwards cannot keep a lease alive for years.
    assert!(second.claim_flight(1_000).unwrap().is_some());
}

#[test]
fn a_flight_uses_the_session_another_tab_refreshed() {
    let (first, second) = two_tabs();
    let mut rotated = session();
    rotated.refresh_token = "rotated".into();
    runtime::save_refresh(&first, &rotated, 1).unwrap();
    let (session, _, _) = second.claim_flight(1_000).unwrap().unwrap();
    assert_eq!(session.refresh_token, "rotated");
}

#[derive(Default)]
struct SessionWriteFailingVault {
    inner: storage::MemoryVault,
    fail_next_session_write: std::cell::Cell<bool>,
}

impl Vault for SessionWriteFailingVault {
    fn read(&self, file: File) -> Result<Option<String>, StoreError> {
        self.inner.read(file)
    }

    fn write(&self, file: File, text: &str) -> Result<(), StoreError> {
        if file == File::Session && self.fail_next_session_write.replace(false) {
            return Err(storage_error());
        }
        self.inner.write(file, text)
    }

    fn remove(&self, file: File) -> Result<(), StoreError> {
        self.inner.remove(file)
    }
}

#[test]
fn next_flight_retries_saving_a_rotated_refresh_token() {
    let vault = Rc::new(SessionWriteFailingVault::default());
    let store = AccountStore::open(
        Rc::new(werdy_store::MemoryStore::default()),
        vault.clone(),
        true,
    );
    store.sign_in(session(), &records()).unwrap();
    let (mut rotated, _, generation) = store.claim_flight(1_000).unwrap().unwrap();
    rotated.refresh_token = "rotated".into();
    vault.fail_next_session_write.set(true);
    assert!(runtime::save_refresh(&store, &rotated, generation).is_err());
    store.release_flight();
    assert_eq!(
        restore(vault.as_ref()).unwrap().0.unwrap().refresh_token,
        session().refresh_token
    );

    let (retry, _, generation) = store.claim_flight(2_000).unwrap().unwrap();
    assert_eq!(retry.refresh_token, rotated.refresh_token);
    store.current_session(generation).unwrap();
    assert_eq!(
        restore(vault.as_ref()).unwrap().0.unwrap().refresh_token,
        rotated.refresh_token
    );
    store.release_flight();

    let other = AccountStore::open(store.inner.clone(), vault.clone(), true);
    rotated.refresh_token = "rotated again".into();
    runtime::save_refresh(&other, &rotated, 0).unwrap();
    let (latest, _, _) = store.claim_flight(3_000).unwrap().unwrap();
    assert_eq!(latest.refresh_token, rotated.refresh_token);
}

#[test]
fn a_late_sync_in_one_tab_keeps_an_edit_saved_in_the_other() {
    let (first, second) = two_tabs();
    let mut backend = FakeBackend::new(1000);
    let (_, mut checkpoint, generation) = first.claim_flight(1_000).unwrap().unwrap();
    let mut edited = second.load().unwrap().unwrap();
    edited.set_mark(DeedId(1), day(), Some(MarkEntry::new(Mark::Done, day())), 1);
    second.save(&edited).unwrap();
    let mut records = from_json(&checkpoint.records).unwrap();
    run(checkpoint.engine.sync(&mut backend, &mut records, || 1000)).unwrap();
    checkpoint.records = to_json(&records).unwrap();
    let merged = first
        .complete(&checkpoint, generation, 2_000)
        .unwrap()
        .unwrap();
    assert_eq!(merged.marks, edited.marks);
    assert_eq!(second.load().unwrap().unwrap().marks, edited.marks);
}

#[test]
fn sync_completion_with_a_stale_screen_keeps_the_other_tabs_mark() {
    let (first, second) = two_tabs();
    let mut screen = crate::state::AppState::start(&first, day(), vec![]);
    let mut backend = FakeBackend::new(1000);
    let (_, _, generation) = first.claim_flight(1_000).unwrap().unwrap();
    let outcome = runtime::Attempt::Synced(Box::new(flight(&first, &mut backend)));

    let mut edited = second.load().unwrap().unwrap();
    edited.set_mark(DeedId(1), day(), Some(MarkEntry::new(Mark::Done, day())), 1);
    second.save(&edited).unwrap();

    runtime::finish_attempt(&first, &mut screen, &outcome, generation)
        .unwrap()
        .unwrap();
    assert_eq!(second.load().unwrap().unwrap().marks, edited.marks);
    let account = first.account.borrow();
    let checkpoint = account.checkpoint.as_ref().unwrap();
    assert!(
        !checkpoint
            .engine
            .outbox()
            .iter()
            .any(|pending| matches!(pending.change, werdy_sync::Change::Mark { entry: None, .. }))
    );
}

#[test]
fn sync_completion_saves_an_unsaved_screen_edit_made_during_the_request() {
    let store = store();
    store.sign_in(session(), &records()).unwrap();
    let mut screen = crate::state::AppState::start(&store, day(), vec![]);
    let mut backend = FakeBackend::new(1000);
    let (_, _, generation) = store.claim_flight(1_000).unwrap().unwrap();
    let outcome = runtime::Attempt::Synced(Box::new(flight(&store, &mut backend)));

    screen.tap(DeedId(1), crate::state::Tap::Done);
    assert!(store.load().unwrap().unwrap().marks.is_empty());

    let merged = runtime::finish_attempt(&store, &mut screen, &outcome, generation)
        .unwrap()
        .unwrap();
    assert_eq!(merged.marks[&(DeedId(1), day())].mark, Mark::Done);
    assert_eq!(store.load().unwrap().unwrap().marks, merged.marks);
}

#[test]
fn a_mark_cleared_during_a_sync_stays_cleared_when_a_stale_tab_reloads() {
    let (first, second) = two_tabs();
    let mut backend = FakeBackend::new(1000);
    let mut marked = first.load().unwrap().unwrap();
    marked.set_mark(DeedId(1), day(), Some(MarkEntry::new(Mark::Done, day())), 1);
    first.save(&marked).unwrap();
    sync_tab(&first, &mut backend, 1_000);
    let mut stale = crate::state::AppState::start(&second, day(), vec![]);
    assert_eq!(stale.data.marks[&(DeedId(1), day())].mark, Mark::Done);

    backend.set_time(2000);
    let (_, _, generation) = first.claim_flight(2_000).unwrap().unwrap();
    let completed = flight(&first, &mut backend);
    let mut cleared = first.load().unwrap().unwrap();
    cleared.set_mark(DeedId(1), day(), None, 3000);
    first.save(&cleared).unwrap();
    first
        .complete(&completed, generation, 2_000)
        .unwrap()
        .unwrap();
    first.release_flight();

    stale.reload_from(&second);
    assert!(!stale.data.marks.contains_key(&(DeedId(1), day())));
    assert!(
        !second
            .load()
            .unwrap()
            .unwrap()
            .marks
            .contains_key(&(DeedId(1), day()))
    );
    let account = first.account.borrow();
    let checkpoint = account.checkpoint.as_ref().unwrap();
    assert!(!checkpoint.engine.outbox().iter().any(|pending| matches!(
        pending.change,
        werdy_sync::Change::Mark { entry: Some(_), .. }
    )));
}

#[test]
fn a_sign_out_in_another_tab_stops_the_next_sync_from_restoring_the_account() {
    let inner: Rc<dyn Store> = Rc::new(werdy_store::MemoryStore::default());
    let vault: Rc<dyn Vault> = Rc::new(storage::MemoryVault::default());
    let first = AccountStore::open(inner.clone(), vault.clone(), true);
    first.sign_in(session(), &records()).unwrap();
    let second = AccountStore::open(inner.clone(), vault.clone(), true);

    second.sign_out(&records()).unwrap();

    assert!(first.claim_flight(1_000).unwrap().is_none());
    assert!(first.account.borrow().session.is_none());
    let reopened = AccountStore::open(inner, vault, true);
    assert!(reopened.account.borrow().session.is_none());
}

#[test]
fn a_sync_running_when_another_tab_signs_out_does_not_write_the_account_back() {
    let (first, second) = two_tabs();
    let mut backend = FakeBackend::new(1000);
    let (_, _, generation) = first.claim_flight(1_000).unwrap().unwrap();
    let completed = flight(&first, &mut backend);

    second.sign_out(&records()).unwrap();

    assert!(
        first
            .complete(&completed, generation, 1_000)
            .unwrap()
            .is_none()
    );
    assert!(first.vault.read(File::Sync).unwrap().is_none());
    assert!(first.vault.read(File::Session).unwrap().is_none());
}

#[test]
fn a_refresh_after_another_tab_signs_out_does_not_restore_the_session() {
    let (first, second) = two_tabs();
    let (mut rotated, _, generation) = first.claim_flight(1_000).unwrap().unwrap();
    rotated.refresh_token = "rotated".into();

    second.sign_out(&records()).unwrap();

    let refresh = runtime::save_refresh(&first, &rotated, generation);
    assert!(first.vault.read(File::Session).unwrap().is_none());
    assert!(refresh.is_err());
    assert!(first.account.borrow().session.is_none());
}

#[test]
fn a_save_after_another_tab_signs_out_does_not_restore_the_checkpoint() {
    let (first, second) = two_tabs();
    let mut stale = first.load().unwrap().unwrap();
    stale.settings.language = "ar".into();
    second.sign_out(&records()).unwrap();

    let save = first.save(&stale);

    assert!(first.vault.read(File::Sync).unwrap().is_none());
    assert!(save.is_err());
    assert_eq!(
        first.inner.load().unwrap().unwrap().settings.language,
        records().settings.language
    );
}
