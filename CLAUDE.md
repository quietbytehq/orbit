# CLAUDE.md — Orbit for rclone

Read `SPEC.md` first. It is the source of truth for scope, behavior, and milestones.
If code and spec disagree, fix the code or update the spec in the same change. Do not let them drift.

## Stack

- Tauri 2 (Rust backend in `src-tauri/`), Svelte 5 + TypeScript + Vite in `src/`.
- rclone is bundled as a sidecar and run as `rclone rcd`. The frontend never talks to rcd directly.
  All RC calls go through the Rust `rclone::client` module.

## Layout

```
src/                  Svelte UI (typed). Pure helpers in src/lib/util, tested with vitest.
src/lib/api/          TypeScript types mirroring Rust serde structs; invoke() wrappers.
src-tauri/src/        Rust backend modules as listed in SPEC.md §2.
src-tauri/tests/      Integration tests (need a real rclone; they skip if none is found).
src-tauri/binaries/   Sidecar rclone binaries, created by scripts/fetch-rclone.*. Not committed.
assets/               Source art: app icon, tray icon set and gen.py.
scripts/              rclone fetch scripts and version pin.
```

## Rules

- **Credentials:** never write secrets outside rclone's config file. Orbit files hold no secrets.
  Do not log config values, tokens, or RC passwords, even at debug level.
- **RC password:** pass it via the `RCLONE_RC_PASS` environment variable, never as an argument.
- **Types stay in sync:** when a Rust serde struct changes, update `src/lib/api/types.ts` in the same
  commit.
- **Tests:** every Rust module with logic has unit tests. Integration tests use the `local` backend.
  Frontend helpers have vitest tests. Run `cargo test`, `npm run check`, and `npm test` before a
  milestone commit.
- **Errors:** Tauri commands return `Result<T, AppError>`. `AppError` serializes to
  `{ kind, message }`. Do not panic in command handlers.
- **Platform code:** Windows-only code goes behind `#[cfg(windows)]`. Linux must compile and run
  without Windows-only dependencies. Use `CREATE_NO_WINDOW` when spawning rclone on Windows.
- **Design:** follow the tokens in `SPEC.md` §3. Status colors are fixed: green `#34C759`, blue
  `#0A84FF`, red `#FF3B30`. Respect `prefers-reduced-motion` and light and dark themes.
- **Icons:** edit `assets/tray-icons-v2/gen.py` rather than hand-editing generated PNG or SVG files.
  Regenerate with the script; it writes into its own folder.
- **Scope:** do not add features from the roadmap (SPEC §9) to v0.1.

## Commands

```
scripts/fetch-rclone.sh          # download pinned rclone for this platform into src-tauri/binaries
npm install
npm run tauri dev                # run the app
cargo test --manifest-path src-tauri/Cargo.toml
npm run check                    # svelte-check, strict TypeScript
npm test                         # vitest
npm run tauri build -- --bundles appimage,deb
```

## Git

- Local git only. Do not push, do not add remotes, do not open PRs.
- One commit per milestone plus fixes as needed. Messages are imperative, short subject line.
- Commit trailer: `Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>`.
