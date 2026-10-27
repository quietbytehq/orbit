# Changelog

All notable changes to Orbit for rclone. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/).

## [1.0.1] - 2026-10-27

### Changed
- The snap is published as `orbit-rclone` (the store name `orbit` was taken). Its command is
  `orbit-rclone`, so `/snap/bin/orbit-rclone` launches it; the app is still "Orbit for rclone".
  Drives need `sudo snap connect orbit-rclone:fuse-support` until the store auto-connects it.
- The snap declares the session-bus name used to keep a single Orbit running, so a second launch
  inside the snap hands over to the first instead of starting again.

## [1.0.0] - 2026-10-26

First stable release. Everything in 0.2.x, plus:

### Changed
- Licence changed from MIT to the Apache License 2.0 (LICENSE, NOTICE). rclone remains MIT and is
  bundled unmodified.
- Version number promoted to 1.0.0; the splash footer and the About information read the version
  from the build instead of a hard-coded string.

### Added
- Snap package (`snap/snapcraft.yaml`, core24, strict confinement) built in CI and attached to
  releases. Drives need `sudo snap connect orbit:fuse-support` until the store auto-connects it.
- The start-at-sign-in entry is named `orbit` on every platform (and is the snap's own autostart
  entry inside the snap).

## [0.2.1] - 2026-10-23

### Fixed
- Only one Orbit runs at a time. A second launch (by hand or from the start-at-sign-in entry)
  hands over to the running one, which opens its window unless the launch was minimized, and
  exits. Previously several copies could pile up with one tray icon each.
- "Quit Orbit" in the tray now really quits: it asks for confirmation when a transfer or backup is
  running, stops them, disconnects drives and folders, stops the storage engine, removes the tray
  icon and ends the process, each step bounded by a timeout and the whole by a watchdog. Asking
  the engine to quit no longer waits the full request timeout.
- The tray menu always offers "Open Orbit" and "Quit Orbit".

### Added
- `orbit --quit` asks a running Orbit to quit without a prompt, for scripts.
- A scripted Linux smoke test (`scripts/smoke-quit.sh`, run in CI under a virtual display) checks
  single instance and clean quit end to end, and an integration test checks that quitting cancels
  work, stops the engine and frees its port.

## [0.2.0] - 2026-10-23

### Added
- Plain-language UI for people who have never heard of rclone: "Cloud storage", "Backup & sync"
  (Copy / Mirror), "Drives" on Windows and "Folders" on Linux, with platform words (drive letter,
  File Explorer, This PC). Every string lives in one module per side, with tests that reject jargon.
- Error messages that say what happened and what to do next, with the technical text behind a
  "Show details" toggle (also on failed connection checks, transfers and backup runs).
- Lucide icons across the app: sidebar, toolbar buttons (icon-only with a tooltip when narrow),
  provider glyphs on storage cards and file-type icons in the browser.
- Scheduled backups: every N minutes or hours, or daily at a time, while Orbit is open, with a
  "next run" label and scheduled runs marked in the history.
- Network share (SMB / Samba) wizard: server, share, username, password, domain, port, with the
  share used as the folder to open first.
- "Start Orbit when I sign in" (starts minimized to the tray) and remembered drives and folders
  with a per-mount "connect automatically when Orbit starts" toggle.
- Intro animation setting: Full (about 5.5 s), Short (1.5 s, default) or Off. The full intro plays
  once on the first launch after an install or update, and the content fades out before the main
  window appears.
- Bundled Inter font (SIL Open Font License) as the fallback after SF Pro on every platform.
- GitHub Actions CI: svelte-check, vitest, clippy and the Rust tests on Linux; AppImage, .deb and
  .msi builds with the Rust tests also run on Windows.
- A tooltip on the size column for documents that live only in the cloud (such as Google Docs).

### Changed
- App icon trimmed to a clean squircle on a transparent background; all sizes regenerated.
- Bundle files are named `orbit_<version>_…` (AppImage, .deb, .msi); the display name is still
  "Orbit for rclone".
- Windows: the storage engine's handle is marked thread-safe so the app builds there; two Rust
  tests made portable (path separators, a dropped OAuth callback reply).
- The Windows rclone fetch script no longer depends on PowerShell module autoloading.

### Fixed
- A race in the backup store where a finishing run and an edit could save at the same time and
  one of them failed.

## [0.1.0] - 2026-10-22

Initial release.

### Added
- Bundled rclone run as a supervised background daemon on the loopback interface: start, health
  check, restart with backoff, clean stop, and no orphaned process on a crash (PR_SET_PDEATHSIG on
  Linux, a kill-on-close job object on Windows).
- Tray icon with synced, syncing and disconnected states, following the system theme.
- Splash screen with the orbit animation, skippable and reduced-motion aware.
- Wizards for Google Drive, OneDrive and Dropbox (browser sign-in through rclone), S3-compatible
  storage and SFTP, with a connection test, edit, rename and delete.
- File browser: list, folders, multi-select, new folder, rename, delete, upload (picker or drag and
  drop), download, copy and move between remotes, with a transfer queue showing progress, speed,
  ETA and cancel.
- Sync and copy jobs with run history (last 20 runs), per-run logs and error counts.
- Mount and unmount: folders on Linux (FUSE 3) and drive letters on Windows (WinFsp, prompted for,
  not bundled).
- Settings: theme, concurrent transfers, custom rclone path, log level, splash on or off.
- Packaging as AppImage and .deb on Linux and .msi on Windows.

[1.0.0]: https://github.com/quietbytehq/orbit/releases/tag/v1.0.0
[0.2.1]: https://github.com/quietbytehq/orbit/releases/tag/v0.2.1
[0.2.0]: https://github.com/quietbytehq/orbit/releases/tag/v0.2.0
[0.1.0]: https://github.com/quietbytehq/orbit/releases/tag/v0.1.0
