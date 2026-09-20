---
last_mapped_commit: 3628ccdef63f6dfb4e6d6b20ffce30c3d515cda3
last_mapped_at: 2026-09-20
---
# Testing Patterns

**Analysis Date:** 2026-09-20

## Current State

**No testing framework is currently configured.** The project has:

- No `vitest.config.ts` or `jest.config.js`
- No testing dependencies in `package.json`
- No `*.test.ts` or `*.spec.ts` files in `src/`
- TypeScript strict mode enabled for compile-time checks

## Recommended Testing Setup

### Test Framework

**Recommended: Vitest**

- Best for Vite + SvelteKit projects
- Fast, uses Vite's transformation pipeline
- Great TypeScript support
- Easy to set up alongside existing config

**Installation:**

```bash
npm install -D vitest @testing-library/svelte @testing-library/user-event happy-dom
npm install -D @vitest/ui  # optional: helpful for debugging
```

**Configuration File: `vitest.config.ts`**

```typescript
import { defineConfig } from "vitest/config";
import { sveltekit } from "@sveltejs/kit/vite";

export default defineConfig({
  plugins: [sveltekit()],
  test: {
    globals: true,
    environment: "happy-dom",
    setupFiles: [],
  },
});
```

**Package Scripts to Add:**

```json
{
  "scripts": {
    "test": "vitest",
    "test:watch": "vitest --watch",
    "test:ui": "vitest --ui",
    "test:coverage": "vitest --coverage"
  }
}
```

## Test File Organization

**Location:**

- Co-located with source files
- Parallel structure: `src/lib/format.ts` → `src/lib/format.test.ts`
- Component tests: `src/lib/components/Modal.svelte` → `src/lib/components/Modal.test.ts`
- Store tests: `src/lib/stores/timer.svelte.ts` → `src/lib/stores/timer.test.ts`

**Naming:**

- Utility: `format.test.ts`
- Component: `Modal.test.ts`
- Store: `timer.test.ts`
- API: `api.test.ts`

**Directory Structure:**

```
src/
├── lib/
│   ├── api.ts
│   ├── api.test.ts
│   ├── format.ts
│   ├── format.test.ts
│   ├── stores/
│   │   ├── timer.svelte.ts
│   │   └── timer.test.ts
│   └── components/
│       ├── Modal.svelte
│       └── Modal.test.ts
└── routes/
    ├── +page.svelte
    └── +page.test.ts
```

## Test Structure

**Suite Organization:**

```typescript
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { formatHms, formatMinutesShort } from "./format";

describe("format.ts", () => {
  describe("formatHms", () => {
    it("formats seconds as MM:SS", () => {
      expect(formatHms(125)).toBe("02:05");
    });

    it("formats hours as HH:MM:SS", () => {
      expect(formatHms(3725)).toBe("01:02:05");
    });

    it("handles zero seconds", () => {
      expect(formatHms(0)).toBe("00:00");
    });

    it("handles negative seconds by clamping to zero", () => {
      expect(formatHms(-10)).toBe("00:00");
    });
  });

  describe("formatMinutesShort", () => {
    it("formats minutes under 60", () => {
      expect(formatMinutesShort(45)).toBe("45m");
    });

    it("formats hours and minutes", () => {
      expect(formatMinutesShort(125)).toBe("2h 5m");
    });
  });
});
```

**Patterns:**

- `describe()` blocks group related tests (one per exported function/component)
- `it()` blocks test specific behavior
- Meaningful test descriptions starting with verb: "should format", "throws on", "renders when"
- One assertion per test (or tightly related assertions)
- Setup with `beforeEach()`, cleanup with `afterEach()`

## Mocking

### Framework: Vitest Native

**Mock Tauri API Calls:**

```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { exportData } from "./api";

vi.mock("@tauri-apps/api/core");

describe("api.ts", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("exportData", () => {
    it("invokes export_data with format param", async () => {
      const mockInvoke = vi.mocked(invoke);
      mockInvoke.mockResolvedValueOnce(true);

      const result = await exportData("csv");

      expect(mockInvoke).toHaveBeenCalledWith("export_data", { format: "csv" });
      expect(result).toBe(true);
    });

    it("handles errors from invoke", async () => {
      const mockInvoke = vi.mocked(invoke);
      mockInvoke.mockRejectedValueOnce(new Error("Backend error"));

      await expect(exportData("json")).rejects.toThrow("Backend error");
    });
  });
});
```

**Mock Store Methods:**

```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { timerStore } from "./timer.svelte";

describe("timerStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("sets error state when refresh fails", async () => {
    vi.spyOn(timerStore, "refresh").mockRejectedValueOnce(new Error("Network"));
    
    await timerStore.refresh();

    expect(timerStore.error).toBe("Couldn't refresh timer. Try again.");
  });
});
```

**What to Mock:**

- Tauri API calls (`invoke()`, `listen()`, `emit()`)
- External service calls (Todoist API)
- Timer/Date functions when testing time-dependent logic
- File system operations (if any)

**What NOT to Mock:**

- Utility functions (`format.ts`, `priority.ts`)
- Component rendering (use @testing-library/svelte)
- Store state logic (test the real store behavior)
- Pure functions (mathematical operations, string manipulation)

## Fixtures and Factories

### Test Data

**Location:** `src/lib/testUtils.ts` or co-located factories

**Example Factory for TaskRecord:**

```typescript
import type { TaskRecord } from "$lib/api";

export function createMockTaskRecord(overrides?: Partial<TaskRecord>): TaskRecord {
  return {
    id: 1,
    class_id: 1,
    parent_task_id: null,
    title: "Sample Task",
    description: null,
    notes: null,
    status: "not_started",
    priority: 2,
    due_at: "2026-09-25",
    scheduled_date: "2026-09-20",
    estimated_minutes: 60,
    source: "local",
    external_id: null,
    completed_at: null,
    tracked_seconds: 0,
    tracked_seconds_direct: 0,
    remaining_minutes: 60,
    external_state: "active",
    task_type: "assignment",
    tags: [],
    time_budget_minutes: null,
    scheduled_minutes_before_due: 300,
    unplanned_minutes: 60,
    schedule_coverage_percent: 0,
    blocked_by_open_count: 0,
    ...overrides,
  };
}
```

**Usage in Tests:**

```typescript
const task = createMockTaskRecord({ title: "LGST Assignment", priority: 4 });
```

**Example Fixture for Classes:**

```typescript
export const MOCK_CLASSES = [
  { id: 1, course_code: "LGST1000", ... },
  { id: 2, course_code: "PHIL1439", ... },
];
```

## Coverage

**Requirements:** Not enforced (no coverage config in codebase)

**Recommendation:** Target 80% for critical paths:

- All API wrapper functions in `src/lib/api.ts`
- All utility functions in `src/lib/format.ts`, `src/lib/priority.ts`
- All store methods with async operations
- Critical component interactions (Modal open/close, form submit)

**View Coverage:**

```bash
npm run test:coverage
```

**Coverage Config (if added to `vitest.config.ts`):**

```typescript
export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "html"],
      exclude: [
        "node_modules/",
        "src-tauri/",
        "**/*.test.ts",
      ],
    },
  },
});
```

## Test Types

### Unit Tests

**Scope:** Individual functions and methods
**Approach:** Test a single behavior per test

**Example: `format.test.ts`**

```typescript
import { describe, it, expect } from "vitest";
import { formatHms, formatDurationShort } from "./format";

describe("format utilities", () => {
  it("formatHms pads single digits", () => {
    expect(formatHms(65)).toBe("01:05");
  });

  it("formatDurationShort uses human-readable units", () => {
    expect(formatDurationShort(3725)).toBe("1h 2m");
  });
});
```

### Component Tests

**Scope:** Svelte component rendering and interaction
**Tools:** @testing-library/svelte + user-event

**Example: `Modal.test.ts`**

```typescript
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Modal from "./Modal.svelte";

describe("Modal", () => {
  it("renders with title and children", () => {
    render(Modal, {
      props: {
        title: "Test Modal",
        onclose: vi.fn(),
        children: "Modal content",
      },
    });

    expect(screen.getByText("Test Modal")).toBeInTheDocument();
    expect(screen.getByText("Modal content")).toBeInTheDocument();
  });

  it("calls onclose when close button clicked", async () => {
    const user = userEvent.setup();
    const onclose = vi.fn();

    render(Modal, {
      props: { title: "Test", onclose, children: "" },
    });

    const closeButton = screen.getByRole("button", { name: /close/i });
    await user.click(closeButton);

    expect(onclose).toHaveBeenCalled();
  });

  it("prevents escape key from closing", async () => {
    const user = userEvent.setup();
    const onclose = vi.fn();

    render(Modal, {
      props: { title: "Test", onclose, children: "" },
    });

    await user.keyboard("{Escape}");

    // Modal should still be open (onclose called once by our click, not by escape)
    expect(onclose).not.toHaveBeenCalled();
  });
});
```

### Integration Tests

**Scope:** Multiple components/systems working together
**Approach:** Test user workflows, not implementation details

**Example: Quick Add Task Flow**

```typescript
describe("QuickAdd component flow", () => {
  it("parses shorthand and creates task", async () => {
    const user = userEvent.setup();
    const mockCreate = vi.mocked(createTask);
    mockCreate.mockResolvedValueOnce(createMockTaskRecord());

    render(QuickAdd, { props: { onclose: vi.fn() } });

    const shorthandInput = screen.getByPlaceholderText(/shorthand/i);
    await user.type(shorthandInput, "LGST1000 assignment due Fri 60m");
    await user.click(screen.getByText("Parse"));

    expect(screen.getByDisplayValue("LGST1000")).toBeInTheDocument();
    expect(screen.getByDisplayValue("assignment due Fri 60m")).toBeInTheDocument();
  });
});
```

### E2E Tests

**Status:** Not implemented
**Recommendation:** Consider Playwright or Cypress for full desktop app testing (Tauri-specific e2e setup exists)
**Focus:** Critical user paths (create task, start timer, complete task)

## Common Patterns

### Async Testing

**Promise-based:**

```typescript
it("loads and displays task data", async () => {
  const mockFetch = vi.mocked(listTasksForClass);
  mockFetch.mockResolvedValueOnce([createMockTaskRecord()]);

  render(TaskList, { props: { classId: 1 } });

  const title = await screen.findByText("Sample Task");
  expect(title).toBeInTheDocument();
});
```

**With Timeout (if needed):**

```typescript
it("displays error after timeout", async () => {
  const mockFetch = vi.mocked(getPomodoroSettings);
  mockFetch.mockImplementation(
    () => new Promise((_, reject) => 
      setTimeout(() => reject(new Error("Timeout")), 100)
    )
  );

  const { rerender } = render(PomodoroSettings);

  await new Promise(resolve => setTimeout(resolve, 150));
  await rerender();

  expect(screen.getByText(/couldn't load/i)).toBeInTheDocument();
}, { timeout: 5000 });
```

### Error Testing

**Exception Handling:**

```typescript
it("displays error message when save fails", async () => {
  const user = userEvent.setup();
  const mockSave = vi.mocked(savePomodoroSettings);
  mockSave.mockRejectedValueOnce(new Error("Network error"));

  render(PomodoroSettings);
  await user.click(screen.getByText("Save"));

  expect(screen.getByText(/couldn't save/i)).toBeInTheDocument();
});
```

**Discriminated Union Errors:**

```typescript
it("handles ActiveSessionConflict error", async () => {
  const mockStart = vi.mocked(startTimer);
  mockStart.mockRejectedValueOnce({
    kind: "ActiveSessionConflict",
    task_id: 2,
    task_title: "Other Task",
    class_course_code: "PHIL1439",
  } as TimerError);

  await timerStore.start(1, "LGST1000");

  expect(timerStore.conflict?.taskId).toBe(2);
  expect(timerStore.conflict?.taskTitle).toBe("Other Task");
});
```

### Store Testing

**Mutable State:**

```typescript
describe("timerStore", () => {
  it("increments revision on timer change", async () => {
    const initialRevision = timerStore.revision;
    
    vi.mocked(startTimer).mockResolvedValueOnce(createMockActiveSessionInfo());
    await timerStore.start(1, "Task");

    expect(timerStore.revision).toBe(initialRevision + 1);
  });

  it("clears error state on successful action", async () => {
    timerStore.error = "Previous error";
    
    vi.mocked(startTimer).mockResolvedValueOnce(createMockActiveSessionInfo());
    await timerStore.start(1, "Task");

    expect(timerStore.error).toBe("");
  });
});
```

## Snapshot Testing

**Not Recommended** for this codebase because:

- Component snapshots are brittle with frequent UI changes
- Hard to review diffs in snapshots
- Encourages approving changes without understanding them

**Exception:** Might use snapshots for stable data structures (e.g., `PRIORITY_LABELS` constant output)

---

*Testing analysis: 2026-09-20*
