use std::collections::BTreeSet;

use fluent_syntax::{ast, parser};
use werdy_data::{Error, Schedule, builtin, load, parse};

/// A small valid file. Tests break one part of it at a time.
const SMALL: &str = r#"
slots = ["fajr", "night"]
groups = ["dua"]
sheet_order = ["fajr", "night", "dua"]
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
            let place = deed.place();
            assert!(
                catalog
                    .slots
                    .iter()
                    .chain(&catalog.groups)
                    .any(|key| key == place),
                "{}",
                deed.key
            );
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
fn every_academy_level_offers_three_sets_in_order() {
    let catalog = builtin().expect("the shipped starter sets load");
    for set in catalog.level_sets() {
        let offers = &set.level.as_ref().expect("level details").offers;
        assert_eq!(
            offers
                .iter()
                .map(|offer| offer.set.as_str())
                .collect::<Vec<_>>(),
            ["five_prayers", "fasting_seasons", "ramadan"]
        );
        assert_eq!(
            offers.iter().map(|offer| offer.ticked).collect::<Vec<_>>(),
            [true, false, false]
        );
    }
}

#[test]
fn invalid_academy_offers_are_rejected() {
    let text = include_str!("../../../data/starter-sets.toml");
    for (replacement, expected) in [
        (
            "missing_set",
            Error::UnknownOfferedSet {
                level: "level_1".into(),
                set: "missing_set".into(),
            },
        ),
        (
            "levels_2_3",
            Error::LevelOfferedAsExtra {
                level: "level_1".into(),
                set: "levels_2_3".into(),
            },
        ),
    ] {
        let changed = text.replacen(
            "{ set = \"five_prayers\"",
            &format!("{{ set = \"{replacement}\""),
            1,
        );
        assert_ne!(changed, text);
        assert_eq!(parse(&changed), Err(expected));
    }
    let duplicate = text.replacen("{ set = \"fasting_seasons\"", "{ set = \"five_prayers\"", 1);
    assert_ne!(duplicate, text);
    assert_eq!(
        parse(&duplicate),
        Err(Error::DuplicateOffer {
            level: "level_1".into(),
            set: "five_prayers".into()
        })
    );
}

#[test]
fn academy_offer_why_keys_exist_in_both_locales() {
    let text = include_str!("../../../data/starter-sets.toml");
    let ar = include_str!("../../../locales/ar/starter-sets.ftl");
    let en = include_str!("../../../locales/en/starter-sets.ftl");
    let changed = text.replacen("level-offer-why-five-prayers", "missing-offer-why", 1);
    assert_ne!(changed, text);
    assert_eq!(
        load(&changed, ar, en),
        Err(Error::MissingName {
            locale: "ar",
            name: "missing-offer-why".into()
        })
    );
    let ar_with_key = format!("{ar}\nmissing-offer-why = Example\n");
    assert_eq!(
        load(&changed, &ar_with_key, en),
        Err(Error::MissingName {
            locale: "en",
            name: "missing-offer-why".into()
        })
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
    let slots: Vec<&str> = deeds.iter().map(|deed| deed.place()).collect();
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
    // The advanced sheet has 113 deeds. Levels 2 and 3 add istikhara and
    // salat al hajah to the sheet's 61, at the owner's request.
    for (key, count) in [("level_1", 44), ("levels_2_3", 63), ("level_advanced", 113)] {
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
fn advanced_weekly_box_is_due_once_a_week() {
    let catalog = builtin().expect("the shipped starter sets load");
    let advanced = catalog.set("level_advanced").expect("set exists");
    let weekly = catalog
        .deeds_in(advanced)
        .into_iter()
        .filter(|deed| deed.schedule == Schedule::OnceAWeek)
        .map(|deed| deed.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(weekly, ["lv_visiting_graves", "lv_weekly_fitra"]);

    // Levels 1 to 3 keep sunan al fitra in the Friday box.
    for key in ["level_1", "levels_2_3"] {
        let set = catalog.set(key).expect("set exists");
        assert!(
            set.deeds.iter().any(|deed| deed == "lv_friday_fitra"),
            "{key}"
        );
        assert!(
            !set.deeds.iter().any(|deed| deed == "lv_weekly_fitra"),
            "{key}"
        );
    }
}

#[test]
fn deeds_for_a_need_are_due_when_they_come_up() {
    let catalog = builtin().expect("the shipped starter sets load");
    let mut comes_up = catalog
        .deeds
        .iter()
        .filter(|deed| deed.schedule == Schedule::WhenItComesUp)
        .map(|deed| deed.key.as_str())
        .collect::<Vec<_>>();
    comes_up.sort_unstable();
    assert_eq!(
        comes_up,
        [
            "lv_asr_makeup",
            "lv_dhuhr_makeup",
            "lv_fajr_makeup",
            "lv_fasting",
            "lv_hajah",
            "lv_isha_makeup",
            "lv_istikhara",
            "lv_maghrib_makeup",
        ]
    );

    // Situational adhkar happen almost every day, so they stay daily.
    for key in [
        "lv_riding_adhkar",
        "lv_mosque_enter_leave_adhkar",
        "lv_food_adhkar",
    ] {
        let deed = catalog.deed(key).expect("deed exists");
        assert_eq!(deed.schedule, Schedule::EveryDay, "{key}");
    }
}

#[test]
fn once_a_week_is_read_from_the_file() {
    let weekly = SMALL.replace(
        "schedule = { hijri = \"ramadan\" }",
        "schedule = \"once_a_week\"",
    );
    let catalog = parse(&weekly).expect("once a week parses");
    assert_eq!(
        catalog.deed("qiyam").map(|deed| &deed.schedule),
        Some(&Schedule::OnceAWeek)
    );
}

#[test]
fn when_it_comes_up_is_read_from_the_file() {
    let comes_up = SMALL.replace(
        "schedule = { hijri = \"ramadan\" }",
        "schedule = \"when_it_comes_up\"",
    );
    let catalog = parse(&comes_up).expect("when it comes up parses");
    assert_eq!(
        catalog.deed("qiyam").map(|deed| &deed.schedule),
        Some(&Schedule::WhenItComesUp)
    );
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

/// SMALL with one more deed that sits in the dua group.
fn small_with_group_deed(place: &str) -> String {
    SMALL.replace(
        "[[sets]]",
        &format!(
            "[[deeds]]
key = \"dua_self\"
name = \"deed-dua-self\"
{place}
category = \"adhkar\"
schedule = \"every_day\"

[[sets]]"
        ),
    )
}

#[test]
fn a_deed_sits_in_one_place_a_slot_or_a_group() {
    let catalog = parse(&small_with_group_deed("group = \"dua\"")).expect("a group deed parses");
    let deed = catalog.deed("dua_self").expect("deed exists");
    assert_eq!(deed.place(), "dua");
    assert_eq!(
        catalog.deed("qiyam").map(|deed| deed.place()),
        Some("night")
    );

    assert_eq!(
        parse(&small_with_group_deed("group = \"quran\"")),
        Err(Error::UnknownGroup {
            deed: "dua_self".into(),
            group: "quran".into()
        })
    );
    assert_eq!(
        parse(&small_with_group_deed(
            "slot = \"night\"
group = \"dua\""
        )),
        Err(Error::NotOnePlace {
            deed: "dua_self".into()
        })
    );
    assert_eq!(
        parse(&small_with_group_deed("")),
        Err(Error::NotOnePlace {
            deed: "dua_self".into()
        })
    );
}

#[test]
fn a_group_key_may_not_repeat_a_slot_key() {
    let clash = SMALL
        .replace("groups = [\"dua\"]", "groups = [\"night\"]")
        .replace("\"night\", \"dua\"]", "\"night\", \"night\"]");
    assert_eq!(
        parse(&clash),
        Err(Error::DuplicateKey {
            kind: "place",
            key: "night".into()
        })
    );
}

#[test]
fn the_sheet_order_lists_every_slot_and_group_once() {
    let catalog = parse(SMALL).expect("small file parses");
    assert_eq!(catalog.sheet_order, ["fajr", "night", "dua"]);
    for (order, key) in [
        ("[\"fajr\", \"dua\"]", "night"),
        ("[\"fajr\", \"night\", \"dua\", \"dawn\"]", "dawn"),
        ("[\"fajr\", \"night\", \"dua\", \"fajr\"]", "fajr"),
    ] {
        let changed = SMALL.replace("[\"fajr\", \"night\", \"dua\"]", order);
        assert_eq!(
            parse(&changed),
            Err(Error::SheetOrder { key: key.into() }),
            "{order}"
        );
    }
}

#[test]
fn the_sheet_shows_slots_then_the_six_groups_then_any_time() {
    let catalog = builtin().expect("the shipped starter sets load");
    assert_eq!(
        catalog.groups,
        [
            "situational_adhkar",
            "quran",
            "dua",
            "heart_and_character",
            "knowledge",
            "self_review"
        ]
    );
    assert_eq!(
        catalog.sheet_order,
        [
            "fajr",
            "morning",
            "dhuhr",
            "asr",
            "maghrib",
            "isha",
            "night",
            "situational_adhkar",
            "quran",
            "dua",
            "heart_and_character",
            "knowledge",
            "self_review",
            "any_time"
        ]
    );
}

/// The level deeds that decision 0045 puts in each group.
const LEVEL_GROUPS: [(&str, &[&str]); 6] = [
    (
        "situational_adhkar",
        &[
            "lv_waking_adhkar",
            "lv_toilet_adhkar",
            "lv_dressing_adhkar",
            "lv_wudu_adhkar",
            "lv_home_adhkar",
            "lv_mosque_adhkar",
            "lv_mosque_enter_leave_adhkar",
            "lv_walking_to_mosque_adhkar",
            "lv_food_adhkar",
            "lv_riding_adhkar",
            "lv_sleep_adhkar",
            "lv_istighfar_100",
            "lv_gathering_istighfar",
            "lv_gathering_kaffara",
        ],
    ),
    (
        "quran",
        &[
            "lv_memorise_half_page",
            "lv_read_six_quarters",
            "lv_memorise_page",
            "lv_read_juz",
            "lv_recite_reflect",
            "lv_review_memorise",
        ],
    ),
    (
        "dua",
        &[
            "lv_dua_self",
            "lv_dua_teachers",
            "lv_dua_brothers",
            "lv_dua_muslims",
        ],
    ),
    (
        "heart_and_character",
        &[
            "lv_tongue",
            "lv_gaze",
            "lv_heart",
            "lv_parents",
            "lv_kinship",
            "lv_rights",
            "lv_dawah",
        ],
    ),
    (
        "knowledge",
        &[
            "lv_attend_lessons",
            "lv_review_lessons",
            "lv_worldly_knowledge",
            "lv_attend_sharia",
            "lv_review_sharia",
        ],
    ),
    ("self_review", &["lv_self_review", "lv_repentance"]),
];

#[test]
fn level_deeds_sit_in_the_groups_of_decision_0045() {
    let catalog = builtin().expect("the shipped starter sets load");
    for (group, keys) in LEVEL_GROUPS {
        for key in keys {
            assert_eq!(catalog.group_of(key), Some(group), "{key}");
        }
    }
    let grouped: BTreeSet<&str> = LEVEL_GROUPS
        .iter()
        .flat_map(|(_, keys)| keys.iter().copied())
        .collect();
    let any_time: BTreeSet<&str> = catalog
        .deeds
        .iter()
        .filter(|deed| deed.key.starts_with("lv_") && deed.place() == "any_time")
        .map(|deed| deed.key.as_str())
        .collect();
    assert_eq!(
        any_time,
        BTreeSet::from([
            "lv_fasting_monday_thursday",
            "lv_fasting",
            "lv_general_istighfar",
            "lv_salawat",
            "lv_charity",
            "lv_istikhara",
            "lv_hajah",
            "lv_keep_wudu",
            "lv_wudu_before_sleep",
            "lv_study_or_work",
            "lv_friday_kahf",
            "lv_friday_salawat",
            "lv_friday_salawat_100",
            // The advanced weekly box came with 0047, after 0045 named its
            // groups, so it stays in any time.
            "lv_visiting_graves",
            "lv_weekly_fitra",
        ])
    );
    let in_groups = catalog
        .deeds
        .iter()
        .filter(|deed| catalog.group_of(&deed.key).is_some())
        .count();
    assert_eq!(in_groups, grouped.len());
}

#[test]
fn group_of_is_none_for_slot_deeds_and_unknown_keys() {
    let catalog = builtin().expect("the shipped starter sets load");
    assert_eq!(catalog.group_of("lv_general_istighfar"), None);
    assert_eq!(catalog.group_of("lv_witr"), None);
    assert_eq!(catalog.group_of("no_such_deed"), None);
}

/// The unit picker offers every shipped unit, hours included (#175), and
/// each one has a picker label in both locales.
#[test]
fn every_unit_has_a_picker_label_and_hours_is_offered() {
    let catalog = builtin().expect("the shipped starter sets load");
    let keys: Vec<_> = catalog.units.iter().map(|unit| unit.key.as_str()).collect();
    assert_eq!(keys, ["pages", "rakaat", "times", "hours"]);
    for (locale, source) in [
        ("ar", include_str!("../../../locales/ar/deeds.ftl")),
        ("en", include_str!("../../../locales/en/deeds.ftl")),
    ] {
        for key in &keys {
            let label = format!("\neditor-unit-{key} = ");
            assert!(
                source.contains(&label),
                "{locale}: missing editor-unit-{key}"
            );
        }
    }
}
