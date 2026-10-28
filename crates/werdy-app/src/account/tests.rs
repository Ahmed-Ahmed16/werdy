#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};
use werdy_core::{
    CategoryKey, Deed, DeedId, DeedRule, Mark, MarkEntry, Schedule as DeedSchedule, SlotKey,
    WerdyDay,
};
use werdy_sync::FakeBackend;

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
fn flight(store: &AccountStore, backend: &mut FakeBackend) -> Checkpoint {
    let mut checkpoint = store.account.borrow().checkpoint.clone().unwrap();
    let mut records = from_json(&checkpoint.records).unwrap();
    run(checkpoint.engine.sync(backend, &mut records, || 1000)).unwrap();
    checkpoint.records = to_json(&records).unwrap();
    checkpoint
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
    std::fs::remove_dir(folder).unwrap();
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
    store.sign_in(session(), &records).unwrap();
    records = store.load().unwrap().unwrap();
    records.set_mark(DeedId(1), day(), Some(MarkEntry::new(Mark::Done, day())), 1);
    assert!(store.save(&records).is_err());
    let reopened = AccountStore::open(Rc::new(BrokenMirror), vault, true);
    assert_eq!(reopened.load().unwrap().unwrap(), records);
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
