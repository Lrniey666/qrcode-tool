//! UI language selection. All translations are bundled into the binary by `build.rs`.

use qrtool_core::locale;

/// The language actually used for `setting` ("" = follow the system).
pub fn effective_language(setting: &str) -> &'static str {
    if !setting.is_empty() {
        if let Some((code, _)) = locale::LANGUAGES.iter().find(|(c, _)| *c == setting) {
            return code;
        }
    }
    locale::resolve(qrtool_platform::system_locale().as_deref())
}

/// Switches all windows to the language for `setting`. Must be called after the first
/// Slint component was created.
pub fn apply(setting: &str) {
    let code = effective_language(setting);
    // "en" is the source language: selecting it restores the untranslated strings.
    if let Err(err) = slint::select_bundled_translation(code) {
        tracing::warn!("cannot select language {code}: {err}");
    } else {
        tracing::info!("language: {code}");
    }
}

/// Entries for the language pickers: "Use system language" followed by native names.
pub fn picker_model(system_label: &str) -> slint::ModelRc<slint::SharedString> {
    let mut items: Vec<slint::SharedString> = vec![system_label.into()];
    items.extend(
        locale::LANGUAGES
            .iter()
            .map(|(_, name)| slint::SharedString::from(*name)),
    );
    slint::ModelRc::new(slint::VecModel::from(items))
}

/// Index in [`picker_model`] for a language setting.
pub fn picker_index(setting: &str) -> i32 {
    if setting.is_empty() {
        return 0;
    }
    locale::LANGUAGES
        .iter()
        .position(|(c, _)| *c == setting)
        .map_or(0, |i| i as i32 + 1)
}

/// Language setting for a picker index.
pub fn setting_for_index(index: i32) -> String {
    if index <= 0 {
        return String::new();
    }
    locale::LANGUAGES
        .get(index as usize - 1)
        .map(|(c, _)| (*c).to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_roundtrip() {
        for (code, _) in locale::LANGUAGES {
            assert_eq!(setting_for_index(picker_index(code)), *code);
        }
        assert_eq!(picker_index(""), 0);
        assert_eq!(setting_for_index(0), "");
    }
}
