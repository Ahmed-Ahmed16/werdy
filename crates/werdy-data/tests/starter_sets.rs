use std::collections::BTreeSet;

use werdy_data::{Error, Schedule, builtin, load, parse};

/// A small valid file. Tests break one part of it at a time.
const SMALL: &str = r#"
slots = ["fajr", "night"]
categories = ["prayer", "adhkar"]
hijri_ranges = ["ramadan"]

[[units]]
key = "rakaat"
name = "unit-rakaat"

[[deeds]]
key = "fajr_on_time"
name = "deed-fajr-on-time"
slot = "fajr"
category = "prayer"
schedule = "every_day"

[[deeds]]
key = "qiyam"
name = "deed-qiyam"
slot = "night"
category = "prayer"
schedule = { hijri = "ramadan" }
target = { amount = 4, unit = "rakaat" }

[[sets]]
key = "basic"
name = "set-basic"
deeds = ["fajr_on_time", "qiyam"]
"#;

const SMALL_FTL: &str = "\
# Names for the small file.
set-basic = Basic
deed-fajr-on-time = Fajr on time
deed-qiyam = Qiyam
unit-rakaat =
    { $count ->
        [one] rakah
       *[other] rakaat
    }
";

#[test]
fn every_set_uses_only_known_keys() {
    let catalog = builtin().expect("the shipped starter sets load");
    for set in &catalog.sets {
        for deed in catalog.deeds_in(set) {
            assert!(catalog.slots.contains(&deed.slot), "{}", deed.key);
            assert!(catalog.categories.contains(&deed.category), "{}", deed.key);
            if let Schedule::Hijri(range) = &deed.schedule {
                assert!(catalog.hijri_ranges.contains(range), "{}", deed.key);
            }
        }
    }

    let bad_slot = SMALL.replace("slot = \"fajr\"", "slot = \"dawn\"");
    assert_eq!(
        parse(&bad_slot),
        Err(Error::UnknownSlot {
            deed: "fajr_on_time".into(),
            slot: "dawn".into()
        })
    );
    let bad_range = SMALL.replace("hijri = \"ramadan\"", "hijri = \"shaban\"");
    assert_eq!(
        parse(&bad_range),
        Err(Error::UnknownHijriRange {
            deed: "qiyam".into(),
            range: "shaban".into()
        })
    );
    let bad_category = SMALL.replace("category = \"prayer\"", "category = \"sport\"");
    assert!(matches!(
        parse(&bad_category),
        Err(Error::UnknownCategory { .. })
    ));
}

#[test]
fn deed_keys_are_unique() {
    let catalog = builtin().expect("the shipped starter sets load");
    let keys: BTreeSet<&str> = catalog.deeds.iter().map(|deed| deed.key.as_str()).collect();
    assert_eq!(keys.len(), catalog.deeds.len());

    let copy = SMALL.replace("key = \"qiyam\"", "key = \"fajr_on_time\"");
    assert_eq!(
        parse(&copy),
        Err(Error::DuplicateKey {
            kind: "deed",
            key: "fajr_on_time".into()
        })
    );
}

#[test]
fn overlapping_sets_share_one_deed() {
    let catalog = builtin().expect("the shipped starter sets load");
    let in_set = |set_key: &str| {
        let set = catalog.set(set_key).expect("set exists");
        set.deeds.contains(&"sleep_adhkar".to_owned())
    };
    assert!(in_set("morning_evening"));
    assert!(in_set("daily_adhkar"));
    let copies = catalog
        .deeds
        .iter()
        .filter(|deed| deed.key == "sleep_adhkar")
        .count();
    assert_eq!(copies, 1);
}

#[test]
fn every_name_key_exists_in_ar_and_en() {
    builtin().expect("the shipped starter sets load");
    assert!(load(SMALL, SMALL_FTL, SMALL_FTL).is_ok());

    let en_without_qiyam = SMALL_FTL.replace("deed-qiyam = Qiyam\n", "");
    assert_eq!(
        load(SMALL, SMALL_FTL, &en_without_qiyam),
        Err(Error::MissingName {
            locale: "en",
            name: "deed-qiyam".into()
        })
    );
}

#[test]
fn five_prayers_set_has_five_deeds_one_per_prayer_slot() {
    let catalog = builtin().expect("the shipped starter sets load");
    let set = catalog.set("five_prayers").expect("set exists");
    let deeds = catalog.deeds_in(set);
    assert_eq!(deeds.len(), 5);
    let slots: Vec<&str> = deeds.iter().map(|deed| deed.slot.as_str()).collect();
    assert_eq!(slots, ["fajr", "dhuhr", "asr", "maghrib", "isha"]);
    for deed in deeds {
        assert_eq!(deed.category, "prayer");
        assert_eq!(deed.schedule, Schedule::EveryDay);
    }
}

#[test]
fn loading_a_broken_file_is_an_error() {
    assert!(matches!(parse("slots = [\"fajr\""), Err(Error::Parse(_))));
    let typo = SMALL.replace("schedule = \"every_day\"", "shedule = \"every_day\"");
    assert!(matches!(parse(&typo), Err(Error::Parse(_))));
    let unknown_deed = SMALL.replace("\"qiyam\"]", "\"witr\"]");
    assert_eq!(
        parse(&unknown_deed),
        Err(Error::UnknownDeedInSet {
            set: "basic".into(),
            deed: "witr".into()
        })
    );
    let zero = SMALL.replace("amount = 4", "amount = 0");
    assert_eq!(
        parse(&zero),
        Err(Error::ZeroTarget {
            deed: "qiyam".into()
        })
    );
}
