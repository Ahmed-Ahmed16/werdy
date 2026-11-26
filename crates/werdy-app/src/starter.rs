//! Pure catalog conversion and set selection.

use std::collections::BTreeSet;

use werdy_core::{CategoryKey, DeedId, DeedRule, Schedule, SlotKey, Target, Weekday};
use werdy_data::{Catalog, Deed as CatalogDeed, Schedule as CatalogSchedule};
use werdy_store::{DeedName, OwnerData};

/// Recognise prayer deeds from saved sheets made before set identities existed.
pub fn migrate_legacy_prayers(data: &mut OwnerData) -> bool {
    let Ok(catalog) = crate::editor_data::catalog() else {
        return false;
    };
    let Some(set) = catalog.set("five_prayers") else {
        return false;
    };
    let mut changed = false;
    for deed in &data.deeds {
        if data.starter_keys.contains_key(&deed.id) {
            continue;
        }
        let Some(DeedName::Key(name)) = data.names.get(&deed.id) else {
            continue;
        };
        if let Some(key) = set.deeds.iter().find(|key| {
            catalog.deed(key).is_some_and(|catalog_deed| {
                catalog_deed
                    .name
                    .strip_prefix("deed-")
                    .is_some_and(|stem| name == &format!("starter-{stem}"))
            })
        }) {
            data.starter_keys.insert(deed.id, key.clone());
            changed = true;
        }
    }
    if changed
        && set
            .deeds
            .iter()
            .all(|key| data.starter_keys.values().any(|stored| stored == key))
    {
        data.chosen_sets.insert("five_prayers".to_owned());
    }
    changed
}

#[cfg(test)]
pub fn five_prayers(today: werdy_core::WerdyDay) -> Result<OwnerData, werdy_data::Error> {
    let catalog = werdy_data::builtin()?;
    let mut data = OwnerData::default();
    if let Some(set) = catalog.set("five_prayers") {
        for key in &set.deeds {
            if let Some(deed) = catalog
                .deed(key)
                .and_then(|deed| rule(deed).map(|rule| (deed, rule)))
            {
                let id = data.next_deed_id();
                data.last_deed_id = id.0;
                data.deeds.push(werdy_core::Deed::new(id, today, deed.1));
                data.names
                    .insert(id, werdy_store::DeedName::Key(deed.0.name.clone()));
                data.starter_keys.insert(id, key.clone());
            }
        }
    }
    data.first_run_complete = true;
    data.chosen_sets.insert("five_prayers".to_owned());
    Ok(data)
}

/// The owner's five fard prayer deeds, in the five prayers set order.
/// A prayer deed is one added from that starter set (decision 0041).
pub fn fard_prayer_ids(data: &OwnerData) -> Vec<DeedId> {
    let Some(set) = crate::editor_data::catalog()
        .ok()
        .and_then(|catalog| catalog.set("five_prayers"))
    else {
        return Vec::new();
    };
    set.deeds
        .iter()
        .filter_map(|key| {
            data.starter_keys
                .iter()
                .find(|(_, stored)| *stored == key)
                .map(|(id, _)| *id)
        })
        .collect()
}

/// Places in sheet order: time slots, then groups, then any time (0045).
/// A deed's place is stored in its rule's slot key.
pub fn slot_order() -> Vec<SlotKey> {
    crate::editor_data::catalog().map_or_else(
        |_| Vec::new(),
        |catalog| {
            catalog
                .sheet_order
                .iter()
                .map(|key| SlotKey::new(key))
                .collect()
        },
    )
}

/// Fluent key that names a place: `group-<key>` for a group, else `slot-<key>`.
pub fn place_name_key(place: &str) -> String {
    let group = crate::editor_data::catalog()
        .is_ok_and(|catalog| catalog.groups.iter().any(|key| key == place));
    if group {
        format!("group-{place}")
    } else {
        format!("slot-{place}")
    }
}

/// Moves each level deed that has always sat in any time to its group
/// (decision 0045), as a rule change from today; past days keep any time
/// (0020). The saved starter key (0041) finds the group. A deed whose rules
/// ever named another place was placed by the owner and is left alone, so
/// running this again changes nothing. Returns the moved deeds.
pub fn move_level_deeds_to_groups(
    data: &mut OwnerData,
    catalog: &Catalog,
    today: werdy_core::WerdyDay,
) -> Vec<DeedId> {
    let mut moved = Vec::new();
    for deed in &mut data.deeds {
        let Some(group) = data
            .starter_keys
            .get(&deed.id)
            .and_then(|key| catalog.group_of(key))
        else {
            continue;
        };
        let placed_by_owner = deed
            .rule_versions()
            .any(|(_, rule)| rule.slot.0 != "any_time");
        let Some(mut rule) = deed.rule_on(today).cloned().filter(|_| !placed_by_owner) else {
            continue;
        };
        rule.slot = SlotKey::new(group);
        if deed.change_rule(today, rule).is_ok() {
            moved.push(deed.id);
        }
    }
    moved
}

/// Keys a set would add to this sheet. Shared deeds appear once.
pub fn missing_deeds(
    catalog: &Catalog,
    data: &OwnerData,
    selected: &BTreeSet<String>,
) -> Vec<String> {
    let mut seen: BTreeSet<_> = data.starter_keys.values().cloned().collect();
    catalog
        .sets
        .iter()
        .filter(|set| selected.contains(&set.key))
        .flat_map(|set| &set.deeds)
        .filter(|key| seen.insert((*key).clone()))
        .cloned()
        .collect()
}

pub fn selected_deeds(catalog: &Catalog, selected: &BTreeSet<String>) -> BTreeSet<String> {
    selected
        .iter()
        .filter_map(|key| catalog.set(key))
        .flat_map(|set| set.deeds.iter().cloned())
        .collect()
}

pub fn rule(deed: &CatalogDeed) -> Option<DeedRule> {
    let schedule = match &deed.schedule {
        CatalogSchedule::EveryDay => Schedule::EveryDay,
        CatalogSchedule::Weekdays(days) => Schedule::Weekdays(
            days.iter()
                .map(|day| match day {
                    werdy_data::Weekday::Monday => Weekday::Monday,
                    werdy_data::Weekday::Tuesday => Weekday::Tuesday,
                    werdy_data::Weekday::Wednesday => Weekday::Wednesday,
                    werdy_data::Weekday::Thursday => Weekday::Thursday,
                    werdy_data::Weekday::Friday => Weekday::Friday,
                    werdy_data::Weekday::Saturday => Weekday::Saturday,
                    werdy_data::Weekday::Sunday => Weekday::Sunday,
                })
                .collect(),
        ),
        CatalogSchedule::Hijri(key) => Schedule::Hijri(
            crate::editor_data::ranges()
                .ok()?
                .into_iter()
                .find(|range| range.key == *key)?
                .range()
                .ok()?,
        ),
    };
    let target = deed
        .target
        .as_ref()
        .map(|target| Target::new(target.amount, &target.unit))
        .transpose()
        .ok()?;
    Some(DeedRule {
        schedule,
        target,
        category: CategoryKey::new(&deed.category),
        slot: SlotKey::new(deed.place()),
    })
}

#[cfg(test)]
mod tests {
    use super::{fard_prayer_ids, missing_deeds, place_name_key, rule, slot_order};
    use std::collections::BTreeSet;
    use werdy_core::DeedId;
    use werdy_store::OwnerData;

    #[test]
    fn fard_prayers_follow_the_five_prayers_set_order() -> Result<(), Box<dyn std::error::Error>> {
        let mut data = OwnerData::default();
        for (id, key) in [
            (7, "isha_on_time"),
            (3, "sunnah_before_fajr"),
            (5, "fajr_on_time"),
            (9, "asr_on_time"),
        ] {
            data.starter_keys.insert(DeedId(id), key.to_owned());
        }
        assert_eq!(fard_prayer_ids(&data), [DeedId(5), DeedId(9), DeedId(7)]);
        Ok(())
    }

    #[test]
    fn a_sheet_without_starter_prayers_has_no_fard_prayers() {
        assert!(fard_prayer_ids(&OwnerData::default()).is_empty());
    }

    #[test]
    fn overlapping_sets_add_shared_deed_once() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = werdy_data::builtin()?;
        let a = catalog
            .sets
            .iter()
            .find(|a| {
                catalog
                    .sets
                    .iter()
                    .any(|b| a.key != b.key && a.deeds.iter().any(|key| b.deeds.contains(key)))
            })
            .ok_or("no overlap")?;
        let b = catalog
            .sets
            .iter()
            .find(|b| a.key != b.key && a.deeds.iter().any(|key| b.deeds.contains(key)))
            .ok_or("no second set")?;
        let selected = BTreeSet::from([a.key.clone(), b.key.clone()]);
        let keys = missing_deeds(&catalog, &OwnerData::default(), &selected);
        assert_eq!(keys.len(), keys.iter().collect::<BTreeSet<_>>().len());
        assert!(keys.len() < a.deeds.len() + b.deeds.len());
        assert_eq!(keys.first(), a.deeds.first());
        Ok(())
    }

    #[test]
    fn every_catalog_deed_has_a_core_rule() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = werdy_data::builtin()?;
        assert!(catalog.deeds.iter().all(|deed| rule(deed).is_some()));
        Ok(())
    }

    #[test]
    fn a_group_is_named_by_a_group_key_and_a_slot_by_a_slot_key() {
        assert_eq!(place_name_key("quran"), "group-quran");
        assert_eq!(place_name_key("self_review"), "group-self_review");
        assert_eq!(place_name_key("fajr"), "slot-fajr");
        assert_eq!(place_name_key("any_time"), "slot-any_time");
    }

    #[test]
    fn the_sheet_order_puts_groups_between_night_and_any_time() {
        let order: Vec<_> = slot_order().into_iter().map(|key| key.0).collect();
        let night = order.iter().position(|key| key == "night");
        let quran = order.iter().position(|key| key == "quran");
        let any_time = order.iter().position(|key| key == "any_time");
        assert!(night < quran && quran < any_time, "{order:?}");
        assert_eq!(order.last().map(String::as_str), Some("any_time"));
    }

    #[test]
    fn a_level_deed_keeps_its_group_as_its_place() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = werdy_data::builtin()?;
        let parents = catalog.deed("lv_parents").ok_or("missing deed")?;
        let rule = rule(parents).ok_or("no rule")?;
        assert_eq!(rule.slot.0, "heart_and_character");
        Ok(())
    }

    mod move_to_groups {
        use super::super::move_level_deeds_to_groups;
        use werdy_core::{CategoryKey, Deed, DeedId, DeedRule, Schedule, SlotKey, WerdyDay};
        use werdy_store::OwnerData;

        type TestResult = Result<(), Box<dyn std::error::Error>>;

        fn rule(place: &str) -> DeedRule {
            DeedRule {
                schedule: Schedule::EveryDay,
                target: None,
                category: CategoryKey::new("family"),
                slot: SlotKey::new(place),
            }
        }

        fn days() -> Result<(WerdyDay, WerdyDay), werdy_core::Error> {
            let today = WerdyDay::new(2026, 11, 26)?;
            Ok((today.previous()?.previous()?, today))
        }

        /// One deed from the parents level deed, added in any time before groups.
        fn owner(start: WerdyDay, key: Option<&str>) -> OwnerData {
            let mut data = OwnerData::default();
            data.deeds
                .push(Deed::new(DeedId(1), start, rule("any_time")));
            data.last_deed_id = 1;
            if let Some(key) = key {
                data.starter_keys.insert(DeedId(1), key.to_owned());
            }
            data
        }

        fn place_on(data: &OwnerData, day: WerdyDay) -> Option<String> {
            data.deeds[0].rule_on(day).map(|rule| rule.slot.0.clone())
        }

        #[test]
        fn a_level_deed_in_any_time_moves_to_its_group_from_today() -> TestResult {
            let catalog = werdy_data::builtin()?;
            let (start, today) = days()?;
            let mut data = owner(start, Some("lv_parents"));
            let moved = move_level_deeds_to_groups(&mut data, &catalog, today);
            assert_eq!(moved, [DeedId(1)]);
            assert_eq!(
                place_on(&data, today).as_deref(),
                Some("heart_and_character")
            );
            assert_eq!(
                place_on(&data, today.previous()?).as_deref(),
                Some("any_time")
            );
            assert_eq!(place_on(&data, start).as_deref(), Some("any_time"));
            Ok(())
        }

        #[test]
        fn running_the_move_again_adds_no_second_rule_version() -> TestResult {
            let catalog = werdy_data::builtin()?;
            let (start, today) = days()?;
            let mut data = owner(start, Some("lv_parents"));
            move_level_deeds_to_groups(&mut data, &catalog, today);
            let after_first = data.clone();
            assert!(move_level_deeds_to_groups(&mut data, &catalog, today).is_empty());
            assert!(move_level_deeds_to_groups(&mut data, &catalog, today.next()?).is_empty());
            assert_eq!(data, after_first);
            assert_eq!(data.deeds[0].rule_versions().count(), 2);
            Ok(())
        }

        #[test]
        fn a_deed_the_owner_moved_by_hand_is_left_alone() -> TestResult {
            let catalog = werdy_data::builtin()?;
            let (start, today) = days()?;
            let mut elsewhere = owner(start, Some("lv_parents"));
            elsewhere.deeds[0].change_rule(start.next()?, rule("night"))?;
            let mut back = elsewhere.clone();
            back.deeds[0].change_rule(today, rule("any_time"))?;
            for mut data in [elsewhere, back] {
                let before = data.clone();
                assert!(move_level_deeds_to_groups(&mut data, &catalog, today).is_empty());
                assert_eq!(data, before);
            }
            Ok(())
        }

        #[test]
        fn custom_deeds_and_level_deeds_without_a_group_stay_in_any_time() -> TestResult {
            let catalog = werdy_data::builtin()?;
            let (start, today) = days()?;
            for key in [None, Some("lv_general_istighfar"), Some("no_such_key")] {
                let mut data = owner(start, key);
                let before = data.clone();
                assert!(move_level_deeds_to_groups(&mut data, &catalog, today).is_empty());
                assert_eq!(data, before, "{key:?}");
            }
            Ok(())
        }
    }
}
