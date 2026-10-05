//! Supported UI languages and mapping from system locales.
//!
//! Language codes are also the folder names under `app/translations/`.

/// `(code, native name)` of every supported UI language. English is the source language.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("zh-TW", "繁體中文"),
    ("zh-CN", "简体中文"),
    ("es", "Español"),
    ("pt-BR", "Português (Brasil)"),
    ("ja", "日本語"),
    ("ko", "한국어"),
    ("fr", "Français"),
    ("de", "Deutsch"),
    ("ru", "Русский"),
];

/// `true` if `code` is one of [`LANGUAGES`].
pub fn is_supported(code: &str) -> bool {
    LANGUAGES.iter().any(|(c, _)| *c == code)
}

/// Index of `code` in [`LANGUAGES`] (0 = English when unknown).
pub fn index_of(code: &str) -> usize {
    LANGUAGES.iter().position(|(c, _)| *c == code).unwrap_or(0)
}

/// Maps a system locale (`zh-Hant-TW`, `zh_TW.UTF-8`, `pt-PT`, `es-419`, …) to a supported
/// language code. Unsupported languages fall back to English.
pub fn resolve(system_locale: Option<&str>) -> &'static str {
    let Some(raw) = system_locale else {
        return "en";
    };
    // Strip encoding / modifiers (`zh_TW.UTF-8@xyz`) and normalise separators.
    let base = raw.split(['.', '@']).next().unwrap_or(raw);
    let norm = base.replace('_', "-").to_ascii_lowercase();
    let parts: Vec<&str> = norm.split('-').collect();
    let lang = parts.first().copied().unwrap_or("");
    let has = |p: &str| parts.iter().skip(1).any(|x| *x == p);

    match lang {
        "zh" => {
            if has("hant") || has("tw") || has("hk") || has("mo") {
                "zh-TW"
            } else {
                "zh-CN"
            }
        }
        "pt" => "pt-BR",
        "es" => "es",
        "ja" => "ja",
        "ko" => "ko",
        "fr" => "fr",
        "de" => "de",
        "ru" => "ru",
        _ => "en",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_chinese_variants() {
        assert_eq!(resolve(Some("zh-TW")), "zh-TW");
        assert_eq!(resolve(Some("zh_TW.UTF-8")), "zh-TW");
        assert_eq!(resolve(Some("zh-Hant-HK")), "zh-TW");
        assert_eq!(resolve(Some("zh-MO")), "zh-TW");
        assert_eq!(resolve(Some("zh-CN")), "zh-CN");
        assert_eq!(resolve(Some("zh-Hans-SG")), "zh-CN");
        assert_eq!(resolve(Some("zh")), "zh-CN");
    }

    #[test]
    fn maps_other_languages_and_fallback() {
        assert_eq!(resolve(Some("pt-PT")), "pt-BR");
        assert_eq!(resolve(Some("es-419")), "es");
        assert_eq!(resolve(Some("ja-JP")), "ja");
        assert_eq!(resolve(Some("de_DE")), "de");
        assert_eq!(resolve(Some("it-IT")), "en");
        assert_eq!(resolve(Some("en-GB")), "en");
        assert_eq!(resolve(None), "en");
    }

    #[test]
    fn every_language_is_supported() {
        assert_eq!(LANGUAGES.len(), 10);
        for (code, _) in LANGUAGES {
            assert!(is_supported(code));
            assert_eq!(LANGUAGES[index_of(code)].0, *code);
        }
    }
}
