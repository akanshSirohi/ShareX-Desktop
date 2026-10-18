# ShareX browser portal

This is the browser interface adapted from [ShareX Android](https://github.com/akanshSirohi/ShareX). The desktop application serves a bundled static export from `src-tauri/portal/`; a production deployment does not need a Next.js server.

## Build for desktop

From the repository root:

```sh
npm --prefix portal-source ci
npm run test:portal
npm run build:portal
```

This builds `portal-source/out/`, then synchronizes it into `src-tauri/portal/`. Commit source edits and the updated export together. From this directory, `npm run build:assets` performs the same build and desktop synchronization. It does not write to the Android application's assets.

## Development

Start the desktop sharing server, then point the development proxy at it. In PowerShell:

```powershell
$env:SHAREX_DEVICE_URL = 'http://127.0.0.1:6060'
npm run dev
```

On macOS or Linux:

```sh
SHAREX_DEVICE_URL=http://127.0.0.1:6060 npm run dev
```

Use the actual port shown by the desktop application. Open `http://localhost:3000` and approve the browser in ShareX Desktop. Browser access and plugin access require separate approvals. The proxy is for development; production uses relative URLs.

## Assets and tests

Build scripts copy browser player resources, PDF workers, fonts, and decoders into `public/players/` so the exported portal can load them locally. Preserve included license notices. `web-ui.json` supplies the portal folder version; build metadata is generated before development and builds.

Run `npm test` in this directory. The exported `copyAssets` helper retains tests for upstream deterministic ZIP packaging; its command-line entry point synchronizes the desktop export.

## License

The ShareX portal and its desktop adaptations are licensed under [AGPL-3.0-or-later](../LICENSE). Preserve the original ShareX attribution in [NOTICE.md](../NOTICE.md). The prebuild script includes project notices and bundled player/viewer licenses in `public/licenses/`, which is carried into the static export. See [third-party notices](../THIRD_PARTY_NOTICES.md) for component-specific terms.

## Theme attribution

Zen Inspired uses [Bikash's theme](https://tweakcn.com/themes/cmlm03etv000204lh15608kec). Additional presets use [2077](https://tweakcn.com/themes/cmlecx2br000004if3m471xxd), [hex](https://tweakcn.com/themes/cmninq0c3000604l25wvb3xgh), [Purple Rain](https://tweakcn.com/themes/cmlh0vbnd000004l112kx8a0l), [Light Green](https://tweakcn.com/themes/cmlhfpjhw000004l4f4ax3m7z), and [Claude +](https://tweakcn.com/themes/cmdght103000n04lh3e2ae93r). The default uses shadcn neutral tokens. Theme tokens ship locally.
