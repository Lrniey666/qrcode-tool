# Windows packaging

| Package | How | Updates |
|---|---|---|
| Installer (`QRcodeTool-win-x64-Setup.exe`) | `vpk pack` in `.github/workflows/release.yml` | in-app (Velopack) |
| Portable zip | release workflow | manual |
| MSIX (Microsoft Store) | this folder | Store |

## MSIX for the Microsoft Store

Store builds must not update themselves; build without the `updater` feature:

```powershell
cargo build --release --no-default-features --features camera
mkdir msix\Assets
copy target\release\qrcode-tool.exe msix\
copy packaging\windows\AppxManifest.xml msix\
# Store logos (PNG): Square44x44Logo, Square150x150Logo, StoreLogo — from assets\icons\app-*.png
copy assets\icons\app-48.png  msix\Assets\Square44x44Logo.png
copy assets\icons\app-256.png msix\Assets\Square150x150Logo.png
copy assets\icons\app-64.png  msix\Assets\StoreLogo.png
# edit msix\AppxManifest.xml: identity values from Partner Center, __VERSION__, __ARCH__ (x64 / arm64)
makeappx pack /d msix /p QRcodeTool.msix
```

The Store signs the package; no certificate is needed for submission. In the MSIX build the
"Start with the computer" switch should be handled by the `StartupTask` declared in the manifest
(the registry-based autostart is not available to packaged apps).

## Code signing

Release executables are currently unsigned. Planned: SignPath Foundation (free for open source)
in the release workflow, after the project has some history.

## Cross-compiling from Linux

`mingw-w64.cmake` is a CMake toolchain file for building zxing-cpp with mingw-w64; see
`docs/BUILDING.md`.
