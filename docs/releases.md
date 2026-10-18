# Automatic releases

[Tauri's official action](https://github.com/tauri-apps/tauri-action) builds Windows x64, macOS Apple Silicon, macOS Intel, and Linux x64 packages. The [Release workflow](../.github/workflows/release.yml) runs after a push or PR merge to `master`, except when every changed file is Markdown (`**/*.md`), under `docs/`, or the root `LICENSE`. Its first successful run publishes `v0.1.3`. Opening a PR or pushing a feature branch does not publish a release.

For README, documentation, or root license updates, commit and push normally without changing the application version. Documentation-only pushes skip the Release workflow; regular CI checks still run. Pushes that also change code or workflow files still trigger the release check. **Run workflow** on `master` remains available for manual releases, including commits containing only documentation.

## Publication

1. Validate that the version agrees in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
2. Skip a version that already has a published release, preserving its assets and edited notes.
3. Pin a new version tag to the triggering commit and create a draft release.
4. Run tests and build native packages on Windows, macOS, and Ubuntu 22.04. Upload them to the draft with `tauri-apps/tauri-action`.
5. Publish and mark the release latest only after all builds succeed and the Windows installer, both macOS disk images, AppImage, and Debian package are present.

The workflow uses GitHub's built-in `GITHUB_TOKEN` with job-scoped `contents: write` permission. No personal access token is needed. Actions must be enabled and repository or organization policy must allow the listed actions and release writes.

## Release another version

Increase the stable application version in all four manifests, update the desktop package's entry in `src-tauri/Cargo.lock`, and update the changelog. Use `npm install --package-lock-only --ignore-scripts` to synchronize npm metadata after changing `package.json`; run a Cargo check to refresh its lockfile. Merge the change into `master`. The release workflow handles the tag, builds, uploads, and publication. Documentation-only changes without a version bump do not replace an existing published release.

Release notes start with package-selection instructions and platform limitations. Maintainers can edit them later on GitHub. Existing draft notes are retained on retries; published release notes are never overwritten by this workflow.

## Recover a failed build

A failed platform build leaves the release as a draft. Open the failed Actions run and choose **Re-run failed jobs** after a transient failure, or **Re-run all jobs** to rebuild the same commit. Successful assets remain attached; publication waits for the complete build matrix.

If a source fix is needed, bump the version and merge the fix; tags are never silently moved to a different commit. An obsolete draft can be reviewed and removed manually. **Run workflow** is also available on `master` for a current version that has not been published. Dispatches on other branches are skipped.

## Scope

The workflow publishes unsigned packages and does not configure automatic in-app updates, Windows code signing, or Apple notarization. macOS and Linux remain experimental until runtime validation is complete. Release creation does not make a private repository public; download access follows repository visibility.
