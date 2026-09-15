# Task Timer

A lightweight, local-first desktop app for tracking coursework and the time
spent on it. Open the app, see today's work grouped by class, press Start,
and know where your time went.

## What it does

- **Today** screen: tasks and subtasks due today or overdue, grouped by class.
- **Timer**: start/pause/resume/finish per task, one active timer at a time,
  timestamp-derived (not a naive per-second counter), survives app restarts
  and system sleep.
- **History**: a day-by-day log of every session, with manual duration
  correction.
- **Analytics**: tracked time today / this week / this month, by class.
- **Todoist** (optional): read-only sync of projects and tasks into classes
  and tasks, mapped by Todoist's stable project ID (never by name).

Local tasks work without any Todoist account.

## Architecture

- **Shell**: Tauri 2 (native OS webview, no bundled browser engine).
- **Frontend**: Svelte 5 + TypeScript, SvelteKit in static-adapter (SPA) mode.
- **Backend**: Rust. Owns SQLite, the timer state machine, and Todoist sync.
  The frontend talks to it only through typed `invoke()` commands — no local
  HTTP server.
- **Database**: SQLite (WAL mode), one file in the OS app-data directory.
  Migrations are plain versioned `.sql` files tracked in a
  `schema_migrations` table.
- **Secrets**: the Todoist API token lives in the OS keychain (via the
  `keyring` crate), never in the SQLite database or on disk in plaintext.

See `src-tauri/src/` for the Rust side (`db/`, `timer/`, `todoist/`,
`commands/`) and `src/` for the Svelte frontend (`routes/`, `lib/`).

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [Node.js](https://nodejs.org/) 20+

## Development

```bash
npm install
npm run tauri dev
```

## Building

```bash
npm run tauri build
```

Release builds use `codegen-units = 1`, `lto = true`, and `panic = "abort"`
(see `src-tauri/Cargo.toml`) to keep the binary small.

## Database

SQLite file location: the OS app-data directory for
`com.vincentc9002.tasktimer` (e.g. `~/Library/Application Support/...` on
macOS). Migrations run automatically on startup; nothing to run by hand.

## Todoist setup

1. Get a personal API token from Todoist: Settings → Integrations →
   Developer.
2. In the app, go to Settings → Todoist and paste the token. It's validated
   against the API before being stored in your OS keychain.
3. Run "Sync now" to pull in your projects.
4. Map each Todoist project to a class in the Settings → Project mappings
   list. Only tasks in mapped projects are imported.
5. Re-run "Sync now" any time; re-syncing never creates duplicates.

Todoist integration is currently read-only: changes made in the app are not
pushed back to Todoist.

## Testing

```bash
cd src-tauri
cargo test --lib
```

Covers the timer state machine (start/pause/resume/finish/cancel, multiple
pauses, crash/restart recovery, manual duration correction, day-boundary
crossing) and Todoist sync idempotency (re-sync doesn't duplicate, renamed
tasks update in place, parent/child relationships are preserved, tasks from
unmapped projects are skipped).

```bash
cargo clippy --all-targets
cargo fmt --check
```
