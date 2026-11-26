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
    first.write_session(&rotated).unwrap();
    let (session, _, _) = second.claim_flight(1_000).unwrap().unwrap();
    assert_eq!(session.refresh_token, "rotated");
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
