# Building QRcode tool

QRcode tool is a Rust workspace. The only non-Rust part is **zxing-cpp** (the barcode
decoder), which the `zxing-cpp` crate compiles from source with CMake and a C++17 compiler.

| Requirement | Version |
|---|---|
| Rust | 1.85 or newer (`rustup update`) |
| CMake | 3.16 or newer |
| C++ compiler | MSVC 2019+ (Windows), Clang (macOS), GCC 9+/Clang (Linux) |

The first build takes a few minutes (zxing-cpp and Slint); later builds are incremental.

## Windows 10 / 11 (recommended: MSVC)

1. Install **Rust**: <https://rustup.rs> (choose the default `x86_64-pc-windows-msvc` toolchain).
2. Install the **Visual Studio 2022 Build Tools** with the workload
   *Desktop development with C++* (includes MSVC, the Windows SDK and CMake):
   ```powershell
   winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
   ```
   Or install CMake separately: `winget install Kitware.CMake`.
3. Build and run (in *Developer PowerShell for VS* or a normal terminal after a restart):
   ```powershell
   git clone https://github.com/naritaroad/qrcode-tool
   cd qrcode-tool
   cargo run --release
   ```
   The executable is `target\release\qrcode-tool.exe`. It is self-contained; copy it anywhere.

The build script embeds the icon, version information and the DPI-aware manifest
(`app/windows/app.manifest`) into the executable.

**ARM64**: `rustup target add aarch64-pc-windows-msvc`, install the *MSVC ARM64 build tools*
component, then `cargo build --release --target aarch64-pc-windows-msvc`.

### Alternative: cross-compile from Linux with mingw-w64

Useful for CI or when you only have Linux at hand. This produces a GNU-ABI executable that runs
on Windows 10/11 x64.

```bash
sudo apt install mingw-w64 cmake
rustup target add x86_64-pc-windows-gnu
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc-posix
export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++-posix
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc-posix
export WINDRES=x86_64-w64-mingw32-windres
export CMAKE_TOOLCHAIN_FILE_x86_64_pc_windows_gnu=$PWD/packaging/windows/mingw-w64.cmake
cargo build --release --target x86_64-pc-windows-gnu
```

The posix-thread variants of the mingw compilers are required (zxing-cpp uses `std::thread`).
Official releases are built with MSVC on GitHub Actions.

## macOS 13+

```bash
xcode-select --install          # Clang
brew install cmake
cargo run --release
```

The app needs **Screen Recording** permission (and **Camera** for webcam scanning). When you
run the binary from a terminal, macOS asks for the permission for the terminal app.
See `packaging/macos/` for the app bundle `Info.plist` (camera usage text, `LSUIElement`).

## Linux

Debian / Ubuntu:

```bash
sudo apt install build-essential cmake clang libclang-dev pkg-config \
  libxcb1-dev libxcb-randr0-dev libxcb-shm0-dev libxkbcommon-dev libfontconfig-dev \
  libwayland-dev libegl-dev libgbm-dev libdrm-dev \
  libpipewire-0.3-dev libspa-0.2-dev linux-libc-dev
cargo run --release
```

Fedora:

```bash
sudo dnf install gcc-c++ cmake clang-devel pkgconf-pkg-config libxcb-devel \
  libxkbcommon-devel fontconfig-devel wayland-devel mesa-libEGL-devel mesa-libgbm-devel \
  libdrm-devel pipewire-devel
```

Runtime notes: the tray icon uses the StatusNotifierItem D-Bus protocol (supported by KDE,
Xfce, Cinnamon, and GNOME with the *AppIndicator* extension). Wi-Fi connection uses
`nmcli` (NetworkManager). Sound uses `canberra-gtk-play` when available.

## Cargo features

| Feature | Default | What it does |
|---|---|---|
| `camera` | on | Webcam scanning (nokhwa). Disable for a smaller build: `--no-default-features --features updater` |
| `updater` | on | In-app updates through Velopack. Disable for store or distribution packages |

## Tests and checks

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
python3 scripts/check_translations.py
```

`cargo run -- decode image.png --json` decodes images without starting the UI, which is handy
for testing detection.

## Packaging

Releases are produced by `.github/workflows/release.yml` with
[Velopack](https://velopack.io) (`vpk`): installer + delta updates for Windows, a `.pkg` for
macOS and an AppImage for Linux, plus a portable zip for Windows. To build a Windows installer
locally:

```powershell
dotnet tool install -g vpk
cargo build --release
mkdir dist; copy target\release\qrcode-tool.exe dist\
vpk pack --packId QRcodeTool --packVersion 1.0.0 --packDir dist --mainExe qrcode-tool.exe `
  --packTitle "QRcode tool" --packAuthors "Lrniey666" --icon assets\icons\app.ico --channel win-x64
```

The MSIX manifest for the Microsoft Store lives in `packaging/windows/`.
