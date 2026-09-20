---
last_mapped_commit: 3628ccdef63f6dfb4e6d6b20ffce30c3d515cda3
last_mapped_at: 2026-09-20
---
# Codebase Structure

**Analysis Date:** 2026-09-20

## Directory Layout

```
task-timer/
├── src/                    # Frontend source (SvelteKit)
│   ├── lib/                # Shared utilities and components
│   │   ├── api.ts          # Tauri IPC command bindings
│   │   ├── stores/         # Svelte stores (reactive state)
│   │   ├── components/     # Reusable Svelte components
│   │   ├── format.ts       # Formatting utilities
│   │   ├── priority.ts     # Priority level helpers
│   │   ├── taskMetadata.ts # Task metadata types
│   │   └── app.css         # Global styles
│   └── routes/             # SvelteKit routes & pages
│       ├── +layout.svelte  # Root layout (app shell)
│       ├── +layout.ts      # Root layout data
│       ├── +page.svelte    # Default page (redirects to /inbox)
│       ├── inbox/          # Inbox view route
│       ├── week/           # Weekly planner route
│       ├── calendar/       # Calendar view route
│       ├── classes/        # Class management route
│       ├── history/        # Task history route
│       ├── analytics/      # Analytics dashboard route
│       ├── semester/       # Semester overview route
│       └── settings/       # Settings page route
├── src-tauri/              # Backend source (Rust/Tauri)
│   ├── src/
│   │   ├── main.rs         # Entry point (delegates to lib)
│   │   ├── lib.rs          # Module declarations & Tauri setup
│   │   ├── commands/       # Tauri IPC command handlers
│   │   │   ├── tasks.rs    # Task CRUD operations
│   │   │   ├── classes.rs  # Class management commands
│   │   │   ├── week.rs     # Weekly planner commands
│   │   │   ├── today.rs    # Today view data commands
│   │   │   ├── analytics.rs # Analytics calculations
│   │   │   ├── planning.rs # Study blocks & milestones
│   │   │   ├── planner.rs  # Task planning commands
│   │   │   ├── exams.rs    # Exam management
│   │   │   ├── recurrence.rs # Recurring template commands
│   │   │   ├── pomodoro.rs # Pomodoro settings
│   │   │   ├── todoist.rs  # Todoist sync commands
│   │   │   └── mod.rs      # Command module exports
│   │   ├── db/             # Database layer
│   │   │   ├── mod.rs      # Database connection & queries
│   │   │   └── migrations/ # SQL migration files (0001-0014)
│   │   ├── timer/          # Timer state & logic
│   │   │   └── mod.rs      # Timer implementation
│   │   ├── todoist/        # Todoist integration
│   │   │   ├── client.rs   # Todoist API client
│   │   │   ├── sync.rs     # Sync logic
│   │   │   └── mod.rs      # Module exports
│   │   ├── desktop.rs      # Desktop integration (tray, menu)
│   │   ├── tray.rs         # System tray implementation
│   │   ├── idle.rs         # Idle detection module
│   │   ├── reminders.rs    # Reminder notifications
│   │   ├── export.rs       # Data export (CSV, JSON, ICS)
│   │   ├── exam_countdown.rs # Exam countdown timer
│   │   └── weekly_review.rs  # Weekly review logic
│   ├── tauri.conf.json     # Tauri app configuration
│   ├── Cargo.toml          # Rust dependencies
│   └── icons/              # App icons (32x32, 128x128, icns, ico)
├── build/                  # Built frontend (SvelteKit static output)
│   └── _app/               # Generated SvelteKit app bundles
├── .svelte-kit/            # SvelteKit build cache & types
├── .planning/codebase/     # Documentation (this folder)
├── .github/workflows/      # GitHub Actions CI/CD
├── .vscode/                # VS Code workspace settings
├── static/                 # Static assets (favicon, logos)
├── scripts/                # Build & utility scripts
│   ├── pre-commit          # Pre-commit hook
│   └── release.mjs         # Release automation script
├── docs/                   # Project documentation
├── node_modules/           # Node.js dependencies
├── svelte.config.js        # SvelteKit configuration
├── vite.config.js          # Vite build configuration
├── tsconfig.json           # TypeScript configuration
├── package.json            # Frontend dependencies & scripts
├── package-lock.json       # Locked frontend versions
└── README.md               # Project overview
```

## Directory Purposes

**`src/lib/`:**

- Purpose: Shared frontend utilities, state management, and components
- Contains: API bindings, Svelte stores, Svelte components, formatting helpers, type definitions
- Key files: `api.ts` (IPC bridge to Tauri), `stores/` (reactive state), `components/` (reusable UI)

**`src/routes/`:**

- Purpose: SvelteKit file-based routing and page components
- Contains: Route pages, nested layouts, data loaders
- Key files: `+layout.svelte` (app shell), `+page.svelte` files for each route
- Pattern: Each directory becomes a route; `+page.svelte` is the page, `+layout.svelte` is the wrapper

**`src-tauri/src/commands/`:**

- Purpose: Tauri IPC command handlers (bridge between frontend and backend)
- Contains: One Rust file per feature domain (tasks, classes, analytics, planning, etc.)
- Pattern: Each command is an async function exported via `#[tauri::command]` macro

**`src-tauri/src/db/`:**

- Purpose: Database initialization, queries, and migrations
- Contains: SQLite setup, query functions, numbered migration SQL files
- Pattern: Migrations run sequentially (0001_init.sql through 0014_task_dependencies.sql)

**`src-tauri/src/timer/`, `todoist/`, `desktop.rs`, `tray.rs`, etc.:**

- Purpose: Feature-specific business logic modules
- Contains: State management, external API integration, desktop features
- Pattern: Each module has a `mod.rs` file (or single `.rs` file) exporting public API

**`build/`:**

- Purpose: Static frontend build output (generated by `vite build`)
- Contains: Compiled SvelteKit app bundles, JavaScript, CSS
- Generated: Yes (from `npm run build`)
- Committed: No (git-ignored)

**`.svelte-kit/`:**

- Purpose: SvelteKit build cache and TypeScript type stubs
- Generated: Yes (from `svelte-kit sync`)
- Committed: No (git-ignored)

**`static/`:**

- Purpose: Static assets served as-is
- Contains: favicon.png, Svelte/Tauri/Vite logos
- Key behavior: Files here are copied to build root

**`scripts/`:**

- Purpose: Build and release automation
- Contains: pre-commit hook, release script (Node.js)

## Key File Locations

**Entry Points:**

- `src/routes/+layout.svelte`: Root app shell (nav, timer bar, modals, keyboard shortcuts)
- `src-tauri/src/main.rs`: Tauri app entry (delegates to lib.rs)
- `src-tauri/src/lib.rs`: Tauri setup, command registration, plugin initialization

**Configuration:**

- `svelte.config.js`: SvelteKit static adapter config (fallback to index.html for SPA)
- `vite.config.js`: Vite build configuration
- `tsconfig.json`: TypeScript settings
- `package.json`: Frontend dependencies and build scripts
- `src-tauri/Cargo.toml`: Rust dependencies and build flags
- `src-tauri/tauri.conf.json`: Tauri window, security, updater settings

**Core Logic:**

- `src/lib/api.ts`: All Tauri IPC command signatures (function wrapper layer)
- `src/lib/stores/timer.svelte.ts`: Active timer state and operations
- `src/lib/stores/pomodoro.svelte.ts`: Pomodoro phase and timing
- `src-tauri/src/commands/*.rs`: Feature implementations (tasks, classes, analytics, etc.)
- `src-tauri/src/db/mod.rs`: SQLite connection and query layer
- `src-tauri/src/timer/mod.rs`: Backend timer session tracking

**Testing:**

- `.planning/codebase/TESTING.md`: Test patterns and examples
- Tests are co-located in Rust files (using `#[cfg(test)]` modules) or separate test files (not yet in place for frontend)

**Database:**

- `src-tauri/src/db/migrations/`: Numbered SQL files (0001-0014) applied sequentially at startup

## Naming Conventions

**Files:**

| Pattern | Example | Purpose |
|---------|---------|---------|
| PascalCase.svelte | `TaskPicker.svelte`, `ThemeSettings.svelte` | Svelte components |
| lowercase.svelte.ts | `timer.svelte.ts`, `pomodoro.svelte.ts` | Svelte stores |
| camelCase.ts | `api.ts`, `format.ts` | TypeScript utility modules |
| snake_case.rs | `lib.rs`, `main.rs`, `tasks.rs` | Rust files |
| +layout.svelte, +page.svelte | (SvelteKit convention) | Routes and layouts |
| NNNN_description.sql | `0001_init.sql`, `0005_recurring_tasks.sql` | Database migrations |

**Directories:**

| Pattern | Example | Purpose |
|---------|---------|---------|
| lowercase | `commands/`, `stores/`, `components/` | Feature or module groups |
| route-path | `inbox/`, `week/`, `calendar/` | SvelteKit routes (become URL paths) |

**Functions:**

| Language | Pattern | Example |
|----------|---------|---------|
| TypeScript (API bindings) | camelCase + "Command" | `listClasses()`, `createTask()`, `getAnalyticsWeek()` |
| Svelte/TypeScript (util) | camelCase | `formatHms()`, `debounce()` |
| Rust (commands) | snake_case with `#[tauri::command]` | `fn list_classes()`, `fn create_task()` |
| Rust (internal) | snake_case | `fn calculate_capacity()`, `fn sync_tasks()` |

**Types:**

| Language | Pattern | Example |
|----------|---------|---------|
| TypeScript | PascalCase | `NotificationPreferences`, `ClassRecord`, `PomodoroSettings` |
| Rust | PascalCase for structs/enums | `TaskRecord`, `ClassRecord`, `RecurrenceRule` |

**Stores:**

| Store | File | Purpose |
|-------|------|---------|
| timerStore | `timer.svelte.ts` | Active timer, session tracking, pause/resume/finish |
| pomodoroStore | `pomodoro.svelte.ts` | Pomodoro phase, remaining time, work/break state |
| themeStore | `theme.svelte.ts` | Dark/light mode, theme preference persistence |
| updaterStore | `updater.svelte.ts` | App update availability and installation status |

## Where to Add New Code

**New Feature (e.g., "Notifications"):**

1. Backend command: Create `src-tauri/src/commands/notifications.rs` with handler functions
2. Export in `src-tauri/src/lib.rs` via `.invoke_handler()` registration
3. Frontend bindings: Add exports to `src/lib/api.ts` (type-safe wrappers)
4. Frontend UI: Create component(s) in `src/lib/components/NotificationsPanel.svelte`
5. Frontend state: If needed, add store in `src/lib/stores/notifications.svelte.ts`
6. Route: If UI-heavy, add route at `src/routes/notifications/+page.svelte`
7. Database: Add migration in `src-tauri/src/db/migrations/` if schema changes needed

**New Component/UI Module:**

- Reusable: `src/lib/components/YourComponent.svelte` (PascalCase, export as default)
- Page-specific: `src/routes/[route]/YourComponent.svelte` (co-locate with route)
- Example: `src/lib/components/TaskPicker.svelte` is reused across routes; `PomodoroSettings.svelte` is shared modal

**New Utility/Helper:**

- Formatting/transforming: `src/lib/format.ts` (or create new `src/lib/helpers.ts`)
- Type definitions: `src/lib/taskMetadata.ts` or dedicated `src/lib/types/` folder
- Example: `formatHms()` in `format.ts`, `PomodoroSettings` in `api.ts`

**Backend Business Logic (no UI):**

- Core algorithm: `src-tauri/src/[feature]/mod.rs` (e.g., `timer/mod.rs` for timer logic)
- Command handler: `src-tauri/src/commands/[feature].rs` (entry point for frontend)
- Database query: `src-tauri/src/db/mod.rs` (add function, keep DB layer separated)

**Database Schema Change:**

1. Create new migration: `src-tauri/src/db/migrations/00NN_description.sql`
2. Update `src-tauri/src/db/mod.rs` to run migrations at startup (already handles sequential loading)
3. Add query functions to `src-tauri/src/db/mod.rs` for new columns/tables
4. Update Rust structs and command handlers to use new schema

**Testing:**

- Rust unit tests: Add `#[cfg(test)] mod tests {}` inside command modules (`src-tauri/src/commands/tasks.rs`, etc.)
- Frontend tests: Follow TESTING.md patterns (not yet integrated; use Vitest if adding)

## Special Directories

**`src-tauri/target/`:**

- Purpose: Cargo build output (Rust compilation artifacts)
- Generated: Yes
- Committed: No (git-ignored)

**`src-tauri/gen/`:**

- Purpose: Tauri code generation (bindings, types)
- Generated: Yes (by Tauri CLI)
- Committed: No (git-ignored)

**`src-tauri/capabilities/`:**

- Purpose: Tauri capability/permission definitions
- Contains: JSON files defining IPC security scopes
- Committed: Yes (part of app security configuration)

**`.github/workflows/`:**

- Purpose: GitHub Actions CI/CD pipeline
- Contains: Workflow YAML files for build, test, release
- Committed: Yes

**`docs/`:**

- Purpose: Project documentation (developer guides, design docs)
- Committed: Yes

## Build Output & Artifacts

**Frontend Build:**

- Command: `npm run build` → Vite builds frontend to `build/`
- Output: Static HTML, CSS, JS bundles in `build/_app/`
- Used by: Tauri's `frontendDist` setting in `tauri.conf.json`

**Rust Build:**

- Command: `npm run tauri build` (or `cargo build`)
- Output: Compiled Tauri app to `src-tauri/target/[arch]/release/`
- Used by: Tauri bundler creates `.app` (macOS), `.exe` (Windows), `.AppImage` (Linux)

**Development:**

- Frontend: `npm run dev` → Vite dev server on `http://localhost:1420`
- Backend: Tauri watches frontend and rebuilds on changes
- Command: `npm run tauri dev` runs both frontend dev server and Tauri app

---

*Structure analysis: 2026-09-20*
