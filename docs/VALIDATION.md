# Validation

Checked on Windows with Rust 1.98.1 and Node.js 26.1.0.

## Automated backend checks

Twelve Rust tests cover:

- Canonical root containment, root modification protection, Windows reserved filenames, and path traversal.
- All-drive policy exclusion of protected and unknown-status volumes.
- Hidden files, private-file tokens, read-only restrictions, folder creation, renaming, deletion, and collision handling.
- Copy behavior and upload commits that refuse to overwrite existing files.
- Plugin ZIP staging, unsafe archive paths, persistent JSON data, and package isolation.
- Real TCP HTTP authentication, approval enforcement, uploads, streamed downloads, byte ranges, single-file download selection, named ZIP selection, and transfer completion.
- HTTPS with a generated certificate, password login, and access revocation.
- SDK-compatible WebSocket user presence, database initialization, adjacent-port Origin handling, and development-key revocation.

The bundled browser portal's ten upstream tests pass. Both frontends build successfully. Rust Clippy passes with warnings treated as errors.

## Device connection update (v0.1.1)

Six frontend tests cover HTTP/HTTPS URL construction, subnet prefixes, IP and port validation, pasted/QR addresses, IPv6, and rejection of non-web schemes and cross-origin paths. The twelve Rust tests, frontend build, and Clippy checks also pass after this update.

An isolated native Tauri test profile confirmed the protocol/IP/port controls prefill the PC's subnet while sharing is stopped. HTTPS selection with a custom port produced the expected URL. Opening `http://192.168.31.85:6060/` succeeded through the native opener and closed the dialog without the previous permission error; reopening retained that device address. A direct HTTP request to this Android device returned HTTP 200 and the ShareX workspace page. A non-web URL remained blocked by the native permission scope. Browser approval on the phone and cross-device file transfers were not part of this check.

The Windows v0.1.1 NSIS installer built successfully. The native connection dialog screenshot is saved in ignored `output/playwright/connection-0.1.1.png`.

## Native smoke checks

The packaged Tauri executable was launched and inspected through its WebView2 debugging connection. Checks used a generated fixture folder inside the ignored `output/` directory, without exposing the user's existing files.

- Native startup, original ShareX branding, storage discovery, folder listing, and the text preview command.
- All eligible drives: two virtual drive entries on this machine. Single-drive and single-folder roots also returned real listings.
- Native browser approval and the browser portal's file listing and document preview.
- An actual browser download completed through the native host.
- The live plugin catalog returned four entries.
- The final release selected the primary Wi-Fi address before virtual/link-local adapters. Importing a generated QR image populated the expected connection URL.
- The rebuilt portal hides Android app navigation and disables uploads in read-only mode.
- Test sharing was stopped; original storage settings were restored and test browser access was removed.

## Visual checks

Playwright checked dark and light desktop layouts and a 375px layout with no horizontal overflow. Native rendering, storage selection, keyboard focus, preview dialogs, browser approval, and reduced-motion CSS were inspected. Screenshots are in ignored `output/playwright/`.

## Remaining hardware/distribution checks

- An actual encrypted BitLocker drive was not available. Policy tests simulate protected and unknown volumes; the machine's C: and D: drives report unencrypted.
- Camera scanning depends on camera hardware, Windows privacy permissions, and WebView2 support. QR image import and manual URLs provide alternatives.
- Cross-device Wi-Fi/hotspot performance and Windows firewall prompts need checking on another physical device.
- The installer is unsigned. Code signing and public distribution are separate release steps.
- Native live transfer speed is an elapsed average. Copy/move progress advances when each source completes.
