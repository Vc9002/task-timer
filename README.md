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

Sync runs once shortly after startup if configured, and on **Sync now**. There
is no periodic polling. Network requests run outside the SQLite mutex; imports
and the incremental cursor commit atomically. Read-only Todoist API v1 Sync
provides completion/deletion updates. A full-sync task disappearance is marked
unknown, not guessed to be completed. Existing rows and tracked history remain.

Project mappings live in `todoist_projects`; multiple projects may map to one
class. Selecting **Unmapped** hides imported work from Today while retaining
history. Mapping to another class moves the existing task IDs, including their
history, to that class. Unmapped remote tasks are cached locally for later mapping.

API behavior: https://developer.todoist.com/api/v1/#tag/Sync

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
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

## Timer recovery

The UI ticks only while a timer is running. Focus/visibility refreshes read the
persisted timestamps; no timer writes happen every second. Starting another
task offers **Finish Current & Start New**, applied atomically. An unfinished
session at startup offers Continue, Finish Now, Edit (duration), or Discard.

## Planning Today

Use a task's **•••** menu to schedule Today, Tomorrow, or a date. Clearing a
schedule does not hide a task that is still due/overdue. Today includes useful
incomplete descendants and ancestor context at any depth, only in active
classes. Local tasks can be completed directly; complete Todoist tasks in
Todoist, then sync. Row time includes descendants; the header is finished time
tracked today, counted once per session, including work already completed.

## Desktop controls

The menu bar/system tray offers Start/Switch Task, Quick Add, Pause/Resume,
Finish, Open, and Quit. On macOS and Windows, closing the window hides it by
default; Settings can change this to Quit. The tray's Quit always exits.
Tray contents update only on timer transitions, with no elapsed-time polling.

Global shortcuts default to Cmd/Ctrl+Shift+T (task picker),
Cmd/Ctrl+Shift+Space (pause/resume), and Cmd/Ctrl+Shift+F (finish).
Change them in Settings, or leave a shortcut blank to disable it. Cmd/Ctrl+K
opens Quick Add inside the app; due date and planned study date are separate,
and an incomplete local task in the same class can be selected as parent.

Login autostart is off by default. Optional hidden startup applies only to
login launches. A second launch opens the existing process.

Overrun notifications are opt-in and compare the task's direct tracked time
with its estimate at 100%, 125%, or 150%. One sleeping worker waits until the
next threshold or a timer/settings change; it does not poll the database.
Successful OS submission is recorded per session and threshold. Failed sends
retry on the next state change, focus, or launch. OS notification settings may
still prevent display. No reminders are guaranteed while the app is closed.

## Export

Settings exports task CSV, time-history CSV (including class context, minutes,
source, and edits), and JSON containing classes, task hierarchy, sessions, and
Todoist project mappings. JSON is a consistent database snapshot with a format
version and export timestamp. Credentials, settings, and sync caches are
excluded. JSON restore/import is not implemented yet.

CSV tracked minutes are direct finished-session time, so nested tasks are not
counted twice. Unfinished session durations stay blank. Session timestamps are
UTC; the history CSV date uses the exporting machine's local timezone.
