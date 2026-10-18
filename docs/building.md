# Building ShareX Desktop

Build on the operating system you want to distribute for. The commands below use the committed lockfiles and Tauri 2. GitHub Actions also builds native release packages automatically; see [automatic releases](releases.md). macOS and Linux remain experimental pending functional testing on native hardware.

## Install a release

Download packages from [the latest GitHub release](https://github.com/akanshSirohi/ShareX-Desktop/releases/latest), under **Assets**. Each asset name includes the version, platform, and architecture. Download the binary package rather than GitHub's source archives.

- **Windows x64:** run the `windows_x64-setup.exe` installer. WebView2 is required. Windows may display an unknown-publisher prompt because the installer is unsigned.
- **Apple Silicon Mac:** open `darwin_aarch64.dmg` and drag ShareX Desktop to Applications.
- **Intel Mac:** use `darwin_x64.dmg` instead. An application archive is also available for each architecture.
- **Debian/Ubuntu x64:** install the `.deb` with `sudo apt install ./<downloaded-file>.deb` so required system dependencies can be resolved.
- **Other Linux x64:** make the AppImage executable with `chmod +x ./<downloaded-file>.AppImage`, then run it. FUSE support may be required. Release packages are built on Ubuntu 22.04; other distributions need compatible libraries.

macOS packages are unsigned and not notarized. Follow Apple's [guidance for opening an app from an unidentified developer](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unidentified-developer-mh40616/mac) only after verifying the download came from this repository. Do not disable system-wide security controls. Future signed distribution requires maintainer certificates and notarization credentials.

Use a single folder on macOS and Linux. Packaging availability does not establish parity with Windows drive protection checks.

## Common requirements

- Git, Node.js 24 or newer, and npm.
- Stable Rust installed through [rustup](https://rustup.rs/).
- The platform dependencies in the [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

After installing the platform requirements:

```sh
git clone https://github.com/akanshSirohi/ShareX-Desktop.git
cd ShareX-Desktop
npm ci
npm run tauri dev
```

Create a release with:

```sh
npm run tauri build
```

The checked-in browser portal is bundled automatically. You only need its separate dependencies when editing or rebuilding it.

## Windows

Install Visual Studio Build Tools with **Desktop development with C++**, including the Windows SDK. Install the Microsoft Edge WebView2 runtime if it is missing. Use the stable Rust MSVC toolchain. Run the common commands in PowerShell or a developer terminal.

The NSIS installer is written to `src-tauri/target/release/bundle/nsis/`. Windows drive discovery uses PowerShell; BitLocker-protected drives and unknown protection states are excluded.

## macOS

Install Xcode Command Line Tools:

```sh
xcode-select --install
```

Install Node.js and stable Rust, then run the common commands in Terminal. Tauri uses the system WKWebView. The platform configuration creates an application bundle in `src-tauri/target/release/bundle/macos/` and a disk image in `src-tauri/target/release/bundle/dmg/`.

The default build targets your Mac's architecture. For a universal Apple Silicon and Intel build:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm run tauri build -- --target universal-apple-darwin
```

Universal outputs are under `src-tauri/target/universal-apple-darwin/release/bundle/`. Automated releases provide separate Apple Silicon and Intel builds. Signing and notarization require your own Apple credentials; neither credentials nor signing settings are included. See [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/).

Use **single folder** sharing when testing. Mounted-volume enumeration and FileVault checks are not implemented.

## Linux

On Ubuntu 24.04 / Debian-based systems with WebKitGTK 4.1 packages:

```sh
sudo apt update
sudo apt install --no-install-recommends \
  build-essential pkg-config curl wget file \
  libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev
```

Install Node.js and stable Rust, then run the common commands. For other distributions, use the equivalent packages listed in the official Tauri prerequisites.

Debian packages are written to `src-tauri/target/release/bundle/deb/`; AppImages go to `src-tauri/target/release/bundle/appimage/`. AppImages may require FUSE support on the target machine. Tray support depends on the desktop environment. Build on the oldest distribution you intend to support to avoid requiring newer system libraries on users' machines. See [Tauri Linux distribution](https://v2.tauri.app/distribute/).

Use **single folder** sharing when testing. Mounted-volume enumeration and LUKS checks are not implemented.

## Rebuild the browser portal

```sh
npm --prefix portal-source ci
npm run test:portal
npm run build:portal
npm run tauri build
```

Commit both portal source changes and the updated `src-tauri/portal/` export. Do not copy the desktop portal into the Android repository's asset directory.

## Configuration and checks

Tauri automatically merges `tauri.windows.conf.json`, `tauri.macos.conf.json`, or `tauri.linux.conf.json` with the base configuration on the respective platform. Each defines its native bundle types and icons.

```sh
npm run build
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Native builds require the platform dependencies even for Rust tests. Cross-compiling from Windows does not replace validating the application on macOS and Linux. The [Release workflow](../.github/workflows/release.yml) publishes packages after successful builds on all targets. Release signing is not configured.
