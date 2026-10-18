<p align="center"><img src="src/assets/sharex-logo.png" width="76" alt="ShareX logo" /></p>
<h1 align="center">ShareX Desktop</h1>
<p align="center">Local file sharing between your desktop, phone, and browser.</p>
<p align="center"><a href="https://github.com/akanshSirohi/ShareX-Desktop/releases">Downloads</a> · <a href="docs/building.md">Build from source</a> · <a href="https://github.com/akanshSirohi/ShareX">Android app</a> · <a href="CONTRIBUTING.md">Contribute</a></p>

![ShareX Desktop file workspace](screenshots/webp/hero-light.webp)

ShareX Desktop brings the [ShareX Android](https://github.com/akanshSirohi/ShareX) workflow to the desktop with **Tauri 2, Rust, React, and TypeScript**. Files move directly over your local network; receiving devices connect through a browser. The original ShareX logo and browser portal are retained.

## Features

- **Choose what to share:** a folder, a drive, all eligible Windows drives, or selected files.
- **Control access:** browser approval, remembered browsers, revocation, optional passwords, and session expiry.
- **Manage files:** browse, search, preview, upload, download, create folders, rename, copy, move, and delete within the shared root.
- **Follow transfers:** history, byte progress, average speeds, and sent/received totals.
- **Connect devices:** QR codes, QR image import, camera scanning, and HTTP/HTTPS connection controls.
- **Extend the portal:** install plugins from the store or ZIP files, approve plugin access, and use plugin messaging and storage.
- **Make it yours:** dark/light appearance, seven portal themes, keyboard search, and tray controls.

Read-only access is enabled by default. Windows BitLocker protected drives and drives with unknown protection status are excluded.

## Download and platform status

Download installers from **[the latest release](https://github.com/akanshSirohi/ShareX-Desktop/releases/latest)**. Expand **Assets** and choose the package for your system. Releases publish automatically after all platform builds succeed.

| Platform | Status | Download |
| --- | --- | --- |
| Windows 10/11 x64 | Primary platform; native builds and local sharing tested | `windows_x64-setup.exe` installer |
| macOS Apple Silicon | Experimental | `darwin_aarch64.dmg` |
| macOS Intel | Experimental | `darwin_x64.dmg` |
| Linux x64 | Experimental | `linux_amd64.AppImage` or `linux_amd64.deb` |

Asset names also include the application version. Install the Windows executable, drag the macOS app from the DMG into Applications, or follow the [Linux installation steps](docs/building.md#install-a-release). The macOS application archive is also included. GitHub's **Source code** archives contain source, not installers.

Packages are currently unsigned; macOS packages are not notarized. See [installation and platform details](docs/building.md#install-a-release). If no release is available yet, the first merge enabling the release workflow must finish, or you can build from source below.

macOS and Linux have not been validated on native hardware. Windows volume discovery and BitLocker checks are platform-specific. Use a single folder when testing on macOS or Linux; mounted-volume enumeration and FileVault/LUKS checks are not implemented.

## Start sharing

1. Open **Manage storage**, choose your shared root, and save.
2. Select **Start sharing**.
3. Open the displayed address or scan the QR code from a device on the same Wi-Fi or hotspot.
4. Approve the browser in the popup or on **Devices**. An optional sharing password also grants browser access.
5. Stop sharing when finished. While sharing, closing the window keeps the app in the tray; use **Quit** in the tray menu to exit.

To connect to another ShareX device, choose **Connect to a device** and enter its protocol, IP address, and port. The other device must be running its sharing server. Plugins require a separate approval after browser access.

Local transfers do not require internet access. Online features such as the plugin store do. Allow incoming connections on your trusted local network when your operating system asks.

## Build from source

Install Node.js 24+, stable Rust, and your operating system's [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), then:

```sh
git clone https://github.com/akanshSirohi/ShareX-Desktop.git
cd ShareX-Desktop
npm ci
npm run tauri dev
```

Create a native release build with `npm run tauri build`. See the [Windows, macOS, and Linux build guide](docs/building.md) for system packages, output locations, universal macOS builds, and portal development.

## Documentation

- [Build and packaging](docs/building.md)
- [Architecture](docs/architecture.md)
- [Development and testing](docs/development.md)
- [Known limits](docs/limitations.md)
- [Automatic releases](docs/releases.md)
- [Screenshots](screenshots/README.md)
- [Contributing](CONTRIBUTING.md)
- [Security reporting](SECURITY.md)
- [Changelog](CHANGELOG.md)

## Android project and attribution

The Android application is developed in **[akanshSirohi/ShareX](https://github.com/akanshSirohi/ShareX)**. Android builds belong in that repository; this repository contains the desktop host and its adapted browser portal.

ShareX was created by **Akansh Sirohi and contributors**. This edition adds the desktop interface, Rust sharing server, Windows storage integration, and native packaging. See [NOTICE](NOTICE.md) and [third-party notices](THIRD_PARTY_NOTICES.md).

## License

**AGPL-3.0-or-later.** See [LICENSE](LICENSE). This project is separate from the unrelated Windows screenshot utility also named ShareX.
