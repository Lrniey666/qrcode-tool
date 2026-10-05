<div align="center">

<img src="docs/assets/banner.jpg" alt="QRcode tool — Scan QR codes on your computer as easily as on your phone.">

# QRcode tool

Press a shortcut → every QR code on your screen gets a label → press <kbd>Enter</kbd>.<br>
Works with your webcam too. Free, open source and private.

[![CI](https://github.com/Lrniey666/qrcode-tool/actions/workflows/ci.yml/badge.svg)](https://github.com/Lrniey666/qrcode-tool/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Lrniey666/qrcode-tool?sort=semver&display_name=tag)](https://github.com/Lrniey666/qrcode-tool/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Lrniey666/qrcode-tool/total)](https://github.com/Lrniey666/qrcode-tool/releases)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](#installation)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange?logo=rust)](docs/BUILDING.md)
[![Made with Slint](https://img.shields.io/badge/UI-Slint-2379F4)](https://slint.dev)
[![Languages](https://img.shields.io/badge/languages-10-informational)](docs/TRANSLATING.md)
[![Privacy](https://img.shields.io/badge/privacy-no%20telemetry-success)](docs/PRIVACY.md)

**English** · [繁體中文](docs/i18n/README.zh-TW.md)

<img src="docs/assets/screen-scan.png" width="820" alt="Screen scan: every code on the screen gets a pill with its site name">

</div>

---

## Features

| | |
|---|---|
| ⚡ **Easy to use** | Press <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Q</kbd>, or click the tray icon, to freeze the screen. A quick scan sweep finds every code automatically. |
| 🏷️ **Link labels** | Each code shows the real site name. <kbd>Enter</kbd> opens it, <kbd>Tab</kbd> moves to the next one, <kbd>C</kbd> copies. |
| ✂️ **Select an area** | When a code is tiny, blurry or tilted, drag a box around it like a snipping tool. |
| 📷 **Webcam** | Hold a printed code or a phone screen up to the camera; results appear live. |
| 🛡️ **Link safety** | Never opens a link by surprise. Warns about look-alike letters, disguised addresses (`bank.com@evil.io`), raw IP addresses, unencrypted `http`, short links and more — all offline. |
| 📶 **Understands the content** | Wi-Fi (connect with one click), contacts (save as vCard), email, phone, SMS, map locations, 2-step verification keys, Taiwan e-invoices (both codes merged) and plain text. |
| 🧾 **Barcodes** | EAN, UPC, Code 128, Data Matrix, PDF417, Aztec… |
| 🖼️ **Images & clipboard** | Scan a screenshot you copied or any image file. |
| 🔳 **Simple QR generator** | Turn a link or text into a QR code; copy it or save PNG/SVG. |
| 🧰 **Toolbar & bubble** | A clean floating toolbar that collapses into a bubble, and hides automatically during full-screen apps. |
| 🎨 **Native look** | Windows style, follows light/dark mode and your accent colour. |
| 🌍 **10 languages** | English, 繁體中文, 简体中文, Español, Português (Brasil), 日本語, 한국어, Français, Deutsch, Русский. |
| 🔒 **Private** | Everything runs on your computer. No account, no telemetry, no online link checks. History is off by default. |
| ☁️ **Light and fast** | A native Rust core, with faster performance and lower memory use. |

## Demo

<table>
<tr>
<td width="50%"><img src="docs/assets/link-warning.png" alt="A short http link is flagged before opening"><br><sub>Suspicious links show why — and wait for you.</sub></td>
<td width="50%"><img src="docs/assets/wifi-details.png" alt="Wi-Fi code with connect button"><br><sub>Wi-Fi codes connect with one click; the password stays hidden.</sub></td>
</tr>
<tr>
<td><img src="docs/assets/image-scan.png" alt="Scanning a picture from the clipboard"><br><sub>Scan the clipboard or an image file.</sub></td>
<td><img src="docs/assets/create-qr.png" alt="Create a QR code"><br><sub>Create a QR code to send a link to your phone.</sub></td>
</tr>
<tr>
<td><img src="docs/assets/settings.png" alt="Settings"><br><sub>Windows 11-style settings.</sub></td>
<td><img src="docs/assets/link-warning-dark.png" alt="Dark mode"><br><sub>Follows dark mode and your accent colour.</sub></td>
</tr>
</table>

## Architecture

```mermaid
flowchart LR
    subgraph Triggers
        HK[Global shortcut] --> D
        TR[Tray icon / menu] --> D
        TB[Toolbar / bubble] --> D
        CLI["qrcode-tool scan-screen<br/>(IPC, e.g. Wayland)"] --> D
    end
    D{{Dispatcher}} --> CAP[Capture all monitors<br/>xcap]
    D --> CAM[Webcam<br/>nokhwa]
    D --> IMG[Image / clipboard]
    CAP --> OV[Frozen full-screen overlay<br/>per monitor]
    CAP --> DEC
    CAM --> DEC
    IMG --> DEC
    DEC[Decode<br/>zxing-cpp + upscale + tiles] --> CLS[Classify payload<br/>URL · Wi-Fi · vCard · e-invoice…]
    CLS --> SAFE[Link safety analysis<br/>offline]
    SAFE --> UI[Pills & detail card<br/>Slint]
    UI -->|Enter / click| ACT[Open in browser · Connect Wi-Fi · Save contact · Copy]
```

The app is a Cargo workspace: a thin UI crate on top of small, testable libraries.
See **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** for threads, data flow and design decisions.

<details>
<summary><b>Technical details</b> (click to expand)</summary>

#### Decoding strategy
- [zxing-cpp](https://github.com/zxing-cpp/zxing-cpp) reads the frozen screenshot of each monitor in parallel.
- Small screens are also read at 2× scale; large screens are additionally split into overlapping 900 px tiles, so a small code on a 4K monitor is still found.
- Region selection adds a quiet zone and upscales the crop 2–3×.
- The webcam decodes the newest frame on its own thread and a code has to be seen in three consecutive frames before it counts.

#### DPI and multiple monitors
- The screen is captured first and every overlay shows that frozen picture. All coordinates stay in screenshot pixels and are scaled inside the UI, so mixed-DPI setups work without conversion bugs.

#### Link safety rules
- Only `http`/`https` links can be opened; any other scheme is shown and can only be copied.
- Nothing is passed to a shell. Links open with the system browser API.
- Warnings: unencrypted `http`, user-info disguise (`a.com@b.com`), international domains (shows the punycode), mixed alphabets in one label, IP addresses, unusual ports, short links, very long or heavily encoded URLs, deep subdomain chains.
- Links are never opened automatically unless they are the *only* code found, have *no* warnings, and the site is on your *trusted list*.
- Short links are **not** expanded online (that would tell the shortener what you scanned).

#### Platform notes
| | Windows 10 21H2+ / 11 | macOS 13+ | Linux X11 | Linux Wayland |
|---|---|---|---|---|
| Screen scan | ✅ | ✅ (Screen Recording permission) | ✅ | ⚠️ via portal / compositor |
| Global shortcut | ✅ | ✅ | ✅ | ➖ bind `qrcode-tool scan-screen` in system settings |
| Hide in full-screen apps | ✅ | ➖ | ➖ | ➖ |
| Updates | in-app (Velopack) | in-app | in-app (AppImage) | in-app (AppImage) |

#### Sensitive data
- 2-step verification secrets and Wi-Fi passwords are never written to history and never copied automatically.
- Logs contain events and errors only, never scanned content.

</details>

## Installation

### Windows 10 (21H2+) / 11 — x64 and ARM64
1. Download **`QRcodeTool-win-x64-Setup.exe`** (or `…-arm64-…`) from the [latest release](https://github.com/Lrniey666/qrcode-tool/releases/latest).
2. Run it. QRcode tool installs for your user only (no admin rights) and starts in the tray.
3. Press <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Q</kbd> to scan (change it in *Settings › Shortcuts* if another app already uses it). Right-click the desktop or a picture for the same tools (*Settings › General › Right-click menu*). On Windows 11, open **Show more options**.

> **"Windows protected your PC"?** Early releases are not code-signed yet, so SmartScreen may warn about an unknown publisher. Click **More info → Run anyway**. You can verify the download with the SHA-256 checksums published with every release. See the [FAQ](docs/FAQ.md#windows-smartscreen).

A **portable** `.zip` is available too (no installer, no automatic updates).

### macOS 13+ (Apple silicon and Intel)
Download the `.pkg` from the [latest release](https://github.com/Lrniey666/qrcode-tool/releases/latest). On first scan, allow **Screen Recording** (and **Camera** for webcam scanning) in *System Settings › Privacy & Security*. Releases are not notarized yet — right-click the app and choose **Open** the first time.

### Linux
Download the `.AppImage`, make it executable and run it. On Wayland, add a keyboard shortcut in your desktop settings that runs `qrcode-tool scan-screen`.

### Build from source
```bash
git clone https://github.com/Lrniey666/qrcode-tool
cd qrcode-tool
cargo run --release
```
You need Rust 1.85+, CMake and a C++ compiler (for zxing-cpp). Details for every platform: **[docs/BUILDING.md](docs/BUILDING.md)**.

### Command line
```text
qrcode-tool                     start (or show the toolbar if already running)
qrcode-tool scan-screen         scan the screen
qrcode-tool scan-region         freeze the screen and drag a box
qrcode-tool scan-camera         scan with the webcam
qrcode-tool scan-clipboard      scan the image on the clipboard
qrcode-tool scan-file FILE      scan one image file
qrcode-tool show-toolbar        show the floating toolbar
qrcode-tool generate "TEXT"     create a QR code
qrcode-tool decode FILE… [--json]   decode images without the UI
```

## Project Structure

```text
qrcode-tool/
├─ app/                    # The desktop app (Slint UI + glue)
│  ├─ src/                 #   tray, hotkeys, overlay, preview, settings, updater…
│  ├─ ui/                  #   .slint windows, widgets, icons
│  └─ translations/        #   .po files for 9 languages
├─ crates/
│  ├─ qrtool-core/         # Payload parsing, link safety, settings, history (pure Rust)
│  ├─ qrtool-decode/       # zxing-cpp decoding strategy + QR generation
│  ├─ qrtool-capture/      # Multi-monitor screenshots (xcap)
│  ├─ qrtool-camera/       # Webcam capture + live decoding (nokhwa)
│  └─ qrtool-platform/     # OS integration: open, autostart, theme, Wi-Fi, IPC, clipboard, shell menu
├─ assets/icons/           # App and tray icons
├─ packaging/              # MSIX, Linux desktop entry, macOS bundle info
├─ docs/                   # Architecture, building, FAQ, privacy, translations
└─ .github/workflows/      # CI and release pipeline
```

## Contribution

Contributions are welcome — bug reports, translations, code and ideas.

- Read **[CONTRIBUTING.md](CONTRIBUTING.md)** to get started.
- Improve a translation: **[docs/TRANSLATING.md](docs/TRANSLATING.md)** (the current translations are machine translations that were reviewed; native speakers are very welcome).
- Found a security problem? Please follow **[SECURITY.md](SECURITY.md)** instead of opening a public issue.
- Everyone taking part is expected to follow the **[Code of Conduct](CODE_OF_CONDUCT.md)**.

If QRcode tool saves you time, you can [sponsor the project](https://github.com/sponsors/Lrniey666). ❤️

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution you submit for inclusion in this project shall be dual licensed as above, without any additional terms or conditions.

Third-party components and their licenses are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). The user interface is built with [Slint](https://slint.dev) under the Slint Royalty-free Desktop License.

### Related projects

QRcode tool was inspired by these projects. No code was copied from them; thank you for showing what desktop QR scanning can be:

- [qrpeek](https://github.com/dzoffice/qrpeek) — screen QR code recognition tool
- [ScreenQRScanner](https://github.com/Konyaco/ScreenQRScanner) — on-screen QR code scanner
- [QR-Code-Reader](https://github.com/ottozumkeller/QR-Code-Reader) — Snipping Tool + ZXing.Net based reader
- [NormCap](https://github.com/dynobo/normcap) — OCR-powered screen capture
- [ShareX](https://github.com/ShareX/ShareX) — screen capture suite with a QR code tool
- [CodeTwo QR Code Desktop Reader](https://www.codetwo.com/freeware/qr-code-desktop-reader/) — freeware desktop reader

Built on the shoulders of [zxing-cpp](https://github.com/zxing-cpp/zxing-cpp), [Slint](https://github.com/slint-ui/slint), [xcap](https://github.com/nashaofu/xcap), [nokhwa](https://github.com/l1npengtul/nokhwa), [global-hotkey](https://github.com/tauri-apps/global-hotkey), [Velopack](https://github.com/velopack/velopack) and [qrcode](https://github.com/kennytm/qrcode-rust).

---

<sub>QR Code is a registered trademark of DENSO WAVE INCORPORATED. QRcode tool is not affiliated with DENSO WAVE, Google or Microsoft.</sub>
