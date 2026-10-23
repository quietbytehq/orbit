# Orbit for rclone

A desktop app for [rclone](https://rclone.org). Connect cloud storage, browse and move files, run sync and copy jobs, and mount remotes as folders or drives, without installing or configuring rclone yourself.

Orbit ships its own rclone and runs it as a background daemon (`rclone rcd`) on the loopback interface. The app talks to it through rclone's JSON remote-control API.

- **Platforms:** Linux (AppImage, `.deb`) and Windows (`.msi`).
- **Stack:** Tauri 2 (Rust), SvelteKit in SPA mode, TypeScript.
- **Design:** see [SPEC.md](SPEC.md). Contributor rules are in [CLAUDE.md](CLAUDE.md).

## What works (v0.2)

| Area | Status |
|---|---|
| rclone lifecycle: start, health check, restart with backoff, clean stop, no orphan on crash | Done, tested with a real rclone |
| Tray with synced, syncing and disconnected icons; white or dark set by system theme | Done (Linux run; Windows icons compile-checked only) |
| Splash: full intro (about 5.5 s) on first launch and after an update, short (1.5 s) otherwise, or off; skippable; reduced-motion aware | Done; timings tested, not reviewed by eye |
| Plain-language UI with Windows or Linux wording, lucide icons, error details behind "Show details" | Done; checked in screenshots of the built UI with fake data |
| Remote wizards: Google Drive, OneDrive, Dropbox (browser sign-in), S3-compatible, SFTP, network shares (SMB) | Done. Browser sign-in is tested up to the callback only, see below. SMB is tested for parameter handling and failure messages, not against a live server |
| Connection test, edit, rename, delete, status badges (up to date, syncing, disconnected) | Done |
| File browser: list, folders, multi-select, new folder, rename, delete | Done, tested with rclone's `alias` backend |
| Upload (picker or drag and drop), download, copy and move between remotes | Done, tested |
| Transfer queue with progress, speed, ETA and cancel; concurrency from settings | Done, tested |
| Sync and copy jobs with run history (last 20 runs), logs, error counts | Done, tested |
| Scheduled jobs: every N minutes or hours, or daily at a time, while Orbit is open | Done, tested against a real rclone |
| Mount and unmount (Linux via FUSE 3; Windows via WinFsp with a drive letter) | Done on Linux, tested with a real FUSE mount. Windows not run |
| Settings: theme, concurrent transfers, custom rclone path, log level, intro animation, start at sign-in | Done; start at sign-in not run on a real desktop |
| Remembered drives and folders, reconnected automatically at startup | Done; store tested, reconnection not run on a real desktop |
| Packaging: AppImage and `.deb`, named `orbit_<version>_…` | Built in CI on every push |
| Packaging: `.msi`, named `orbit_<version>_…` | Built in CI on Windows on every push; not installed on a real machine |

Out of scope for v0.1 (see SPEC §9): cron-style schedules and missed-run catch-up, bisync, conflict handling, the advanced remote editor, auto-update, macOS.

## Not verified yet

Please read this before relying on v0.1:

- **Real cloud accounts.** Google Drive, OneDrive and Dropbox sign-in was not completed against live accounts, since none were available. The flow was tested up to rclone's local callback: a bogus code makes rclone report the failure, the call returns, and no stub remote is left behind. S3 and SFTP were tested for their parameter handling and their failure messages, not against live servers.
- **Windows at runtime.** The Rust tests (including the integration tests against a real rclone, the kill-on-close job object and the no-console spawn path) run on Windows in CI and pass, and the `.msi` builds there. The app itself has not been launched on a Windows desktop: the tray, windows, WinFsp mounting, start-at-sign-in and the installer are unverified by eye.
- **Look and feel.** Pages were reviewed from screenshots of the built UI rendered in Chromium with fake data, not in the real Tauri window.
- **GNOME tray.** On GNOME, tray icons need the AppIndicator extension. Without it, the tray icon will not appear, though the app still works.

## Build from source

### Prerequisites

Linux (Debian or Ubuntu):

```bash
sudo apt install build-essential pkg-config curl unzip \
  libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev \
  libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev \
  fuse3   # for mounting remotes
```

Then install [Rust](https://rustup.rs) (stable) and Node.js 22 or newer.

Windows:

- Rust with the MSVC toolchain, Node.js 22 or newer, and PowerShell.
- WebView2 (already installed on current Windows 10 and 11).
- [WinFsp](https://winfsp.dev) to mount remotes. Orbit prompts for it and does not bundle it.

### Steps

```bash
npm install
scripts/fetch-rclone.sh            # Windows: scripts\fetch-rclone.ps1
npm run tauri dev                  # run the app
```

The fetch script downloads the rclone version pinned in `scripts/rclone-version.txt` from `downloads.rclone.org`. It checks the archive against the published SHA256SUMS before installing it into `src-tauri/binaries/`. Those binaries are not committed.

### Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # unit and integration tests
npm run check                                     # svelte-check, strict TypeScript
npm test                                          # vitest
```

Integration tests run a real `rclone rcd`. They use the sidecar in `src-tauri/binaries/`, or an rclone on `PATH`, or the path in `ORBIT_TEST_RCLONE`. They skip with a message when none is found. Mount tests skip when FUSE is missing.

### Packaging

```bash
npm run bundle:linux     # fetch rclone, then AppImage and .deb
npm run bundle:windows   # fetch rclone for Windows, then .msi (run on Windows)
```

Outputs go to `src-tauri/target/release/bundle/`, named `orbit_<version>_…` (the scripts rename Tauri's "Orbit for rclone_…" files; the display name is unchanged).

### Regenerating icons

- App icon: `npx tauri icon assets/orbit-icon.png` writes into `src-tauri/icons/`.
- Tray icons: `assets/tray-icons-v2/gen.py` makes all sizes and the `.ico` files. It needs `cairosvg`, `numpy` and `Pillow`. Run it from inside `assets/tray-icons-v2/`.

## Where things live

| Path | Purpose |
|---|---|
| `src-tauri/src/rclone/` | Process supervision, RC client |
| `src-tauri/src/remotes/` | Provider wizards, rclone config operations, sign-in |
| `src-tauri/src/files/` | Browsing, transfer queue |
| `src-tauri/src/jobs/` | Sync and copy jobs, run history |
| `src-tauri/src/mounts.rs` | Mounting, FUSE and WinFsp checks |
| `src-tauri/tests/` | Integration tests against real rclone |
| `src/routes/` | Screens (splash, overview, remotes, browse, sync, mounts, settings) |
| `src/lib/` | Typed API layer, stores, components, helpers |

## Where your data lives

Everything is under the Tauri app data folder: `~/.local/share/com.orbit.rclone` on Linux and `%APPDATA%\com.orbit.rclone` on Windows.

- `rclone.conf` is rclone's own config. **It is the only place credentials are stored.** Orbit does not keep its own copy.
- `settings.json`, `jobs.json` and `mounts.json` hold preferences, backup definitions and remembered drives, with no secrets.
- `logs/` holds the rclone log and one log per job run.

## Known limitations

- Drives and folders end when Orbit quits. Ones marked "connect automatically" come back once Orbit is running and its engine is up.
- Scheduled jobs run only while Orbit is open. A run that was due while Orbit was closed is not made up, and nothing runs on login.
- Deletes are permanent. Orbit does not use a provider's trash.
- A job's error list shows each new error as it appears, not every error from the run.
- Browser sign-in uses rclone's shared OAuth client IDs, so providers may rate-limit heavy use.
- The rclone that ships with Orbit is what it runs by default. If you choose a custom rclone, a snap-packaged build may not be able to write to the app data folder.
- The WinFsp installer is not bundled. See SPEC §6.

## Third-party

- [rclone](https://rclone.org), MIT licence, bundled as a sidecar binary.
- [Inter](https://rsms.me/inter/) typeface, SIL Open Font License 1.1, bundled in `static/fonts/` with its licence file.

## Roadmap

See SPEC §9.
