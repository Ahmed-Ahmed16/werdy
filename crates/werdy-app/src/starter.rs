//! Pure catalog conversion and set selection.

use std::collections::BTreeSet;

use werdy_core::{CategoryKey, DeedRule, Schedule, SlotKey, Target, Weekday};
use werdy_data::{Catalog, Deed as CatalogDeed, Schedule as CatalogSchedule};
use werdy_store::OwnerData;

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

pub fn slot_order() -> Vec<SlotKey> {
    crate::editor_data::catalog().map_or_else(
        |_| Vec::new(),
        |catalog| catalog.slots.iter().map(|key| SlotKey::new(key)).collect(),
    )
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
        slot: SlotKey::new(&deed.slot),
    })
}

#[cfg(test)]
mod tests {
    use super::{missing_deeds, rule};
    use std::collections::BTreeSet;
    use werdy_store::OwnerData;

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
}
