---
last_mapped_commit: 3628ccdef63f6dfb4e6d6b20ffce30c3d515cda3
last_mapped_at: 2026-09-20
---
# External Integrations

**Analysis Date:** 2026-09-20

## APIs & External Services

**Todoist:**

- Todoist API v1 - Task and project synchronization (optional, opt-in)
  - Endpoint: `https://api.todoist.com/api/v1`
  - SDK/Client: reqwest (Rust HTTP client with blocking mode)
  - Auth: Personal API token (user-provided, validated before storage)
  - Purpose: Read-only sync of Todoist projects and tasks into local classes and tasks
  - Scope: Projects and items; bidirectional sync for task completion only

**GitHub:**

- GitHub Releases - Application update distribution
  - Endpoint: `https://github.com/Vc9002/task-timer/releases/latest/download/latest.json`
  - Client: @tauri-apps/plugin-updater
  - Purpose: Check and download application updates

## Data Storage

**Databases:**

- SQLite (primary)
  - File location: OS app-data directory for `com.vincentc9002.tasktimer`
  - File name: `task-timer.sqlite`
  - Mode: WAL (Write-Ahead Logging) enabled via pragma
  - Client: rusqlite (Rust SQLite driver with bundled SQLite)
  - Migrations: 14 versioned migrations tracked in `schema_migrations` table
  - Purpose: Local storage for tasks, sessions, timer state, classes, exams, study blocks, templates, recurring tasks, task dependencies, task notes, and integration metadata

**File Storage:**

- Local filesystem only
  - No cloud storage integration
  - Backup/export via JSON export command

**Caching:**

- In-memory Todoist token cache
  - Cached via `OnceLock<Mutex<Option<...>>>` to prevent repeated OS keychain prompts
  - Invalidated when token is explicitly changed or removed

## Authentication & Identity

**Auth Provider:**

- Custom/Manual
  - Implementation: User-provided personal API token from Todoist
  - Token storage: OS keychain (service: `com.vincentc9002.tasktimer`, key: `todoist_api_token`)
  - Validation: Token validated against Todoist API before storage
  - No local user authentication required (single-user desktop app)

**Keychain Integration:**

- Platform: Native OS keychain via `keyring` crate
  - macOS: Keychain Access
  - Linux: Secret Service
  - Windows: Credential Manager
- Service identifier: `com.vincentc9002.tasktimer`
- Stored credentials: Todoist personal API token only (never stored in SQLite)

## Monitoring & Observability

**Error Tracking:**

- Not integrated (desktop app, local data)

**Logs:**

- Console output (stderr) for errors and debugging
- No persistent log aggregation

## CI/CD & Deployment

**Hosting:**

- GitHub Releases (binary distribution)
- Native downloads for macOS, Linux, Windows

**CI Pipeline:**

- GitHub Actions
  - **Frontend job:** Ubuntu runner
    - Node.js 20 + npm cache
    - Svelte type checking via `svelte-check`
    - Vite build
    - npm audit (high severity threshold)
  
  - **Rust job:** Ubuntu runner
    - Tauri Linux dependencies installed
    - Cargo cache
    - cargo fmt check
    - cargo clippy with strict warnings-as-errors
    - cargo test (library tests only)
    - cargo audit (security scanning)
  
  - **macOS job:** macOS latest runner
    - Full build and test on native platform
    - Node.js 20 + npm cache
    - Cargo cache
    - Full Rust checks (fmt, clippy, tests)
  
  - **Release job:** macOS runner (on version tags)
    - Triggered by: push to tags matching `v*`
    - Uses: tauri-action for signed release builds
    - Secrets required: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `GITHUB_TOKEN`

**Dependency Management:**

- Dependabot automated updates
  - npm packages: weekly schedule
  - Cargo (Rust) packages: weekly schedule
  - GitHub Actions: weekly schedule

## Environment Configuration

**Required env vars:**

- No runtime environment variables required
- Development only: `TAURI_DEV_HOST` (optional, for custom dev server host)

**Secrets location:**

- OS keychain for Todoist API token (recommended, used in production)
- No `.env` file used (app data stored in SQLite, secrets in keychain)
- GitHub Actions secrets for release builds:
  - `TAURI_SIGNING_PRIVATE_KEY`
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  - `GITHUB_TOKEN`

**App data directory:**

- macOS: `~/Library/Application Support/com.vincentc9002.tasktimer/`
- Linux: `~/.local/share/com.vincentc9002.tasktimer/`
- Windows: `%APPDATA%\com.vincentc9002.tasktimer\`

## Webhooks & Callbacks

**Incoming:**

- None

**Outgoing:**

- Todoist Sync API - Pull-based (no webhooks)
  - `GET /sync` - Incremental sync with cursor token
  - `POST /sync` - Full sync request with optional command batch
  - Used for: Task completion updates sent in durable outbox queue

**Integration Outbox:**

- Durable queue for failed Todoist commands
  - Stored in `integration_outbox` table
  - Retry mechanism: Reprocessed on next sync or explicit "Sync now"
  - Status tracking: pending, sending, completed
  - Commands: task completion (item_complete)

---

*Integration audit: 2026-09-20*
