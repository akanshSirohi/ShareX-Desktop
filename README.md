# ShareX Desktop

A Windows desktop edition of [ShareX Android](https://github.com/akanshSirohi/ShareX), built with **Tauri v2, Rust, React, and TypeScript**. Share files directly with browsers on your local network. The original ShareX logo and browser portal are preserved.

## Run

```powershell
npm ci
npm run tauri dev
```

Requirements: Windows 10/11, Microsoft Edge WebView2 Runtime, Node.js 22+, Rust MSVC, and Visual Studio C++ Build Tools with a Windows SDK.

```powershell
npm run tauri build
```

The executable is generated at `src-tauri/target/release/sharex-desktop.exe`. The NSIS installer is generated under `src-tauri/target/release/bundle/nsis/`. Install the app or keep the executable with its generated `portal/` resource folder. A standalone executable without the portal resources cannot serve the browser interface.

## Share files

1. Open **Manage storage**. Choose all eligible drives, one drive, one folder, or a private selection of files.
2. Save your selection. Read-only access is enabled by default. Change it under **Settings** when browsers should upload or manage files.
3. Select **Start sharing**. Open the displayed address on a device connected to the same Wi-Fi or hotspot, or scan the QR code.
4. Approve the browser under **Devices**, or enter the optional browser password. Remember trusted browsers when appropriate.
5. Stop sharing when finished. The tray menu can also stop sharing or reopen the window.

While sharing, closing the window hides it in the system tray. Choose **Quit** from the tray menu to stop sharing and exit.

Windows may ask whether to allow network access. Allow it on your private network to connect from other devices. No administrator privileges are required. The application does not change firewall rules, unlock drives, or disable BitLocker.

To open another ShareX device, choose **Connect to a device**, select HTTP or HTTPS, and enter the other device's IP and port. The IP field starts with this PC's network prefix, so complete the destination's last number. ShareX remembers the last opened device. You can also paste a full address or scan a QR code to populate all three fields. Starting a desktop sharing session is not required to open the Android portal.

## Features

- All-drive virtual root, single-drive root, single-folder root, and selected-file private mode.
- Exclusion of BitLocker protected, unreadable, network, and unknown-status drives. Drive protection is checked at startup and every 10 seconds; a changed root stops the session.
- Browser approval, remembered browsers, revocation, optional password, session duration, and login rate limits.
- File browsing, search, sorting, list/grid layouts, selection, rename, folders, deletion, copy, move, and Explorer reveal.
- Streamed uploads/downloads, multiple-file ZIP downloads, media byte ranges, transfer history, live byte counts, average transfer speeds, and persistent totals.
- Native image/text previews. The browser portal handles image, audio, video, PDF, Office, Markdown, code, CSV, and other supported document previews.
- Connection QR code, camera or image QR reader, and manual connection to another ShareX device.
- Private uploads are saved in the application's `Received` folder. Private selections remain separate from incoming files.
- Plugin store, local ZIP installation, updates, enable/disable, uninstall, browser-specific plugin approval, peer messaging, and persistent JSON databases.
- Revocable plugin development connections on sharing port plus one, with isolated development storage.
- HTTP or HTTPS using a generated local certificate, configurable port, seven portal themes, and native dark/light appearance.
- System tray controls, keyboard search shortcut, reduced-motion support, and responsive layouts.

## Platform differences and limits

Android APK export is unavailable on Windows. The browser's Apps list stays empty; normal files and executable installers can be shared from the selected root. ShareX does not package installed Windows applications.

HTTPS uses a self-signed certificate. Browsers must trust it before connecting; regenerated network addresses may require regenerating the certificate. Camera availability depends on Windows privacy settings and the installed WebView2 runtime. Manual addresses and QR images work without a camera.

Uploads and assembled ZIP downloads have a 10 GiB limit per file/archive. Individual file downloads are streamed without that archive limit. Existing destinations are not overwritten. Copy/move failures report partial completion; copies already made remain in the destination. Operation progress is tracked per completed source item. Transfer speed is the elapsed average, not a rolling sample.

Drive detection uses Windows Shell's BitLocker volume property and excludes any nonzero or unavailable value. This is deliberately conservative: some decrypted volumes may also be excluded. The test machine has unencrypted C: and D: drives; an actual encrypted-drive transition still needs hardware validation. Junctions, symbolic links, and mounted folder reparse points inside a root are excluded.

Plugins execute browser code with the access granted to that browser. Install only trusted plugins. The desktop plugin database implements the SDK's common JSONPath selection and update operations; unusual Jayway-specific JSONPath extensions may differ from Android. Legacy on-phone plugin development folders are replaced by desktop development connections.

## Validation

```powershell
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib -- --test-threads=1
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Tests cover root boundaries, excluded drives, private selections, hidden files, restricted changes, safe names, collision prevention, plugin archive validation, plugin database isolation, real browser authentication, streamed transfers, and HTTP ranges. UI checks use Playwright against the local frontend. See `docs/VALIDATION.md` for checked flows.

## Browser portal source

`portal-source/` contains the upstream portal source. `src-tauri/portal/` contains the bundled `sharex_web_v4_8` static export from the supplied Android project. Build with `npm ci && npm run build` inside `portal-source`, then run the desktop-safe synchronization script:

```powershell
node scripts/sync-portal.mjs
```

The upstream `copy:assets` script targets Android asset paths; use the desktop script for this project.

## License and attribution

AGPL-3.0-or-later. The browser portal and logo originate from ShareX by **Akansh Sirohi and contributors**. The original license is included in `LICENSE`. This edition adds the Tauri host, Rust server, Windows storage policy, and desktop interface. Distribution must retain the applicable attribution and license and make the corresponding source available. This project is separate from the unrelated Windows screenshot application also called ShareX.
