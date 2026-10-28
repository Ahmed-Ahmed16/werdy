use werdy_core::{CategoryKey, Deed, DeedId, DeedRule, Schedule, SlotKey, WerdyDay};
use werdy_store::{DeedKey, OwnerData, from_json, merge_records, to_json};

fn records() -> Result<OwnerData, Box<dyn std::error::Error>> {
    let mut records = OwnerData::default();
    for id in 1..=2 {
        records.deeds.push(Deed::new(
            DeedId(id),
            WerdyDay::new(2026, 10, 28)?,
            DeedRule {
                schedule: Schedule::EveryDay,
                target: None,
                category: CategoryKey::new("prayer"),
                slot: SlotKey::new("fajr"),
            },
        ));
        records.deed_keys.insert(
            DeedId(id),
            DeedKey {
                install: "phone".into(),
                sequence: id,
            },
        );
    }
    records.last_deed_id = 2;
    Ok(records)
}

#[test]
fn identity_round_trip_and_deleted_identity_survive_stale_merge()
-> Result<(), Box<dyn std::error::Error>> {
    let stored = records()?;
    assert_eq!(from_json(&to_json(&stored)?)?, stored);
    let mut local = stored.clone();
    local.deeds.retain(|deed| deed.id != DeedId(1));
    local.deleted_deeds.insert(DeedId(1));
    let merged = merge_records(&stored, &local)?;
    assert_eq!(merged.deed_keys, stored.deed_keys);
    assert_eq!(from_json(&to_json(&merged)?)?, merged);
    let mut legacy: serde_json::Value = serde_json::from_str(&to_json(&stored)?)?;
    legacy["schema_version"] = 11.into();
    if let Some(object) = legacy.as_object_mut() {
        object.remove("deed_keys");
    }
    assert!(from_json(&legacy.to_string())?.deed_keys.is_empty());
    Ok(())
}

#[test]
fn malformed_and_duplicate_identities_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let original: serde_json::Value = serde_json::from_str(&to_json(&records()?)?)?;
    for invalid in [serde_json::json!(0), serde_json::json!("")] {
        let mut edited = original.clone();
        if invalid.is_number() {
            edited["deed_keys"][0]["key"]["sequence"] = invalid;
        } else {
            edited["deed_keys"][0]["key"]["install"] = invalid;
        }
        assert!(from_json(&edited.to_string()).is_err());
    }
    let mut duplicate = original.clone();
    duplicate["deed_keys"][1]["key"] = duplicate["deed_keys"][0]["key"].clone();
    assert!(from_json(&duplicate.to_string()).is_err());
    let mut orphan = original;
    orphan["deed_keys"][0]["deed_id"] = 999.into();
    assert!(from_json(&orphan.to_string()).is_err());
    Ok(())
}

#[test]
fn merging_a_local_deed_absent_from_stored_does_not_leave_an_orphan_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let local = records()?;
    let stored = OwnerData::default();
    let merged = merge_records(&stored, &local)?;
    assert_eq!(from_json(&to_json(&merged)?)?, merged);
    Ok(())
}

#[test]
fn merging_two_valid_copies_with_conflicting_identities_returns_an_error()
-> Result<(), Box<dyn std::error::Error>> {
    let stored = records()?;
    let mut local = stored.clone();
    local.deed_keys.remove(&DeedId(1));
    local
        .deed_keys
        .insert(DeedId(2), stored.deed_keys[&DeedId(1)].clone());
    assert_eq!(from_json(&to_json(&local)?)?, local);
    assert!(merge_records(&stored, &local).is_err());
    // Different numeric ids can denote the same remote deed after independent imports.
    let mut local = OwnerData::default();
    let mut deed = stored.deeds[0].clone();
    deed.id = DeedId(3);
    local.deeds.push(deed.clone());
    local
        .deed_keys
        .insert(deed.id, stored.deed_keys[&DeedId(1)].clone());
    let mut target = stored.clone();
    target.deeds.push(deed);
    assert!(merge_records(&target, &local).is_err());
    assert_eq!(from_json(&to_json(&stored)?)?, stored);
    Ok(())
}
