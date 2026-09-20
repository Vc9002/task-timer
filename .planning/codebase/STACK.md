---
last_mapped_commit: 3628ccdef63f6dfb4e6d6b20ffce30c3d515cda3
last_mapped_at: 2026-09-20
---
# Technology Stack

**Analysis Date:** 2026-09-20

## Languages

**Primary:**

- TypeScript ~6.0.3 - Frontend code (Svelte components, routes, utilities)
- Rust (edition 2021) - Backend code, Tauri app, database operations, timer logic, Todoist sync

**Secondary:**

- JavaScript - Build configuration, development scripts, Node.js tooling

## Runtime

**Environment:**

- Node.js 20+ (development and build-time only)
- Rust stable toolchain (backend compilation)

**Package Manager:**

- npm (Node.js packages)
- Cargo (Rust packages)
- Lockfile: `package-lock.json` and `Cargo.lock` present

## Frameworks

**Core:**

- Tauri 2 - Desktop application shell (native OS webview, no bundled browser engine)
- Svelte 5.56.3 - Frontend UI framework and component system
- SvelteKit 2.65.1 - Frontend routing, build orchestration, type safety

**Build/Dev:**

- Vite 8.0.16 - JavaScript bundler and dev server
- @sveltejs/vite-plugin-svelte 7.1.2 - Svelte integration with Vite
- @sveltejs/adapter-static 3.0.10 - Static site adapter (SPA mode, no server-side rendering)

**Testing:**

- Cargo built-in test framework (Rust library tests via `cargo test --lib`)
- No JavaScript test framework configured

**Code Quality:**

- svelte-check 4.6.0 - Svelte type checking
- cargo fmt - Rust code formatting
- cargo clippy - Rust linting

## Key Dependencies

**Critical:**

- @tauri-apps/api ^2 - TypeScript bindings to Tauri runtime
- @tauri-apps/plugin-updater ^2.12.0 - Application auto-update via GitHub releases
- @tauri-apps/cli ^2 - Build and development CLI for Tauri

**Desktop & System Integration:**

- tauri-plugin-global-shortcut 2 - Global keyboard shortcuts (hotkey support)
- tauri-plugin-autostart 2 - App autostart on system boot
- tauri-plugin-single-instance 2 - Prevent multiple app instances
- tauri-plugin-notification 2 - Native OS notifications
- tauri-plugin-dialog 2 - Native file/folder dialogs
- tauri-plugin-process 2 - Process management
- window-vibrancy 0.6 - macOS window vibrancy effects

**Backend Data & Logic:**

- rusqlite 0.40.2 - SQLite driver (with bundled SQLite via feature flag)
- serde 1 - Serialization framework (JSON support)
- serde_json 1 - JSON encoding/decoding
- chrono 0.4.45 - Date and time utilities (with serde support)
- uuid 1 - UUID v4 generation

**External Integration:**

- reqwest 0.13.5 - HTTP client with blocking mode, rustls, and JSON support (for Todoist API)
- keyring 4.2.0 - OS keychain access (credentials storage)

**Build Dependencies:**

- tauri-build 2 - Tauri build script support

## Configuration

**Environment:**

- No runtime `.env` file required
- Configuration stored locally in SQLite database
- Secrets (Todoist API token) stored in OS keychain
- App data directory: OS-specific app data folder for `com.vincentc9002.tasktimer`

**Build:**

- `vite.config.js` - Vite bundler configuration (port 1420, HMR setup, Svelte plugin)
- `svelte.config.js` - Svelte preprocessor and adapter configuration (static adapter with SPA fallback)
- `tsconfig.json` - TypeScript compiler options (strict mode, moduleResolution: bundler)
- `src-tauri/tauri.conf.json` - Tauri framework configuration (window size, security CSP, updater endpoints)
- `src-tauri/Cargo.toml` - Rust dependencies and release profile optimization

## Platform Requirements

**Development:**

- Rust stable toolchain
- Node.js 20+
- For macOS development: Xcode command-line tools
- For Linux development: webkit2gtk, libxdo, openssl development headers, librsvg2
- For Windows development: Visual Studio toolchain

**Production:**

- **Target platforms:** macOS (primary, with private API enabled), Linux, Windows
- **Deployment:** Native executable binaries for each platform, built via GitHub Actions
- **Distribution:** GitHub Releases with automatic updates via tauri-updater plugin

## Release Configuration

**Optimization (Release Profile):**

```toml
codegen-units = 1      # Single codegen unit for maximum optimization
lto = true             # Link-time optimization enabled
opt-level = 3          # Aggressive optimization
panic = "abort"        # Abort on panic (smaller binary)
strip = true           # Strip symbols for smaller size
```

---

*Stack analysis: 2026-09-20*
