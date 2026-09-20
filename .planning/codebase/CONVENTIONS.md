---
last_mapped_commit: 3628ccdef63f6dfb4e6d6b20ffce30c3d515cda3
last_mapped_at: 2026-09-20
---
# Coding Conventions

**Analysis Date:** 2026-09-20

## Naming Patterns

**Files:**

- Utility files: `camelCase.ts` (`api.ts`, `format.ts`, `priority.ts`, `taskMetadata.ts`)
- Component files: `PascalCase.svelte` (`Modal.svelte`, `Icon.svelte`, `QuickAdd.svelte`)
- Store files: `camelCase.svelte.ts` (`timer.svelte.ts`, `pomodoro.svelte.ts`, `theme.svelte.ts`)
- Route files: `+page.svelte`, `+layout.svelte`, `+layout.ts` (SvelteKit convention)

**Functions:**

- All functions use `camelCase`: `exportData()`, `createTask()`, `formatHms()`, `loadParents()`
- Async functions use same pattern: `async function refresh()`
- Constructor-like functions (store exports) are PascalCase classes: `class TimerStore`

**Variables:**

- Local variables: `camelCase` (`workMinutes`, `taskType`, `error`, `busy`)
- Private fields in classes: `private fieldName` or `private _fieldName` (`private fetchedAtMs`, `private tickHandle`, `private request`)
- Component state: `let stateVar = $state(value)` in Svelte 5
- Loop variables: `for (let item of items)` or destructuring in `.find()`, `.map()` chains

**Types:**

- Interface names: `PascalCase` (`TaskRecord`, `NotificationPreferences`, `ActiveSessionInfo`, `TimerError`)
- Type aliases: `PascalCase` (`TaskType` = union of string literals)
- Union types: `"not_started" | "in_progress" | "completed"`
- Generic type parameters: `<T>` (standard convention)
- Database fields: `snake_case` when mirroring backend schema (`task_id`, `class_id`, `course_code`, `is_paused`)

**Constants:**

- UPPERCASE_SNAKE_CASE: `DEFAULT_PRIORITY`, `TASK_TYPES`, `PRIORITY_LABELS`
- Location: Exported from utility files or defined at module level

## Code Style

**Formatting:**

- No external formatter configured (eslint, prettier not in dependencies)
- Use spaces for indentation (visible in all source files)
- Single-line function exports when concise:
  ```typescript
  export function exportData(format: "csv" | "tasks" | "json" | "ics"): Promise<boolean> { return invoke("export_data", { format }); }
  ```
- Multi-line for complex logic:
  ```typescript
  async function add() {
    if (busy || classId === null) return;
    busy = true;
    error = "";
    try {
      // ... logic
    } catch {
      error = "Couldn't create the task.";
    } finally {
      busy = false;
    }
  }
  ```

**Linting:**

- TypeScript strict mode enabled (`strict: true` in `tsconfig.json`)
- Check JS enabled (`checkJs: true`)
- No eslint or prettier config file present — rely on IDE defaults and TypeScript compiler

## Import Organization

**Order:**

1. Framework/library imports (`svelte`, `@tauri-apps/api`, `@sveltejs/kit`)
2. Relative imports (`$lib/*`, `$app/*`)
3. Type-only imports use `type` keyword: `import type { Snippet } from "svelte"`
4. Mixed imports separate types: `import { onMount, type Snippet } from "svelte"`

**Examples:**

```typescript
// src/lib/components/QuickAdd.svelte
import { onMount } from "svelte";
import Modal from "./Modal.svelte";
import { createTask, getEstimateSuggestion, listClasses, type ClassRecord, type TaskType } from "$lib/api";
import { TASK_TYPES, parseTags } from "$lib/taskMetadata";
import { timerStore } from "$lib/stores/timer.svelte";
```

**Path Aliases:**

- `$lib/*` → `src/lib/` (SvelteKit standard)
- `$app/*` → SvelteKit internal APIs

## Error Handling

**Patterns:**

- Try/catch wrapping async operations:
  ```typescript
  try {
    const settings = await getPomodoroSettings();
    workMinutes = settings.work_minutes;
  } catch (e) {
    error = String(e);
  }
  ```
- Error stored in reactive state (`error = $state("")`)
- Error message displayed to user: `{#if error}<p class="error">{error}</p>{/if}`
- Generic error messages (don't expose backend details)
- Finally blocks for cleanup:
  ```typescript
  try {
    await action();
  } catch {
    error = "Couldn't complete action.";
  } finally {
    busy = false;
  }
  ```
- Type-discriminated errors for complex cases:
  ```typescript
  export type TimerError = 
    | { kind: "ActiveSessionConflict"; task_id: number; task_title: string; }
    | { kind: "NotFound" }
    | { kind: "Other"; message: string };
  
  if (err?.kind !== "ActiveSessionConflict") throw e;
  ```

## Logging

**Framework:** No logger library configured; uses:

- `console.log` (not observed in codebase, avoid)
- Reactive error state: `error = $state("")`
- User-facing error messages stored in component state

**Patterns:**

- Errors in state, not console: `error = String(e)`
- Success indicators: `saved = $state(false)` with auto-timeout `setTimeout(() => saved = false, 1500)`
- No debug logging in production code

## Comments

**When to Comment:**

- Non-obvious algorithmic logic (e.g., date parsing, regex, clamping):
  ```typescript
  // Todoist's priority field already runs 1 (lowest) to 4 (urgent), matching
  // our scale numerically — but Todoist tasks with no priority set arrive as
  // null, not 1, so that case must fall through to our own default rather
  // than being assumed equal to Todoist's.
  export function fromTodoistPriority(todoistPriority: number | null): number {
  ```
- Section headers in large files:
  ```typescript
  // ---------- Classes ----------
  export interface ClassRecord { ... }
  ```

**JSDoc/TSDoc:**

- Not observed in codebase
- TypeScript types provide documentation via IDE hover

**What NOT to comment:**

- Variable names that are self-documenting
- Type annotations (the type itself documents the intent)
- Obvious control flow

## Function Design

**Size:**

- Short utility functions (1-3 lines): Single expression when possible
  ```typescript
  export function priorityLabel(priority: number | null): string {
    return PRIORITY_LABELS[priority ?? DEFAULT_PRIORITY] ?? "Normal";
  }
  ```
- Event handlers: Keep under 20 lines in component script
- Store methods: Longer OK for complex state transitions (up to 30 lines)

**Parameters:**

- Use destructuring for multiple params:
  ```typescript
  let { title, children, onclose }: { title: string; children: Snippet; onclose: () => void } = $props();
  ```
- Object params for many args (prefer over positional):
  ```typescript
  export function createTask(input: NewTask): Promise<TaskRecord>
  ```
- Null/undefined defaults using nullish coalescing `??`:
  ```typescript
  const paths: Record<string, string> = { ... };
  <svg><path d={paths[name] ?? paths.clock} /></svg>
  ```

**Return Values:**

- Explicit Promise types: `Promise<TaskRecord>`, `Promise<void>`, `Promise<boolean>`
- Union returns for errors (discriminated unions preferred):
  ```typescript
  export type TimerError = 
    | { kind: "ActiveSessionConflict"; ... }
    | { kind: "NotFound" }
    | { kind: "Other"; message: string };
  ```
- Void for side-effect functions: `export function mount(): void`

## Module Design

**Exports:**

- Explicit exports only (no `export *`):
  ```typescript
  export interface TaskRecord { ... }
  export function createTask(input: NewTask): Promise<TaskRecord> { ... }
  ```
- Type exports use `type` keyword:
  ```typescript
  export type TaskType = "assignment" | "reading" | "problem_set" | "exam" | "project" | "other";
  ```

**Barrel Files:**

- Not used; each module imports what it needs directly
- Example: `import { createTask, type TaskRecord } from "$lib/api"`

**File Purpose:**

- `src/lib/api.ts`: All Tauri invoke wrappers and type definitions
- `src/lib/format.ts`: Pure formatting utilities (no side effects)
- `src/lib/priority.ts`: Priority constants and conversion functions
- `src/lib/taskMetadata.ts`: Task metadata arrays and parsers
- `src/lib/stores/*.svelte.ts`: Reactive stores (one store per file)
- `src/lib/components/*.svelte`: Reusable UI components
- `src/routes/*/+page.svelte`: Route pages
- `src/routes/+layout.svelte`: Root layout with global listeners
- `src/routes/+layout.ts`: SSR config (disables for Tauri)

## Svelte 5 Patterns

**Reactive State:**

```typescript
let count = $state(0);
let obj = $state({ name: "Task", priority: 2 });
let array = $state<TaskRecord[]>([]);
```

**Props:**

```typescript
let { title, onclose } = $props();
// With types:
let { onclose }: { onclose: () => void } = $props();
```

**Effects:**

```typescript
$effect(() => {
  // Re-run when classId or taskType change
  void classId;
  void taskType;
  void loadSuggestion();
});
```

**Binding:**

- Input binding: `bind:value={taskName}`
- Element reference: `bind:this={dialog}`

**Template:**

- Conditionals: `{#if error}<p>{error}</p>{/if}`
- Loops: `{#each items as item}<div>{item.name}</div>{/each}`
- Component slots: `{@render children()}`

## API Layer

**Tauri Invocation:**
All backend calls go through `$lib/api.ts`:

```typescript
export function exportData(format: "csv" | "tasks" | "json" | "ics"): Promise<boolean> {
  return invoke("export_data", { format });
}
```

**Type Definitions:**

- Interfaces for all request/response shapes
- Separate `New*` types for create operations
- Separate `Update*` types for mutations
- Discriminated unions for error cases

**Request Cancellation:**
Used when response order is uncertain:

```typescript
let suggestionRequest = 0;

async function loadSuggestion() {
  const request = ++suggestionRequest;
  try {
    const value = await getEstimateSuggestion(classId, taskType);
    if (request === suggestionRequest) suggestion = value;
  } catch {
    if (request === suggestionRequest) suggestion = null;
  }
}
```

---

*Convention analysis: 2026-09-20*
