# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

KoalaTalk is a **Windows-only Tauri 2 desktop app** that automates outreach on KakaoTalk PC (`KakaoTalk.exe`). The Rust backend scrapes a logged-in KakaoTalk process to harvest an `Authorization` token, searches public KakaoTalk channels by keyword, adds them as friends via the `pf.kakao.com` web API, and then drives the KakaoTalk window with Win32 messages + clipboard to paste and send text/image blocks. The Svelte 5 frontend is the operator UI on top of those commands.

## Common commands

Package manager is **pnpm**. The workspace package manager field is not set, so use pnpm explicitly. All commands run from the repo root.

- `pnpm install` — install frontend deps (first time / lockfile changes).
- `pnpm tauri dev` — full dev loop: starts Vite on `localhost:1420` and launches the Tauri shell against it. Use this, not `pnpm dev` alone, when testing anything that touches `invoke()` or Tauri APIs.
- `pnpm dev` — Vite only (browser preview). `src/lib/tauri.ts` detects `__TAURI_INTERNALS__` and returns mock data when running in a plain browser, so most UI work can be done without launching Tauri.
- `pnpm build` — `vite build` only; emits SPA to `build/` (consumed by Tauri as `frontendDist`).
- `pnpm tauri build` — produce installer. Bundle config (NSIS/WiX in Korean) lives in `src-tauri/tauri.conf.json`.
- `pnpm check` — `svelte-kit sync && svelte-check`. Use this for TS/Svelte type errors.
- `cargo check -p koalatalk` / `cargo check -p kakaotalk` (run inside `src-tauri/` or `kakaotalk/` respectively) — fast Rust type-check without invoking Tauri build.
- `cargo run --example demo -p kakaotalk` — standalone smoke test of the kakaotalk crate against a running KakaoTalk.exe; useful for debugging memory scraping / search without the Tauri layer.

There are **no test suites** in this repo. Verify behavior end-to-end via `pnpm tauri dev` against a running KakaoTalk PC client.

## Architecture

Three crates / packages, with a strict data-flow direction `kakaotalk → src-tauri → src (SvelteKit)`:

### `kakaotalk/` — domain crate (Rust, Windows-targeted)

Pure library; no Tauri dependency. Public surface is re-exported from `kakaotalk/src/lib.rs`: `KakaoTalk`, `Channel`, `Content`, `User`.

- `KakaoTalk` (`kakaotalk/src/kakaotalk.rs`) is the long-lived client. It caches `pid`, `authorization`, sso token, cookies, and user behind `tokio::sync::Mutex`es. **Cache invalidation matters**: the Tauri layer calls `invalidate()` whenever the user re-checks status, and `check_auth_alive()` is a live ping against `login_token.json` used between batch steps.
- Token extraction (`modules/memory.rs`) walks `KakaoTalk.exe`'s committed private memory regions via `ReadProcessMemory` and matches two regexes (`AUTH_RE` for `Authorization: <token>`, `CRED_RE` for a null-terminated standalone token). Each candidate is validated by hitting `katalk.kakao.com/win32/account/login_token.json` and the first one returning `status == 0` wins. The user is told to open the mail icon in KakaoTalk first because that's what causes the token to be paged into memory.
- Channel search (`search_channels`) fans out 8 parallel requests across all `(f1, f5, f8)` boolean combinations of the `talkfinder-service` `searchOption` to maximize coverage, paginates each, and deduplicates by channel id.
- `Channel::add` calls the `pf.kakao.com/rocket-web` web API as the logged-in web session (cookies from `KakaoTalk::cookies()`, which exchanges the sso token via `accounts.kakao.com/weblogin/login_redirect`).
- `Channel::send` is the Win32 part (`modules/windows.rs`): it finds the KakaoTalk top-level window, types the channel title into the friends-tab search box (`WM_SETTEXT`), opens the chat, pushes each `Content::Text`/`Content::Image` onto the clipboard (`CF_UNICODETEXT` / `CF_DIB`), and posts `Ctrl+V`+`Enter` to the chatroom HWND. This is the brittle part — UI changes in KakaoTalk break it.

### `src-tauri/` — Tauri shell

Thin orchestration layer. `src-tauri/src/lib.rs` defines all `#[tauri::command]`s, the shared `AppState` (`Arc<KakaoTalk>` + a `running` mutex + an extracted-channel map + a persisted `sent_cache: HashSet<String>` + an `AtomicBool` cancel flag), and emits two events to the frontend:

- `log` — `{ level: 'info' | 'ok' | 'skip' | 'err', text }` for the operator log panel.
- `auth_lost` — fired when `ensure_auth` can't refresh the token mid-run; the UI uses this to bounce back to the "check connection" state.
- `done` — final `{ ok, sent, skipped }` after `run_automation` / `send_to_selected`.

Key invariants:
- The `running` mutex serializes `extract_channels`, `run_automation`, and `send_to_selected` — only one long task runs at a time. Cancellation is cooperative via `cancel_flag` polled inside the loop.
- `sent_cache` is persisted to `app_data_dir/sent_channels.json` and consulted before every send so a channel is never messaged twice across sessions. `clear_sent_cache` is the only way to reset it.
- The extracted-channels map is the bridge between the "extract" flow and the "send to selected" flow — the frontend sends back channel ids, the backend looks up the full `Channel` struct (with cookies/auth) from this in-memory map.

### `src/` — SvelteKit 5 frontend (SPA)

`adapter-static` with `ssr = false` (see `src/routes/+layout.ts`); served as a single page. The entire UI is `src/routes/+page.svelte` (~1000 lines, Svelte 5 runes). It talks to the backend only through `src/lib/tauri.ts`, which:
- Wraps every `invoke()` in an `isTauri` check and returns mock data in the browser, so layout/styling can be iterated with just `pnpm dev`.
- Translates the frontend's discriminated-union `MessageBlock` (`text` | `image`) into the `BlockDto` shape the Rust commands expect.
- Owns the `listen()` subscriptions for `log` and `auth_lost` events.

Styling is Tailwind v4 via `@tailwindcss/vite` (no `tailwind.config.js`) + a small shadcn-style component set in `src/lib/components/ui/`. Settings (delay ranges, dedupe toggle) are persisted to `localStorage` under `koalatalk:settings` — that's separate from the Rust-side `sent_channels.json` cache.

### `gui/` — standalone scratchpad (not shipped)

A separate, self-contained SvelteKit project (its own `package.json`, no Tauri). Treat it as a design sandbox; the production frontend is `src/`, not `gui/`.

## Things to know before changing code

- **Windows-only at runtime.** `modules/memory.rs` and `modules/windows.rs` have `#[cfg(windows)]` impls and stub fallbacks for other targets that return `None`/`false`. The whole product is non-functional off Windows; don't try to "fix" the stubs.
- **KakaoTalk version strings are hard-coded.** Headers like `A: win32/26.3.1/ko` and `talk-agent: android/25.11.2` appear in `kakaotalk/src/kakaotalk.rs`. If KakaoTalk updates and the API starts rejecting requests, these are the first thing to bump.
- **Token regexes assume a specific token shape** (`[0-9a-f]{32}[0-9]{15,25}[A-Za-z0-9+/\-_]{60,}`). If extraction starts failing across the board, the regexes in `modules/memory.rs` are the suspect.
- **DTOs use `serde(rename)` for camelCase.** When adding fields that cross the Rust↔TS boundary, mirror the rename in both `src-tauri/src/lib.rs` (`#[serde(rename = "...")]`) and `src/lib/data.ts` — there's no codegen.
- **Commands must guard `running`.** Any new long-running command should follow the pattern in `extract_channels` / `run_automation`: take the `running` mutex, reset `cancel_flag`, run the inner async fn, release the mutex in all paths.
- **`is_chattable()` currently just returns `consult`.** Channels without 1:1 consult enabled are skipped before friend-add; don't assume non-consult channels can be messaged.
- The Tauri window is fixed-width (`width: 825, minWidth: 825, maxWidth: 825` in `tauri.conf.json`). The frontend layout assumes this — don't add wide content without changing the config.
