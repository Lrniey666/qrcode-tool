# AGENTS.md

Guidance for AI coding agents (and humans) working on this repository.
Read this before changing code. Keep it short and up to date when conventions change.

## What this project is

**QRcode tool** — a tray app that scans QR codes from the screen, the webcam, the clipboard
or image files, shows Lens-style "pills" on every code, and opens links only after safety
checks. Rust workspace + [Slint](https://slint.dev) UI. Windows 10 21H2+/11 is the primary
platform; macOS 13+ and Linux (X11 full, Wayland best-effort) are supported by the same code.

## Layout

| Path | Responsibility | Depends on UI? |
|---|---|---|
| `crates/qrtool-core` | Payload classification, URL safety, Wi-Fi/vCard/e-invoice parsers, settings, history, locale | no — pure Rust, no I/O except settings/history files |
| `crates/qrtool-decode` | zxing-cpp decoding strategy (upscale, tiles, region), QR generation | no |
| `crates/qrtool-capture` | Multi-monitor screenshots (xcap) | no |
| `crates/qrtool-camera` | Webcam capture + background decoding (nokhwa) | no |
| `crates/qrtool-platform` | OS integration: open links, autostart, theme/accent, sound, Wi-Fi connect, clipboard, single-instance IPC, Windows right-click menu, paths | no |
| `app/src` | Slint glue: dispatcher (`app.rs`), overlay, preview, floating toolbar/bubble, settings, generator, welcome, tray, hotkeys, updater | yes |
| `app/ui` | `.slint` files. `main.slint` re-exports every window | — |
| `app/translations` | gettext `.po` files, bundled into the binary at build time | — |

Put logic in the crates and keep `app/` thin. New parsing or safety rules belong in
`qrtool-core` **with unit tests**.

## Non-negotiable rules (security & privacy)

1. **Only `http`/`https` URLs may be opened.** `qrtool_platform::open::open_web` takes a
   `UrlReport` and re-checks the scheme. Never call `open::that`, `ShellExecute`, a shell, or
   `Command` with decoded content.
2. **Never auto-open** unless `scan::auto_open_candidate` says so (single code, clean report,
   trusted site, setting enabled). Warnings (`Severity::Warning`+) always need a click/Enter;
   `Danger` needs a second confirmation ("Open anyway").
3. **No network calls** except the optional update check (Velopack → GitHub). No telemetry,
   no URL reputation lookups, no short-link expansion.
4. **Sensitive payloads** (`Payload::is_sensitive`: OTP secrets, Wi-Fi with password) are never
   written to history or logs and never auto-copied.
5. **Never log decoded content.** Log events and errors only.
6. Text from codes is shown through `scan::display` / `text::sanitize_for_display`
   (strips bidi/zero-width characters, single line, length limit).
7. Wi-Fi profiles on Windows are written to a temporary file that is deleted immediately;
   never build `netsh`/`nmcli` command lines by string concatenation of untrusted input
   beyond the escaped XML/argv already implemented.

## Threading model

Everything that touches Slint runs on the event-loop thread. State lives in a thread-local
`Rc<App>` (`app::app()`). Background work (capture, decode, camera, Wi-Fi, updates, IPC,
hotkey events) runs on `std::thread`s and returns with `slint::invoke_from_event_loop`.
Do not hold a `RefCell` borrow of `App` fields while calling code that may borrow it again;
drop Slint components with `slint::Timer::single_shot(Duration::ZERO, …)` when the drop is
triggered from one of their own callbacks.

## UI conventions

- Every window imports `Theme`; Rust pushes `dark`/`accent`/`high-contrast` into each
  component with `theme::apply_theme!` (globals are per component).
- All user-visible text lives in `.slint` files with `@tr(...)`. Rust gets translated text via
  `Strings.t(key)` / `msg-title` etc. Do not hard-code English in Rust (log messages excepted).
- Overlay pills and region coordinates are **screenshot pixels**; scaling happens in Slint
  (`img-w`/`img-h`). Do not convert to logical pixels in Rust.
- Follow the Windows 11 look: 4/8/12 px radii, `Theme` colours, Fluent std-widgets.

## Translations

1. After changing `@tr` strings run `scripts/update-translations.sh` (or `.ps1`). It regenerates
   `app/translations/qrcode-tool.pot` and merges it into every `.po`.
2. Keep `{}` placeholders identical; the CI checks this.
3. Languages: en (source), zh-TW, zh-CN, es, pt-BR, ja, ko, fr, de, ru.

## User manual

`docs/manual` holds the user manual (PDF + Word, English and Traditional Chinese). Its text is in
`docs/manual/build/content.*.js`; the screenshots are rendered from the real `.slint` files. When a
visible feature or window changes, update the text and rebuild with
`docs/manual/build/build.ps1` (Windows, Node.js, Microsoft Word). The release workflow attaches
the committed files to every GitHub release.

## Checks before you finish

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/check_translations.py
```

On Linux the build needs the packages listed in `docs/BUILDING.md`. zxing-cpp is compiled
from source (CMake + C++17 compiler).

## Things to avoid

- Adding heavy dependencies (web engines, async runtimes for the UI) — size and idle memory
  are product goals (installer < 15 MB, idle < 30 MB).
- Keeping windows alive when hidden; create them on demand and drop them.
- Changing the IPC command format without keeping old commands working.
- Using Google, Microsoft or DENSO WAVE logos or names as branding.
