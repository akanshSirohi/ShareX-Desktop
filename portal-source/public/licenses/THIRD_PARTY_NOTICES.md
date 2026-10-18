# Third-party components

The desktop application uses Tauri, Rust crates, React, Vite, TypeScript, and other open-source libraries. Exact versions are recorded in `package-lock.json`, `src-tauri/Cargo.lock`, and `portal-source/package-lock.json`.

The browser portal is adapted from [ShareX Android](https://github.com/akanshSirohi/ShareX). See [NOTICE.md](NOTICE.md) and [the portal README](portal-source/README.md) for attribution.

Bundled browser players, document viewers, fonts, and their accompanying notices live under `portal-source/public/` and the exported `src-tauri/portal/` tree. Preserve their license files and embedded notices. Dependency packages retain their own licenses; the repository's AGPL license does not replace those terms.

The following additional license files are preserved verbatim under `portal-source/public/licenses/` and `src-tauri/portal/licenses/`:

| Component | License | Included files |
| --- | --- | --- |
| movi-player | Apache-2.0 | `movi-player-LICENSE` |
| PDF.js (`pdfjs-dist`) | Apache-2.0 | `pdfjs-dist-LICENSE` |
| omni-doc-viewer | MIT | `omni-doc-viewer-LICENSE`, `omni-doc-viewer-NOTICE` |

The same directory includes the project's `LICENSE`, `NOTICE.md`, and this attribution index. The portal's prebuild script refreshes these copies from the repository and installed packages so they survive rebuilding and desktop export synchronization. PDF.js font, character-map, ICC profile, and decoder licenses remain beside their assets under `players/pdf/`.

Before distributing binaries, review the licenses of the exact resolved dependencies and include required notices and source offers. This document is an attribution index, not an exhaustive dependency license inventory.
