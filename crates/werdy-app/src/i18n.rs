//! UI text from Fluent files (ADR 0025). Code asks for keys, never for text.

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use std::collections::BTreeSet;
use unic_langid::LanguageIdentifier;
use werdy_core::{HijriRange, Schedule, Weekday};
use werdy_store::Digits;

/// Rewrites decimal glyphs in displayed text. Stored numbers stay unchanged.
pub fn format_digits(text: &str, digits: Digits) -> String {
    text.chars()
        .map(|ch| {
            let digit_value = match ch {
                '0'..='9' => Some(u32::from(ch) - u32::from('0')),
                '٠'..='٩' => Some(u32::from(ch) - u32::from('٠')),
                '۰'..='۹' => Some(u32::from(ch) - u32::from('۰')),
                _ => None,
            };
            digit_value
                .and_then(|digit| {
                    char::from_u32(match digits {
                        Digits::Western => u32::from('0') + digit,
                        Digits::ArabicIndic => u32::from('٠') + digit,
                    })
                })
                .unwrap_or(ch)
        })
        .collect()
}

/// Interface language. Arabic is the default (ADR 0014).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Language {
    /// Arabic, right to left.
    #[default]
    Arabic,
    /// English, left to right.
    English,
}

impl Language {
    /// Reads a stored language code. Unknown codes fall back to Arabic.
    pub fn from_code(code: &str) -> Self {
        match code {
            "en" => Self::English,
            _ => Self::Arabic,
        }
    }

    /// Returns the code stored in settings and used for the `lang` attribute.
    pub fn code(self) -> &'static str {
        match self {
            Self::Arabic => "ar",
            Self::English => "en",
        }
    }

    /// Returns the text direction for the `dir` attribute.
    pub fn dir(self) -> &'static str {
        match self {
            Self::Arabic => "rtl",
            Self::English => "ltr",
        }
    }

    /// Returns the other language, for the language switch.
    pub fn other(self) -> Self {
        match self {
            Self::Arabic => Self::English,
            Self::English => Self::Arabic,
        }
    }
}

/// Fluent files for one language, one per feature.
const ARABIC: &[&str] = &[
    include_str!("../../../locales/ar/common.ftl"),
    include_str!("../../../locales/ar/sheet.ftl"),
    include_str!("../../../locales/ar/starter.ftl"),
    include_str!("../../../locales/ar/reports.ftl"),
    include_str!("../../../locales/ar/deeds.ftl"),
    include_str!("../../../locales/ar/starter-sets.ftl"),
    include_str!("../../../locales/ar/first-run.ftl"),
    include_str!("../../../locales/ar/settings.ftl"),
];
const ENGLISH: &[&str] = &[
    include_str!("../../../locales/en/common.ftl"),
    include_str!("../../../locales/en/sheet.ftl"),
    include_str!("../../../locales/en/starter.ftl"),
    include_str!("../../../locales/en/reports.ftl"),
    include_str!("../../../locales/en/deeds.ftl"),
    include_str!("../../../locales/en/starter-sets.ftl"),
    include_str!("../../../locales/en/first-run.ftl"),
    include_str!("../../../locales/en/settings.ftl"),
];

/// All translations, loaded once at start.
pub struct Texts {
    arabic: FluentBundle<FluentResource>,
    english: FluentBundle<FluentResource>,
    arabic_indic: FluentBundle<FluentResource>,
    english_indic: FluentBundle<FluentResource>,
}

/// Reads a schedule back as one sentence in the selected language.
pub fn schedule_sentence(texts: &Texts, language: Language, schedule: &Schedule) -> String {
    match schedule {
        Schedule::EveryDay => texts.get(language, "schedule-every-day-sentence", &[]),
        Schedule::Weekdays(days) => weekdays_sentence(texts, language, days),
        Schedule::Hijri(range) => hijri_sentence(texts, language, range),
    }
}

pub const WEEKDAY_ORDER: [Weekday; 7] = [
    Weekday::Saturday,
    Weekday::Sunday,
    Weekday::Monday,
    Weekday::Tuesday,
    Weekday::Wednesday,
    Weekday::Thursday,
    Weekday::Friday,
];

fn weekdays_sentence(texts: &Texts, language: Language, days: &BTreeSet<Weekday>) -> String {
    if days.is_empty() {
        return texts.get(language, "schedule-no-weekdays", &[]);
    }
    let names = WEEKDAY_ORDER
        .into_iter()
        .filter(|day| days.contains(day))
        .map(|day| texts.get(language, weekday_key(day), &[]))
        .collect::<Vec<_>>();
    let names = join_weekday_names(texts, language, &names);
    texts.get(
        language,
        "schedule-weekdays-sentence",
        &[("days", FluentValue::from(names))],
    )
}

fn join_weekday_names(texts: &Texts, language: Language, names: &[String]) -> String {
    let Some((last, first)) = names.split_last() else {
        return String::new();
    };
    if first.is_empty() {
        return last.clone();
    }
    let separator = if first.len() == 1 {
        "schedule-pair-separator"
    } else {
        "schedule-final-separator"
    };
    format!(
        "{}{}{}",
        first.join(&texts.get(language, "schedule-list-separator", &[])),
        texts.get(language, separator, &[]),
        last
    )
}

fn hijri_sentence(texts: &Texts, language: Language, range: &HijriRange) -> String {
    let choice = crate::editor_data::ranges()
        .ok()
        .into_iter()
        .flatten()
        .find(|choice| choice.range().ok().as_ref() == Some(range));
    let Some(choice) = choice else {
        return texts.get(language, "schedule-hijri-unavailable", &[]);
    };
    let name = texts.get(
        language,
        &format!("range-{}", choice.key.replace('_', "-")),
        &[],
    );
    texts.get(
        language,
        "schedule-hijri-sentence",
        &[("range", FluentValue::from(name))],
    )
}

pub fn weekday_key(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => "weekday-monday",
        Weekday::Tuesday => "weekday-tuesday",
        Weekday::Wednesday => "weekday-wednesday",
        Weekday::Thursday => "weekday-thursday",
        Weekday::Friday => "weekday-friday",
        Weekday::Saturday => "weekday-saturday",
        Weekday::Sunday => "weekday-sunday",
    }
}

impl Texts {
    /// Loads every Fluent file. A file with a syntax error still loads the
    /// messages that parsed; a test makes sure there are no such errors.
    pub fn load() -> Self {
        Self {
            arabic: bundle(Language::Arabic, ARABIC, Digits::Western),
            english: bundle(Language::English, ENGLISH, Digits::Western),
            arabic_indic: bundle(Language::Arabic, ARABIC, Digits::ArabicIndic),
            english_indic: bundle(Language::English, ENGLISH, Digits::ArabicIndic),
        }
    }

    /// Returns the text for a key. A missing key shows the key itself, so a
    /// gap is visible and never a crash.
    pub fn get(&self, language: Language, key: &str, args: &[(&str, FluentValue<'_>)]) -> String {
        let bundle = match language {
            Language::Arabic => &self.arabic,
            Language::English => &self.english,
        };
        Self::format(bundle, key, args)
    }

    /// Formats number arguments with chosen digits while preserving text arguments.
    pub fn get_with_digits(
        &self,
        language: Language,
        digits: Digits,
        key: &str,
        args: &[(&str, FluentValue<'_>)],
    ) -> String {
        let bundle = match (language, digits) {
            (Language::Arabic, Digits::Western) => &self.arabic,
            (Language::English, Digits::Western) => &self.english,
            (Language::Arabic, Digits::ArabicIndic) => &self.arabic_indic,
            (Language::English, Digits::ArabicIndic) => &self.english_indic,
        };
        Self::format(bundle, key, args)
    }

    fn format(
        bundle: &FluentBundle<FluentResource>,
        key: &str,
        args: &[(&str, FluentValue<'_>)],
    ) -> String {
        let Some(pattern) = bundle.get_message(key).and_then(|message| message.value()) else {
            return key.to_owned();
        };
        let mut fluent_args = FluentArgs::new();
        for (name, value) in args {
            fluent_args.set(*name, value.clone());
        }
        let mut errors = Vec::new();
        bundle
            .format_pattern(pattern, Some(&fluent_args), &mut errors)
            .into_owned()
    }
}

fn bundle(language: Language, sources: &[&str], digits: Digits) -> FluentBundle<FluentResource> {
    let locale: Vec<LanguageIdentifier> = language.code().parse().into_iter().collect();
    let mut bundle = FluentBundle::new(locale);
    if digits == Digits::ArabicIndic {
        bundle.set_formatter(Some(|value, _| match value {
            FluentValue::Number(number) => {
                Some(format_digits(&number.as_string(), Digits::ArabicIndic))
            }
            _ => None,
        }));
    }
    for source in sources {
        let resource = match FluentResource::try_new((*source).to_owned()) {
            Ok(resource) => resource,
            Err((partial, _errors)) => partial,
        };
        bundle.add_resource_overriding(resource);
    }
    bundle
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use fluent_bundle::{FluentResource, FluentValue};

    use super::{ARABIC, ENGLISH, Language, Texts};
    use werdy_store::Digits;

    #[test]
    fn digits_convert_ascii_fluent_and_existing_arabic_glyphs() {
        assert_eq!(
            super::format_digits("2026: ٣ of ۵ ✓", Digits::Western),
            "2026: 3 of 5 ✓"
        );
        assert_eq!(
            super::format_digits("2026: ٣ of ۵ ✓", Digits::ArabicIndic),
            "٢٠٢٦: ٣ of ٥ ✓"
        );
        let fluent = Texts::load().get(
            Language::Arabic,
            "count-progress",
            &[
                ("count", FluentValue::from(3)),
                ("target", FluentValue::from(5)),
            ],
        );
        assert_eq!(
            super::format_digits(&fluent, Digits::Western).replace(['\u{2068}', '\u{2069}'], ""),
            "3 من 5"
        );
        assert_eq!(
            super::format_digits(&fluent, Digits::ArabicIndic)
                .replace(['\u{2068}', '\u{2069}'], ""),
            "٣ من ٥"
        );
    }

    fn message_ids(source: &str) -> Result<BTreeSet<String>, String> {
        let resource = FluentResource::try_new(source.to_owned())
            .map_err(|(_, errors)| format!("{errors:?}"))?;
        Ok(resource
            .entries()
            .filter_map(|entry| match entry {
                fluent_syntax::ast::Entry::Message(message) => Some(message.id.name.to_owned()),
                _ => None,
            })
            .collect())
    }

    #[test]
    fn every_arabic_key_exists_in_english_and_the_reverse() -> Result<(), String> {
        assert_eq!(ARABIC.len(), ENGLISH.len());
        for (arabic, english) in ARABIC.iter().zip(ENGLISH) {
            let arabic_ids = message_ids(arabic)?;
            let english_ids = message_ids(english)?;
            let only_arabic: Vec<_> = arabic_ids.difference(&english_ids).collect();
            let only_english: Vec<_> = english_ids.difference(&arabic_ids).collect();
            assert!(
                only_arabic.is_empty(),
                "missing in English: {only_arabic:?}"
            );
            assert!(
                only_english.is_empty(),
                "missing in Arabic: {only_english:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn fluent_copy_has_no_streak_or_badge_terms() {
        for source in ARABIC.iter().chain(ENGLISH) {
            let lower = source.to_lowercase();
            for forbidden in ["streak", "badge", "سلسلة", "شارة"] {
                assert!(!lower.contains(forbidden), "found {forbidden}");
            }
        }
    }

    #[test]
    fn variables_are_filled_in() {
        let texts = Texts::load();

        let text = texts.get(
            Language::English,
            "count-progress",
            &[
                ("count", FluentValue::from(3)),
                ("target", FluentValue::from(5)),
            ],
        );

        assert_eq!(text.replace(['\u{2068}', '\u{2069}'], ""), "3 of 5");
    }

    #[test]
    fn a_missing_key_shows_the_key() {
        assert_eq!(
            Texts::load().get(Language::Arabic, "no-such-key", &[]),
            "no-such-key"
        );
    }

    #[test]
    fn arabic_is_the_default_and_reads_right_to_left() {
        assert_eq!(Language::default().dir(), "rtl");
        assert_eq!(Language::from_code("xx"), Language::Arabic);
        assert_eq!(Language::from_code("en").dir(), "ltr");
    }

    #[test]
    fn owner_reference_hijri_month_exists_in_both_languages() {
        let texts = Texts::load();
        assert_eq!(
            texts.get(Language::English, "hijri-month-5", &[]),
            "Jumada al Ula"
        );
        assert_eq!(
            texts.get(Language::Arabic, "hijri-month-5", &[]),
            "جمادى الأولى"
        );
    }

    #[test]
    fn report_lead_uses_deed_name_in_both_languages() {
        let texts = Texts::load();
        let english = texts.get(
            Language::English,
            "reports-lead-missed",
            &[("deed", FluentValue::from("Witr"))],
        );
        let arabic = texts.get(
            Language::Arabic,
            "reports-lead-missed",
            &[("deed", FluentValue::from("الوتر"))],
        );
        assert_eq!(
            english.replace(['\u{2068}', '\u{2069}'], ""),
            "Most missed: Witr."
        );
        assert_eq!(
            arabic.replace(['\u{2068}', '\u{2069}'], ""),
            "أكثر ما فاتك: الوتر."
        );
    }

    #[test]
    fn schedule_sentences_read_back_daily_weekdays_and_hijri_in_both_languages() {
        use werdy_core::{HijriRange, Schedule, Weekday};
        let texts = Texts::load();
        let cases = [
            (Schedule::EveryDay, "Due every day.", "مستحق كل يوم."),
            (
                Schedule::Weekdays(BTreeSet::from([Weekday::Monday, Weekday::Thursday])),
                "Due every Monday and Thursday.",
                "مستحق كل الاثنين والخميس.",
            ),
            (
                Schedule::Weekdays(BTreeSet::from([Weekday::Saturday])),
                "Due every Saturday.",
                "مستحق كل السبت.",
            ),
            (
                Schedule::Weekdays(BTreeSet::from([
                    Weekday::Saturday,
                    Weekday::Monday,
                    Weekday::Thursday,
                ])),
                "Due every Saturday, Monday, and Thursday.",
                "مستحق كل السبت، الاثنين، والخميس.",
            ),
            (
                Schedule::Hijri(HijriRange::ramadan()),
                "Due during Ramadan.",
                "مستحق خلال رمضان.",
            ),
        ];
        for (schedule, english, arabic) in cases {
            assert_eq!(
                super::schedule_sentence(&texts, Language::English, &schedule)
                    .replace(['\u{2068}', '\u{2069}'], ""),
                english
            );
            assert_eq!(
                super::schedule_sentence(&texts, Language::Arabic, &schedule)
                    .replace(['\u{2068}', '\u{2069}'], ""),
                arabic
            );
        }
    }
}
