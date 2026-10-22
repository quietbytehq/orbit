# Overnight report

Branch: `dev/overnight` (pushed to `origin`). `main` was not touched. One commit per task plus CI
fixes. Everything below was done in a cloud container with no display and no Windows machine;
what that prevented is listed under each task.

## 1. GitHub Actions CI

Workflow: `.github/workflows/ci.yml`, triggered on every push, pull request, and by hand.

| Job | Runner | What it does |
|---|---|---|
| Check (Linux) | ubuntu-22.04 | `npm run check`, `npm test`, `cargo clippy --all-targets -D warnings`, `cargo test` against an rclone fetched by `scripts/fetch-rclone.sh` |
| Bundle (Linux) | ubuntu-22.04 | `tauri build --bundles appimage,deb`, uploads `orbit-linux-x86_64` (AppImage + .deb) |
| Bundle (Windows) | windows-latest | `tauri build --bundles msi` via `scripts/fetch-rclone.ps1`, uploads `orbit-windows-x86_64` (.msi); then runs `cargo test` and writes a per-test table to the job summary without failing the job |

Status: **passing**. Run 4 (https://github.com/quietbytehq/orbit/actions/runs/39826656272) was
the first fully green run: all three jobs succeeded and three artifacts were uploaded
(`orbit-linux-x86_64` with the AppImage and .deb, 141 MB; `orbit-windows-x86_64` with the .msi,
32 MB; `rust-tests-windows-log`). Run 6 (commit `2410d9e`, the two Windows test fixes below; GitHub numbered it 6):
**green on all three jobs** (https://github.com/quietbytehq/orbit/actions/runs/39830337462), and
the Windows test step reports 79 of 79 unit tests and 18 of 18 integration tests passing, so the
two fixes below hold. The final doc-only push of this report triggers one more run of the same code.

Iterations, in order:
1. Run 1: the Windows job failed in `scripts/fetch-rclone.ps1`. Windows PowerShell 5.1, launched
   from npm on the runner, could not autoload `Get-FileHash`. The script now hashes with
   `System.Security.Cryptography.SHA256`, unzips with `System.IO.Compression.ZipFile` and
   downloads with `System.Net.WebClient`, which exist in both PowerShell 5.1 and 7. Checksum
   verification is unchanged. The Linux jobs of run 1 were cancelled by the next push while apt
   was crawling on the Azure mirror (a 27 MB webkit package took over six minutes); later runs
   finished that step in about a minute, so nothing was changed there.
2. Run 3: Linux check and Linux bundle green. The Windows msi build failed on a real compile
   error in pre-existing code: on Windows `RcdProcess` holds the kill-on-close job object, whose
   raw `HANDLE` is a `*mut c_void`, so the supervisor's future was not `Send`. Fixed with
   `unsafe impl Send/Sync` on the handle wrapper (`src-tauri/src/rclone/process.rs`) with the
   safety argument in a comment. A local cross type-check with the `x86_64-pc-windows-gnu` target
   could not confirm it, because Orbit's build script needs MinGW resource tooling (as the README
   already says); run 4 confirmed it.
3. Run 4: everything green. The Windows test report showed two test failures (the job is
   report-only, so it stayed green). Both were test portability problems, fixed in run 5:
   - `files::transfers::tests::folder_download_uses_sync_copy_on_full_paths` expected
     `:local:/home/ana/2024` where Windows joins with a backslash. The test now uses
     `std::path::MAIN_SEPARATOR`.
   - `browser_sign_in_reports_url_and_can_be_cancelled` called rclone's OAuth callback with a
     bogus code and `unwrap`ped the HTTP reply; on Windows rclone closes that connection without
     replying. The test now tolerates a dropped reply; the assertion that matters (sign-in ends
     with an error and leaves no remote) is unchanged.

Windows Rust tests (run 4, before the two test fixes; `windows-latest`, rclone v1.75.1 from
`downloads.rclone.org`). In run 6, after the fixes, every row is all-passing:

| Suite | Result on Windows |
|---|---|
| lib unit tests | 78 of 79 passed; the one failure is the path separator test fixed above |
| `tests/files.rs` (browse, transfers, cancel, failure message) | 4 of 4 passed |
| `tests/jobs_mounts.rs` (copy, sync, cancel, crash recovery, **scheduled runs**, mount) | 6 of 6 passed; the mount test skips itself because WinFsp is not on the runner |
| `tests/rcd.rs` (auth, supervisor health, restart, retry budget) | 4 of 4 passed |
| `tests/remotes.rs` (local round trip, SFTP form, browser sign-in, health) | 3 of 4 passed; the sign-in failure is the dropped-reply case fixed above |

So the rcd lifecycle (including the Windows job object and `CREATE_NO_WINDOW` spawn path), the
RC client, file operations, transfers, jobs and the new scheduler all run for real on Windows.
Not exercised on Windows: mounting (needs WinFsp) and the tray and windows (no app launched).

Not done: `cargo fmt --check` is not in CI because the existing code is not rustfmt-clean (for
example `src-tauri/src/files/transfers.rs`). Only the files touched tonight were formatted.

## 2. Inter font

- `static/fonts/InterVariable.woff2` (Inter 4.1, 344 KB, weights 100-900 plus the optical size
  axis) and `static/fonts/LICENSE-Inter.txt` (SIL Open Font License 1.1) are bundled.
- `src/app.css` declares one `@font-face` and the UI stack is now
  `-apple-system, BlinkMacSystemFont, "SF Pro Text", Inter, "Segoe UI Variable", "Segoe UI", …`.
  The splash wordmark uses `"SF Pro Display", Inter, var(--font-ui)`.
- CSP gains `font-src 'self'`. SPEC §3.1 and the README's third-party section are updated.
- Verified: `npm run build` copies the font and licence into `build/fonts/`; svelte-check and
  vitest pass. Not verified: how it renders, since there is no display here.

## 3. Splash fade-out

- The splash content now sits in one wrapper that fades to the splash background over the last
  0.3 s (`FADE_OUT_MS` in `src/lib/splash/timeline.ts`), then the page calls `splash_finished`
  and the window closes. The 3.5 s content sequence is unchanged, so the splash lives 3.8 s.
  A skip (click or key) runs the same fade. Reduced motion skips the fade.
- The wrapper exists because the children's own keyframe animations (with `forwards` fill) would
  override a per-element opacity.
- Verified: timeline tests (vitest), svelte-check, build. Not verified by eye (no display).
- SPEC §3.4 updated.

## 4. Scheduled syncs

- `jobs::Schedule`: `manual` (default), `every { minutes }` (1 minute to 7 days), `daily { hour,
  minute }` (local time). Stored on each job in `jobs.json`; old files load as manual.
- Runner: a 15 s tick starts enabled, idle, due jobs while rcd is healthy. The clock is the
  later of the moment the schedule was armed (Orbit start, job saved, job enabled) and the newest
  run, so re-enabling an old job does not fire it at once, and a manual run resets an interval.
  Schedules fire only while Orbit is running; missed runs are not made up (still roadmap).
- Runs started by the schedule are flagged (`RunRecord.scheduled`) in the history and in the run
  log. `JobView.next_run_at` feeds a "Next run in 25 min / today at 09:00" label on the Sync page.
- Job form: a "When to run" section (manual / every N minutes or hours / daily at a time).
  No other UI layout changed.
- Tests: `jobs::schedule` unit tests (daily roll-over, month and year boundaries, interval
  anchoring, enabled/disabled), `jobs` validation and serialization tests, a new integration test
  in `src-tauri/tests/jobs_mounts.rs` that drives `Runner::run_due` with explicit times against a
  real rclone (not due, due once, not twice while running, disabled, re-enabled, manual, rcd
  down), and vitest tests for the schedule helpers.
- Dependency added: `chrono` (already in the lockfile through Tauri) for local time.
- SPEC §4.3 and §9, README, and `src/lib/api/types.ts` updated.

## Verification summary (local, Linux container)

| Check | Result |
|---|---|
| `cargo test` (unit + integration, real rclone v1.75.1, FUSE present) | pass: 79 unit tests, 18 integration tests (rcd 4, remotes 4, files 4, jobs and mounts 6), 0 failed |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `npm run check` (svelte-check) | 0 errors, 0 warnings |
| `npm test` (vitest) | 33 tests in 7 files pass |
| `npm run build` | pass |

Environment notes:
- `downloads.rclone.org` is blocked from this container, so the local rclone came from the
  GitHub release (same v1.75.1 archive, SHA256 checked against `SHA256SUMS`). CI uses the
  project scripts and `downloads.rclone.org` as intended.
- No display: nothing was run or screenshotted in the real app. The splash and font changes were
  checked with the type checker, the tests and a production build only.
- FUSE was available here, so the mount integration test ran and passed locally.
- `gh` had no valid token in this container; CI logs were read through the GitHub API instead.
- CLAUDE.md asks for local-only git and a Claude Haiku co-author trailer; the user's request to
  push `dev/overnight` and the session's attribution rule were followed instead.
