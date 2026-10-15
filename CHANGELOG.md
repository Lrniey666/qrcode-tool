# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/2.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-10-12

### Added

- The floating toolbar can be a vertical column at the right (default) or left edge of the screen, in addition to the horizontal row at the top. Choose it in *Settings › General › Toolbar layout*; the toolbar moves to the default place of the new layout right away.
- User manual in English and Traditional Chinese, as PDF and Word, in `docs/manual` and attached to every release. The new **?** button on the floating toolbar opens the PDF in the browser, in Traditional Chinese when the UI is Chinese and in English otherwise.
- On Windows, the desktop right-click menu can scan the screen, scan a selection, open the camera and show the toolbar. Image files (PNG, JPEG, BMP, GIF, WebP) can be scanned from their own right-click menu. Turn it off in *Settings › General › Right-click menu*. On Windows 11 the entries are under **Show more options**.

### Changed

- The default screen-scan shortcut is <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Q</kbd> on every platform. Windows installs that still had the previous untouched default (<kbd>Win</kbd>+<kbd>Shift</kbd>+<kbd>Q</kbd>) are moved back; a shortcut you chose yourself is kept.

## [0.1.0] - 2026-10-05

First public preview.

### Added

- Screen scan with one shortcut (<kbd>Win</kbd>+<kbd>Shift</kbd>+<kbd>Q</kbd> on Windows, <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Q</kbd> elsewhere) or a tray click, with a scan-sweep animation:
  freezes all monitors, finds every code automatically and shows Lens-style pills.
- Region selection for small or hard codes; keyboard control (Enter, Tab, C, F, Esc).
- Webcam scanning with a camera picker and mirror option. Detected codes stay in a steady list
  for a few seconds after they leave the picture (adjustable), so links do not jump around.
- Scanning images from files and the clipboard.
- Link safety analysis (offline) with warnings and a second confirmation for dangerous links;
  automatic opening only for trusted sites.
- Understands Wi-Fi (one-click connect on Windows, macOS and NetworkManager), vCard/MECARD
  contacts, email, phone, SMS, geo locations, 2-step verification keys, payment addresses,
  Taiwan e-invoices (left and right code merged) and all common barcodes.
- QR code generator with copy, PNG and SVG export.
- Floating toolbar and floating bubble (hidden during full-screen apps on Windows).
- Tray icon, autostart, single instance and command-line interface
  (`scan-screen`, `scan-camera`, `scan-clipboard`, `generate`, `decode`, …).
- Settings window, optional local history (off by default), welcome screen.
- Windows 11 look that follows light/dark mode and the accent colour.
- 10 languages: English, 繁體中文, 简体中文, Español, Português (Brasil), 日本語, 한국어,
  Français, Deutsch, Русский.
- In-app updates through Velopack and GitHub Releases.

[Unreleased]: https://github.com/naritaroad/qrcode-tool/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/naritaroad/qrcode-tool/releases/tag/v1.0.0
[0.1.0]: https://github.com/naritaroad/qrcode-tool/releases/tag/v0.1.0
