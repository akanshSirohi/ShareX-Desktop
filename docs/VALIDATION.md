# Validation

Checked on Windows with Rust 1.98.1 and Node.js 26.1.0.

## Automated backend checks

Thirteen Rust tests cover:

- Canonical root containment, root modification protection, Windows reserved filenames, and path traversal.
- All-drive policy exclusion of protected and unknown-status volumes.
- Hidden files, private-file tokens, read-only restrictions, folder creation, renaming, deletion, and collision handling.
- Copy behavior and upload commits that refuse to overwrite existing files.
- Plugin ZIP staging, unsafe archive paths, persistent JSON data, and package isolation.
- Real TCP HTTP authentication, approval enforcement, uploads, streamed downloads, byte ranges, single-file download selection, named ZIP selection, and transfer completion.
- HTTPS with a generated certificate, password login, and access revocation.
- Direct plugin links, browser registration, persisted plugin approval requests, and separate access checks.
- SDK-compatible WebSocket user presence, database initialization, adjacent-port Origin handling, and development-key revocation.

The bundled browser portal's ten upstream tests pass. Both frontends build successfully. Rust Clippy passes with warnings treated as errors.

## Device connection update (v0.1.1)

Six frontend tests cover HTTP/HTTPS URL construction, subnet prefixes, IP and port validation, pasted/QR addresses, IPv6, and rejection of non-web schemes and cross-origin paths. The twelve Rust tests, frontend build, and Clippy checks also pass after this update.

An isolated native Tauri test profile confirmed the protocol/IP/port controls prefill the PC's subnet while sharing is stopped. HTTPS selection with a custom port produced the expected URL. Opening `http://192.168.31.85:6060/` succeeded through the native opener and closed the dialog without the previous permission error; reopening retained that device address. A direct HTTP request to this Android device returned HTTP 200 and the ShareX workspace page. A non-web URL remained blocked by the native permission scope. Browser approval on the phone and cross-device file transfers were not part of this check.

The Windows v0.1.1 NSIS installer built successfully. The native connection dialog screenshot is saved in ignored `output/playwright/connection-0.1.1.png`.

## Native smoke checks

### Plugin approval update (v0.1.2)

Thirteen Rust tests pass, including a real HTTP regression test that opens a plugin URL without an existing browser cookie. It verifies browser registration, request deduplication, separate plugin approval, persisted pending requests, password login, cross-origin rejection, and denied access. Clippy passes with warnings treated as errors; the frontend build and six connection tests also pass.

An isolated native app profile shared an empty fixture folder on port 6234 and installed a fixture plugin. Opening the plugin link directly showed the browser approval popup and request count. Approving the browser produced a separate plugin popup; approving the plugin automatically loaded the plugin page in the browser. The password form rejected an incorrect password and advanced to plugin approval after the correct password. Opening another native dialog deferred request popups. Native popup rendering was inspected; the screenshot is in ignored `output/playwright/plugin-approval-0.1.2.png`. Test sharing, browser records, and the fixture plugin were removed afterward. The v0.1.2 Windows installer built successfully.

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
