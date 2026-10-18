# Development

Follow [the build guide](building.md) for native dependencies, then run `npm ci` and `npm run tauri dev` from the repository root. Use Node.js 24 or newer. The root frontend and portal have separate lockfiles.

## Validation

```sh
npm run build
npm test
npm --prefix portal-source ci
npm run test:portal
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Connection tests cover URL validation and protocol/IP/port handling. Release tests cover version consistency, published-version skipping, immutable tags, retries, and the complete-package publication requirement. Native tests cover server and filesystem behavior. Portal tests cover its export and supporting logic. CI also attempts a native compilation on each platform; a passing compile does not establish functional parity.

## Browser portal

See [the portal guide](../portal-source/README.md). After editing, run `npm run build:portal` from the repository root and review the source and bundled export together.

## Manual checks

- Test browser approval, rejection, remembering, and revocation with a fresh browser session.
- Verify plugin requests appear after browser approval and that denied plugins stay blocked.
- Test read-only and writable sharing using disposable files, including nested folders and uploads.
- Confirm QR and manual connections preserve HTTP/HTTPS, IP, and port.
- Resize the desktop window to its minimum size; check sidebar content and scrollbars in both themes.
- Test tray quit, server stop, and session expiry.
- On Windows, confirm protected or unknown-status drives cannot become shared roots.
- Validate native packages and actual network transfers on each target operating system before claiming support.

Keep personal notes, captured runtime state, credentials, local test data, and build outputs out of commits. Use documentation addresses and disposable sample files in tests and screenshots.
