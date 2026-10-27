//! UI text from Fluent files (ADR 0025). Code asks for keys, never for text.

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

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
];
const ENGLISH: &[&str] = &[
    include_str!("../../../locales/en/common.ftl"),
    include_str!("../../../locales/en/sheet.ftl"),
    include_str!("../../../locales/en/starter.ftl"),
];

/// All translations, loaded once at start.
pub struct Texts {
    arabic: FluentBundle<FluentResource>,
    english: FluentBundle<FluentResource>,
}

impl Texts {
    /// Loads every Fluent file. A file with a syntax error still loads the
    /// messages that parsed; a test makes sure there are no such errors.
    pub fn load() -> Self {
        Self {
            arabic: bundle(Language::Arabic, ARABIC),
            english: bundle(Language::English, ENGLISH),
        }
    }

    /// Returns the text for a key. A missing key shows the key itself, so a
    /// gap is visible and never a crash.
    pub fn get(&self, language: Language, key: &str, args: &[(&str, FluentValue<'_>)]) -> String {
        let bundle = match language {
            Language::Arabic => &self.arabic,
            Language::English => &self.english,
        };
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

fn bundle(language: Language, sources: &[&str]) -> FluentBundle<FluentResource> {
    let locale: Vec<LanguageIdentifier> = language.code().parse().into_iter().collect();
    let mut bundle = FluentBundle::new(locale);
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
}
