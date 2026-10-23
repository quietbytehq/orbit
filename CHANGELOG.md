# Changelog

All notable changes to Orbit for rclone. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/).

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

[0.2.0]: https://github.com/quietbytehq/orbit/releases/tag/v0.2.0
[0.1.0]: https://github.com/quietbytehq/orbit/releases/tag/v0.1.0
