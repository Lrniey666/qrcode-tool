# Translating QRcode tool

The app ships in 10 languages. English is the source; the others were machine-translated and
reviewed, so corrections from native speakers are very welcome.

| Language | Folder |
|---|---|
| 繁體中文 (Taiwan) | `zh-TW` |
| 简体中文 | `zh-CN` |
| Español | `es` |
| Português (Brasil) | `pt-BR` |
| 日本語 | `ja` |
| 한국어 | `ko` |
| Français | `fr` |
| Deutsch | `de` |
| Русский | `ru` |

## Fixing or improving a translation

1. Open `app/translations/<folder>/LC_MESSAGES/qrcode-tool.po` with any text editor or with
   [Poedit](https://poedit.net/).
2. Change the `msgstr` lines. Keep:
   - every `{}` placeholder (the same number as in `msgid`),
   - product names (`QRcode tool`, `GitHub`), key names (`Ctrl`, `Tab`, `Enter`…) and command
     lines (`qrcode-tool scan-screen`),
   - short labels short — toolbar and tray texts have little room.
3. `msgctxt` tells you which window the text belongs to (e.g. `ToolbarWindow`, `AppTray`,
   `SettingsWindow`, `Strings` for shared texts).
4. Check your work: `python3 scripts/check_translations.py`.
5. Open a pull request. Screenshots of the changed screens are a big help.

To see your change: build and run the app, then pick the language in *Settings › General ›
Language*. The language switches immediately.

## Adding a new language

1. Create `app/translations/<code>/LC_MESSAGES/` (`<code>` like `it` or `pt-PT`).
2. Copy `app/translations/qrcode-tool.pot` there as `qrcode-tool.po`, fill in the header
   (`Language`, `Plural-Forms`) and translate.
3. Add the language to `LANGUAGES` in `crates/qrtool-core/src/locale.rs` (code and native name)
   and to `locale::resolve` if the system locale needs mapping.
4. Add the language to the table above and to the README.

## For developers: changing UI text

All user-visible text lives in `.slint` files inside `@tr("…")`. After changing it run

```bash
scripts/update-translations.sh      # Linux/macOS
scripts\update-translations.ps1     # Windows
```

This regenerates `app/translations/qrcode-tool.pot` (with `slint-tr-extractor`) and merges it
into every `.po` (with `msgmerge`). New strings stay empty — the app shows English for them
until they are translated.
