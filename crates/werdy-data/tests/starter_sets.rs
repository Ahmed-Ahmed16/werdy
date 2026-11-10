use std::collections::BTreeSet;

use fluent_syntax::{ast, parser};
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
fn academy_levels_load_in_card_order() {
    let catalog = builtin().expect("the shipped starter sets load");
    let levels = catalog.level_sets();
    assert_eq!(
        levels
            .iter()
            .map(|set| set.key.as_str())
            .collect::<Vec<_>>(),
        ["level_1", "levels_2_3", "level_advanced"]
    );
    assert_eq!(
        levels
            .iter()
            .filter_map(|set| set.level.as_ref().map(|level| level.order))
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
}

#[test]
fn duplicate_academy_level_order_is_rejected() {
    let text = include_str!("../../../data/starter-sets.toml");
    let duplicate = text.replacen(
        "level = { order = 2, title =",
        "level = { order = 1, title =",
        1,
    );
    assert_ne!(duplicate, text);
    assert_eq!(
        parse(&duplicate),
        Err(Error::DuplicateKey {
            kind: "level order",
            key: "1".into()
        })
    );
}

#[test]
fn unknown_academy_level_title_or_summary_is_rejected_in_both_locales() {
    let text = include_str!("../../../data/starter-sets.toml");
    let ar = include_str!("../../../locales/ar/starter-sets.ftl");
    let en = include_str!("../../../locales/en/starter-sets.ftl");
    for (old, missing) in [
        ("level-card-1-title", "unknown-level-title"),
        ("level-card-1-summary", "unknown-level-summary"),
    ] {
        let changed = text.replacen(old, missing, 1);
        assert_ne!(changed, text);
        assert_eq!(
            load(&changed, ar, en),
            Err(Error::MissingName {
                locale: "ar",
                name: missing.into()
            })
        );
        let ar_with_key = format!("{ar}\n{missing} = placeholder\n");
        assert_eq!(
            load(&changed, &ar_with_key, en),
            Err(Error::MissingName {
                locale: "en",
                name: missing.into()
            })
        );
    }
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
fn shipped_names_parse_with_values_for_every_key() {
    let catalog = builtin().expect("the shipped starter sets load");
    for (locale, source) in [
        ("ar", include_str!("../../../locales/ar/starter-sets.ftl")),
        ("en", include_str!("../../../locales/en/starter-sets.ftl")),
    ] {
        let resource = parser::parse(source)
            .unwrap_or_else(|(_, errors)| panic!("{locale}: invalid Fluent: {errors:?}"));
        let mut ids = BTreeSet::new();
        for entry in resource.body {
            if let ast::Entry::Message(message) = entry {
                assert!(
                    message
                        .value
                        .as_ref()
                        .is_some_and(|value| !value.elements.is_empty()),
                    "{locale}: {} has no value",
                    message.id.name
                );
                ids.insert(message.id.name);
            }
        }
        for name in catalog.name_keys() {
            assert!(ids.contains(name), "{locale}: missing {name}");
        }
    }
}

#[test]
fn empty_set_is_rejected() {
    let empty = SMALL.replace("deeds = [\"fajr_on_time\", \"qiyam\"]", "deeds = []");
    assert_eq!(
        parse(&empty),
        Err(Error::EmptySet {
            set: "basic".into()
        })
    );
}

#[test]
fn repeated_deed_in_set_is_rejected() {
    let repeated = SMALL.replace(
        "deeds = [\"fajr_on_time\", \"qiyam\"]",
        "deeds = [\"fajr_on_time\", \"fajr_on_time\"]",
    );
    assert_eq!(
        parse(&repeated),
        Err(Error::DeedTwiceInSet {
            set: "basic".into(),
            deed: "fajr_on_time".into()
        })
    );
}

#[test]
fn empty_weekday_schedule_is_rejected() {
    let no_days = SMALL.replace("schedule = \"every_day\"", "schedule = { weekdays = [] }");
    assert_eq!(
        parse(&no_days),
        Err(Error::NoWeekdays {
            deed: "fajr_on_time".into()
        })
    );
}

#[test]
fn unknown_target_unit_is_rejected() {
    let unknown = SMALL.replace("unit = \"rakaat\"", "unit = \"pages\"");
    assert_eq!(
        parse(&unknown),
        Err(Error::UnknownUnit {
            deed: "qiyam".into(),
            unit: "pages".into()
        })
    );
}

#[test]
fn repeated_name_key_is_rejected() {
    let repeated = SMALL.replace("name = \"deed-qiyam\"", "name = \"deed-fajr-on-time\"");
    assert_eq!(
        parse(&repeated),
        Err(Error::DuplicateName {
            name: "deed-fajr-on-time".into()
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

#[test]
fn level_sets_come_first_with_the_sheet_counts() {
    let catalog = builtin().expect("the shipped starter sets load");
    let keys: Vec<&str> = catalog.sets.iter().map(|set| set.key.as_str()).collect();
    assert_eq!(keys[..3], ["level_1", "levels_2_3", "level_advanced"]);
    // The advanced sheet has 113 deeds; visiting graves and sunan al fitra
    // wait for weekly schedules (#108). Levels 2 and 3 add istikhara and
    // salat al hajah to the sheet's 61, at the owner's request.
    for (key, count) in [("level_1", 44), ("levels_2_3", 63), ("level_advanced", 111)] {
        let set = catalog.set(key).expect("set exists");
        assert_eq!(catalog.deeds_in(set).len(), count, "{key}");
    }
}

#[test]
fn level_sets_follow_sheet_order() {
    let catalog = builtin().expect("the shipped starter sets load");
    let first = |key: &str, count: usize| -> Vec<String> {
        catalog.set(key).expect("set exists").deeds[..count].to_vec()
    };
    assert_eq!(
        first("level_1", 4),
        [
            "lv_fajr_sunnah_before",
            "lv_fajr_congregation",
            "lv_fajr_adhkar_after",
            "lv_morning_adhkar"
        ]
    );
    assert_eq!(
        first("level_advanced", 3),
        [
            "lv_fajr_early",
            "lv_fajr_adhan_adhkar",
            "lv_fajr_dua_between"
        ]
    );
}

#[test]
fn moving_up_a_level_keeps_shared_deeds() {
    let catalog = builtin().expect("the shipped starter sets load");
    let shared = [
        "lv_fajr_congregation",
        "lv_witr",
        "lv_parents",
        "lv_sleep_adhkar",
    ];
    for key in ["level_1", "levels_2_3", "level_advanced"] {
        let set = catalog.set(key).expect("set exists");
        for deed in shared {
            assert!(
                set.deeds.iter().any(|key| key == deed),
                "{key} lacks {deed}"
            );
        }
    }
}

#[test]
fn friday_box_deeds_are_due_only_on_fridays() {
    let catalog = builtin().expect("the shipped starter sets load");
    let friday = catalog
        .deeds
        .iter()
        .filter(|deed| deed.key.starts_with("lv_friday_"))
        .collect::<Vec<_>>();
    assert_eq!(friday.len(), 13);
    for deed in friday {
        assert_eq!(
            deed.schedule,
            Schedule::Weekdays(vec![werdy_data::Weekday::Friday]),
            "{}",
            deed.key
        );
    }
}

#[test]
fn only_level_sets_credit_the_instructors() {
    let catalog = builtin().expect("the shipped starter sets load");
    for set in &catalog.sets {
        let expected = set
            .key
            .starts_with("level")
            .then_some("set-credit-instructors");
        assert_eq!(set.credit.as_deref(), expected, "{}", set.key);
    }

    let credit_is_a_deed_name = SMALL.replace(
        "name = \"set-basic\"",
        "name = \"set-basic\"\ncredit = \"deed-qiyam\"",
    );
    assert_eq!(
        parse(&credit_is_a_deed_name),
        Err(Error::DuplicateName {
            name: "deed-qiyam".into()
        })
    );

    let credited = SMALL.replace(
        "name = \"set-basic\"",
        "name = \"set-basic\"\ncredit = \"set-credit\"",
    );
    assert_eq!(
        load(&credited, SMALL_FTL, SMALL_FTL),
        Err(Error::MissingName {
            locale: "ar",
            name: "set-credit".into()
        })
    );
}

#[test]
fn counted_level_deeds_carry_a_times_target() {
    let catalog = builtin().expect("the shipped starter sets load");
    for key in ["lv_tahlil_100", "lv_istighfar_100", "lv_friday_salawat_100"] {
        let target = catalog
            .deed(key)
            .and_then(|deed| deed.target.as_ref())
            .unwrap_or_else(|| panic!("{key} has no target"));
        assert_eq!(
            (target.amount, target.unit.as_str()),
            (100, "times"),
            "{key}"
        );
    }
}
