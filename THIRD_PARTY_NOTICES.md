# Third-party components

The desktop application uses Tauri, Rust crates, React, Vite, TypeScript, and other open-source libraries. Exact versions are recorded in `package-lock.json`, `src-tauri/Cargo.lock`, and `portal-source/package-lock.json`.

The browser portal is adapted from [ShareX Android](https://github.com/akanshSirohi/ShareX). See [NOTICE.md](NOTICE.md) and [the portal README](portal-source/README.md) for attribution.

Bundled browser players, document viewers, fonts, and their accompanying notices live under `portal-source/public/` and the exported `src-tauri/portal/` tree. Preserve their license files and embedded notices. Dependency packages retain their own licenses; the repository's AGPL license does not replace those terms.

Before distributing binaries, review the licenses of the exact resolved dependencies and include required notices and source offers. This document is an attribution index, not an exhaustive dependency license inventory.
