# Architecture

ShareX Desktop consists of a native host, its desktop interface, and a browser portal adapted from [ShareX Android](https://github.com/akanshSirohi/ShareX).

| Location | Responsibility |
| --- | --- |
| `src/` | React and TypeScript desktop interface; Tauri command calls and events |
| `src-tauri/src/lib.rs` | Native lifecycle, commands, events, and tray integration |
| `src-tauri/src/model.rs` | Shared settings and runtime models |
| `src-tauri/src/storage.rs` | Shared roots, filesystem operations, and Windows volume checks |
| `src-tauri/src/network.rs` | Network addresses and connection helpers |
| `src-tauri/src/server.rs` | Axum HTTP/HTTPS server, browser access, transfers, and portal routes |
| `src-tauri/src/plugins.rs` | Plugin installation, access, and runtime support |
| `portal-source/` | Editable browser portal |
| `src-tauri/portal/` | Committed static portal bundled with the native application |
| `scripts/sync-portal.mjs` | Replaces the bundled portal with a completed static export |

The desktop interface controls the native server through Tauri IPC. Browser clients connect directly to the advertised local address. Browser approval and plugin approval are separate decisions; stopping a session ends access to its server. Optional passwords, expiry, remembered clients, and revocation are managed by the native host.

Filesystem requests resolve against configured shared roots. The server enforces access and root containment before performing operations. Writable access is optional. Settings and runtime state belong in the per-user application data directory, outside the checkout; sharing roots are user-selected locations.

The portal remains a separate frontend so phone and desktop clients can use a normal browser. Changes to its source require rebuilding and synchronizing the static export before packaging the desktop application.
