# Orbit for rclone — Specification

Version: 0.1 (target)
Status: Draft, drives the v0.1 build.

## 1. Purpose

Orbit is a desktop GUI for [rclone](https://rclone.org) that makes cloud storage feel like a native
Mac-style app. Users connect cloud accounts, browse and move files, run sync jobs, and mount remotes,
without installing or configuring rclone themselves.

Platforms: Linux (AppImage, .deb) and Windows (.msi). macOS is out of scope for v0.1.

## 2. Architecture

The frontend is SvelteKit running in SPA mode (adapter-static), which is Tauri's recommended setup.

```
┌──────────────────────────── Orbit (Tauri 2 process) ────────────────────────────┐
│  Svelte + TypeScript UI (splash window, main window)                            │
│        │  invoke() commands / listen() events                                   │
│  Rust backend (src-tauri)                                                       │
│    ├─ rclone::process   spawns and supervises `rclone rcd`                      │
│    ├─ rclone::client    JSON RC client (localhost, Basic auth)                  │
│    ├─ remotes / wizard  provider definitions, config create/update/delete       │
│    ├─ files             list, copy, move, delete, mkdir, rename                 │
│    ├─ transfers         queue with bounded concurrency and progress             │
│    ├─ jobs              sync/copy jobs, run history, logs                       │
│    ├─ mounts            mount/unmount, WinFsp detection (Windows)               │
│    ├─ tray              tray icon and status computation                        │
│    └─ settings          app settings (no secrets)                               │
│        │  HTTP, 127.0.0.1:<random port>, Basic auth with random token           │
│  rclone rcd  (bundled binary, pinned version)                                   │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 2.1 rclone lifecycle

- Orbit ships a pinned rclone binary as a Tauri sidecar (`externalBin`). Resolution order at runtime:
  1. Path override from settings (advanced, for developers).
  2. Sidecar next to the Orbit executable.
  3. `rclone` on `PATH`.
- On launch Orbit picks a free loopback port by binding `127.0.0.1:0`, generates a random 32-byte
  token, and starts:
  `rclone rcd --rc-addr 127.0.0.1:<port> --rc-user orbit --config <app-data>/rclone.conf --log-file <app-data>/logs/rclone.log --log-level INFO`
  The RC password is passed through the environment variable `RCLONE_RC_PASS`, never on the command
  line, so it does not appear in the process list.
- Health check: `core/version` every 2 s while running, and once at startup with a 15 s timeout.
- Crash handling: if the process exits unexpectedly, Orbit restarts it with exponential backoff
  (1 s, 2 s, 4 s, max 3 attempts), then reports the disconnected state.
- On app exit Orbit stops `rcd` (`core/quit`, then kill after 3 s).
- If Orbit is force-killed, rcd must not outlive it. On Linux the child is started with
  `PR_SET_PDEATHSIG`, so the kernel signals rcd when Orbit dies. Windows uses a job object with
  kill-on-close (section 6).
- The token and port live in memory only. They are regenerated on every start.

### 2.2 Credentials

- Orbit never writes secrets (tokens, passwords, keys) anywhere except rclone's own config file.
- The wizard keeps entered secrets in memory only until they are handed to `config/create`.
- Orbit's own files (`settings.json`, `jobs.json`, logs) contain no credentials.

### 2.3 Data locations

The app data directory is what Tauri's `app_data_dir()` returns for identifier `com.orbit.rclone`.

| Item        | Linux                                          | Windows                              |
|-------------|------------------------------------------------|--------------------------------------|
| App data    | `~/.local/share/com.orbit.rclone`              | `%APPDATA%\com.orbit.rclone`         |
| rclone.conf | `<app data>/rclone.conf`                       | `<app data>\rclone.conf`             |
| settings    | `<app data>/settings.json`                     | `<app data>\settings.json`           |
| jobs        | `<app data>/jobs.json`                         | `<app data>\jobs.json`               |
| logs        | `<app data>/logs/`                             | `<app data>\logs\`                   |

## 3. Design

### 3.1 Look and feel

- macOS Big Sur/Sonoma inspired: sidebar navigation, translucent (vibrancy-like) panels, 12 px
  corner radius on windows and cards, 8 px on controls.
- Typography: `-apple-system, BlinkMacSystemFont, "SF Pro Text", Inter, "Segoe UI Variable", "Segoe UI", Roboto, Ubuntu, sans-serif`.
  Inter is bundled (`static/fonts/InterVariable.woff2`, SIL Open Font License 1.1, licence file
  alongside), so after SF Pro every platform falls back to the same face. The splash wordmark uses
  `"SF Pro Display", Inter`.
- Light and dark themes. Default follows the system; the setting can be forced to light or dark.
- Translucency: CSS `backdrop-filter` with a solid fallback when it is unavailable.

### 3.2 Color tokens

| Token         | Value     | Use                                     |
|---------------|-----------|-----------------------------------------|
| `--status-synced`       | `#34C759` | Remote or job is up to date   |
| `--status-syncing`      | `#0A84FF` | Transfer or job in progress   |
| `--status-disconnected` | `#FF3B30` | Connection failed or rcd down |

The same colors are used for the tray icons and for the status badges on remotes.

### 3.3 Icons

- App icon: `assets/orbit-icon.png` (1024×1024), generated into `src-tauri/icons/` with `tauri icon`.
- Tray icons: `assets/tray-icons-v2/`. Variants `white` (for dark system theme) and `dark` (for light
  system theme), states `base`, `synced`, `syncing`, `disconnected`. Sizes 16, 20, 22, 24, 32, 48,
  64, 256, plus a multi-size `.ico` per variant and state for Windows. The generator is `gen.py`.
  The tray chooses variant from the system theme and state from rcd health and activity.

### 3.4 Splash

- Shown in its own borderless window (360×360), on a light or dark gradient matching the system theme.
- Sequence over 3.5 s (timings in `src/lib/splash/timeline.ts`):
  - 0.0–1.2 s: the orbit ring draws itself and satellites start orbiting.
  - 1.2–1.8 s: the cloud fades and scales in at the centre.
  - 1.8–2.4 s: the "Orbit" wordmark fades up below the icon.
  - 2.4–3.0 s: the tagline "All your clouds, in one orbit." fades in.
  - 3.0–3.5 s: "Powered by rclone · v0.1" appears at the bottom.
- Text is white on dark and dark on light. Easing is ease-out only, so nothing bounces.
- Skippable by click or any key press.
- Reduced motion: no animation. The final frame is shown for 1.5 s, then the splash finishes.
- The main window opens once the splash has finished and rclone has settled. Settings can turn the splash off; the change applies at the next launch.

## 4. Features for v0.1

### 4.1 Remotes and wizards (M2)

Supported provider wizards:

| Wizard          | rclone type | Auth                         |
|-----------------|-------------|------------------------------|
| Google Drive    | `drive`     | OAuth (rclone's authorize flow) |
| OneDrive        | `onedrive`  | OAuth                        |
| Dropbox         | `dropbox`   | OAuth                        |
| S3-compatible   | `s3`        | Access key and secret; provider preset (AWS, MinIO, Wasabi, Cloudflare R2, Backblaze B2, Other) |
| SFTP            | `sftp`      | Host, port, user, password or key file |

Wizard rules:
- Each wizard is a short sequence of steps written in plain language ("Name this connection",
  "Sign in with Google", "Check it works").
- OAuth uses rclone's own authorize flow via `config/create` with the interactive continuation
  options. rclone runs its local callback server on `127.0.0.1:53682` and Orbit opens the browser
  URL with the opener plugin.
- The last step runs a connection test (`operations/list` on the remote root, with a 20 s timeout)
  and shows a friendly result. A failed test offers "Edit" and "Save anyway".
- Remote names are unique and validated (letters, digits, dashes, underscores; max 40 characters).

Remote management: list, rename (copy config then delete old), edit parameters, delete (with
confirmation). Delete removes only the rclone config section.

### 4.2 File browser (M3)

- Sidebar lists remotes with status badges. Clicking one opens its root.
- Breadcrumb navigation, back and forward, and a path field.
- Listing via `operations/list` (folders first, then name). Columns: name, size, modified.
- Actions: upload (file picker, or drag and drop onto the list), download (folder picker), copy,
  move (to another remote or folder), delete (to trash is not supported by all backends, so the
  confirmation says "permanently delete"), new folder, rename.
- Copy and move between two remotes use `operations/copyfile` for files and `sync/copy` for folders.
  Move uses `operations/movefile` for files and `sync/move` for folders.
- Transfer queue panel (bottom): each item shows name, direction, bytes, speed, ETA, status, and
  a cancel button. Concurrency is a setting (default 3). Cancel uses `job/stop` on the job id.
- Progress comes from `core/stats` filtered by the transfer's group (`_group: "transfer-<id>"`).
  Orbit polls every 500 ms while transfers are active.

### 4.3 Sync jobs and mounts (M4)

Sync jobs:
- Kinds: **Sync** (`sync/sync`, makes destination identical to source, deletes extras) and
  **Copy** (`sync/copy`, never deletes). Each job has a name, source, destination, and kind.
- Jobs are stored in `jobs.json` (names and paths only). Run now, enable or disable, edit,
  delete. Running jobs are asynchronous (`_async: true`); Orbit polls `job/status`.
- Progress: bytes, files, current file, errors count, elapsed time.
- Errors and logs: each run writes `logs/job-<id>-<timestamp>.log` with the summary, every error
  message returned by `core/stats`, and the final result. The job page lists the last 20 runs and
  shows the log of any run.
- Sync is a manual trigger in v0.1. Scheduling is roadmap.

Mounts:
- Mount a remote with `mount/mount`. Linux mounts to a directory (default `~/Orbit/<remote>`).
  Windows mounts to a free drive letter (default: first free letter from `Z:` downward).
- Unmount with `mount/unmount`. Mounted state is read from `mount/listmounts` every 5 s.
- Linux requires FUSE (`fusermount3`). If it is missing, Orbit explains how to install it.
- Windows requires WinFsp. See section 6.

### 4.4 Tray (M1)

- Tray menu: "Show Orbit", "Sync status" (read only: last activity line), "Quit".
- Icon state, computed from the status function in section 4.5:
  - `disconnected`: rcd not running or unhealthy.
  - `syncing`: at least one transfer or sync job is active.
  - `synced`: otherwise.
- Icon variant follows system theme. Orbit listens for theme changes and swaps the variant.
- Closing the main window hides it to the tray. Quit exits the app.

### 4.5 Status function

Pure function, unit tested:

```
fn overall(rcd: RcdHealth, active_transfers: usize, active_jobs: usize) -> OverallStatus
  rcd != Healthy           -> Disconnected
  active_transfers + jobs > 0 -> Syncing
  else                     -> Synced
```

Per-remote badge:
- `disconnected`: last connection test or last operation on that remote failed.
- `syncing`: a transfer or sync job references that remote.
- `synced`: otherwise (last test passed, or it has never been tested, shown with no badge).

### 4.6 Settings (M5)

- Appearance: system, light, dark.
- Transfers: concurrent transfers (1–8, default 3).
- rclone: path override (advanced), log level (`ERROR`, `NOTICE`, `INFO`, `DEBUG`).
- Splash: enabled or disabled (default enabled).
- Settings are stored in `settings.json`. Changes apply immediately, except rclone path and log
  level, which restart rcd.

## 5. Milestones

Each milestone is committed to local git with a message describing it. Nothing is pushed.

| Milestone | Scope | Done when |
|-----------|-------|-----------|
| M1 | Tauri 2 + Svelte TS skeleton; rcd lifecycle (start, stop, health, restart); tray with three states; icon generator extended for 20/22/24/48 and `.ico` | App starts rcd, tray reflects health, `cargo test` covers lifecycle and status function |
| M2 | Five remote wizards; OAuth through rclone; connection test; remote list, rename, edit, delete | Local-backend wizard test passes end to end; OAuth path verified up to the browser step |
| M3 | File browser; upload, download, copy, move, delete, mkdir, rename; transfer queue with progress and cancel | Integration tests against a local remote pass; UI shows live progress |
| M4 | Sync and copy jobs with run history and logs; mount and unmount; WinFsp detection | Sync job test passes; mount test runs on Linux when FUSE is present |
| M5 | Splash; settings page; packaging: AppImage and .deb (Linux), .msi (Windows) | `tauri build` produces AppImage and .deb; Windows build script ready |

## 6. Windows specifics

- rclone and Orbit spawn child processes with `CREATE_NO_WINDOW` so no console flashes.
- rcd is assigned to a job object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, so it ends when Orbit
  does, including after a crash. This is the Windows counterpart of `PR_SET_PDEATHSIG` on Linux.
- WinFsp detection (Rust, `cfg(windows)`): check the registry key
  `HKLM\SOFTWARE\WOW6432Node\WinFsp` (`InstallDir`) and the presence of `winfsp-x64.dll` in
  `System32`. If missing, the mount page shows a prompt with the WinFsp download link and a
  "Check again" button.
- Decision for v0.1: the WinFsp installer is **not** bundled. Bundling it means shipping a
  third-party installer and running it elevated from our MSI, which needs review and a signing
  plan. The prompt-and-link approach is used instead. Revisit in the roadmap.
- The `.msi` is built with the Tauri WiX bundler and can only be built on Windows.

## 7. Testing

- Rust backend: unit tests for the status function, provider definitions, name validation, config
  and settings serialization, the transfer queue, and the job log writer. Integration tests spawn a
  real rclone `rcd` (from the sidecar or `PATH`) and use the `local` backend as a stand-in remote for
  wizard, file, transfer, and job flows. Integration tests skip with a message when no rclone is
  available.
- Frontend: TypeScript with `strict` mode, `svelte-check` in CI. Vitest for pure helpers (byte
  formatting, status mapping, path utilities).
- OAuth providers and real cloud accounts cannot be tested in automation. This is documented in the
  README as manual verification.

## 8. Packaging

- rclone pinned in `scripts/rclone-version.txt`. `scripts/fetch-rclone.sh` (Linux) and
  `scripts/fetch-rclone.ps1` (Windows) download the official build from `downloads.rclone.org`,
  verify the SHA256 from `SHA256SUMS`, and place it in `src-tauri/binaries/` as
  `rclone-<target-triple>` (`.exe` on Windows).
- Linux: `tauri build --bundles appimage,deb`.
- Windows: `tauri build --bundles msi`.
- Version: Orbit 0.1.0. App identifier `com.orbit.rclone`.

## 9. Out of scope for v0.1 (roadmap)

- Scheduled syncs (cron-like schedules, run on login, missed-run handling).
- Bisync (two-way sync) with a conflict model and conflict UI.
- Conflict handling for one-way sync and copy (choose winner, keep both).
- Advanced remote editor (every rclone option, with search and help text).
- Auto-update (Tauri updater, signed release feed).
- Bundled WinFsp installer and code signing for Windows.
- macOS and ARM64 Linux builds.
- Multiple RC instances, shared across windows or accounts.
- Encrypted remotes (crypt) wizard.
