# CODEBUDDY.md This file provides guidance to CodeBuddy when working with code in this repository.

## Common Commands

- **Install dependencies**: `pnpm install` (requires Node >= 24.16.0; enable pnpm via `corepack enable pnpm`)
- **Dev server**: `pnpm dev -- --open` — starts Vite dev server on port 3000. Use `pnpm dev:force` to force optimization and use local mermaid package
- **Build**: `pnpm build` — produces static output to `docs/`
- **Preview build**: `pnpm preview` — serves the built `docs/` directory on port 3000
- **Lint**: `pnpm lint` (check only) or `pnpm lint:fix` (auto-fix). Runs Prettier + ESLint
- **Format**: `pnpm format` — runs Prettier write on all files
- **Type check**: `pnpm check` — runs `svelte-kit sync` then `svelte-check`
- **Unit tests**: `pnpm test:unit` — runs Vitest in jsdom. Use `pnpm test:unit:ui` for UI mode, `pnpm test:unit:coverage` for coverage
- **E2E tests**: `pnpm test:e2e` — Playwright tests against `http://localhost:3000`. Use `pnpm test:e2e:ui` for interactive mode, `pnpm test:e2e:debug` for debugging
- **Run all tests**: `pnpm test` (unit + e2e). E2E tests require the dev server to be running; Playwright auto-starts it via `webServer` config
- **Docker dev**: `docker compose up --build` — builds and serves on port 3000 with hot-reload via volume mount
- **Run a single unit test file**: `pnpm vitest run src/lib/util/serde.test.ts` (or any test file path)
- **Run a single E2E test**: `pnpm test:e2e -- tests/loadSite.spec.ts` or filter by test name with `--grep "test name"`

## Architecture Overview

### Tech Stack
- **Framework**: Svelte 5 + SvelteKit 2 (adapter-static for fully static SPA output)
- **Language**: TypeScript 6 with strict null checks
- **Build**: Vite 8 with env prefix `MERMAID_`
- **Styling**: Tailwind CSS 4 + shadcn-svelte (bits-ui components)
- **Editor**: Monaco Editor with custom mermaid language grammar (syntax highlighting, tokenizer, themes)
- **Rendering**: mermaid ^11.15.0, svg-pan-zoom, svg2roughjs (hand-drawn mode), Hammer.js for touch gestures
- **Serialization**: State stored as JSON → Pako (deflate) → Base64 in URL hash. Plain Base64 and legacy formats also supported
- **Testing**: Vitest (jsdom, in-source testing) + Playwright (E2E on Chromium)
- **Package manager**: pnpm (with `packageManager` field pinned in package.json)
- **CI/CD**: Netlify auto-deploys on PRs targeting master; merges auto-release

### Route Structure (SPA, SSG with `ssr = false`)
| Route | Purpose |
|-------|---------|
| `/` | Redirects to `/edit` (handles legacy hash-based URL format) |
| `/edit` | **Main editor**: Monaco code/config editor (resizable panes), diagram preview, history sidebar, presets, toolbar actions |
| `/view` | Read-only diagram view (noindex meta, no editor/controls, just the rendered SVG) |

All three routes share the same `+layout.svelte` which provides dark/light mode via `mode-watcher`, toast notifications (`toaster`), service worker registration, and a full-screen loading overlay.

### State Management (Svelte 5 Runes)

The entire app uses Svelte 5 runes (`$state`, `$derived`, `$derived.by`, `$state.raw`, `$effect`, `untrack`). There is no external store library.

#### Core State (`src/lib/util/state.svelte.ts`)
The single source of truth is the `input` variable, a `$state<State>`. All mutations must go through the `update()` function, which:
1. Runs the mutator inside `untrack()` so `$effect` callers don't subscribe to reads
2. Writes to localStorage (`codeStore` key) via `writeJSON`
3. Asynchronously re-parses/validates mermaid code, producing `validatedState`

Key exports:
- **`inputState`**: Readonly reactive `State` — use for exporting to URL, History, etc.
- **`validatedState.current`**: A `$state.raw<ValidatedState>` that includes `error`, `errorMarkers`, `serialized` (the compressed URL-ready string), and `diagramType`
- **`urls.current`**: A `$derived.by` object providing all external URLs (PNG, SVG, view link, new diagram, Mermaid Chart links, Kroki, markdown embed)
- **`updateCode(newCode)`, `updateConfig(config)`**: Convenience mutation functions
- **`replaceInputState(next)`**: Replaces entire state (history restore)
- **`loadState(data)`**: Deserializes state from URL hash, sanitizes config, applies it
- **`sanitizeConfig(config)`**: Removes unsafe keys (XSS, prototype pollution) from mermaid config; prompts user via `confirm()` if unsafe values differ from defaults
- **`initURLSubscription()`**: Sets up debounced (250ms) `history.replaceState` for URL hash sync

#### Data Flow
```
URL hash change → loadState(data) → deserializeState → update() → persistAndProcess()
                                                    ↓
                                              localStorage save
                                              parse mermaid code
                                              produce validatedState
                                                    ↓
                                              View.svelte ($effect) → mermaid.render() → innerHTML SVG
```

User edits in Monaco trigger `onUpdate` → `updateCode(text)` → `update()` → persist + validate → View reactive update.

#### Other State Modules
- **`persist.svelte.ts`**: `readJSON(key, fallback)`, `writeJSON(key, value)` for raw localStorage I/O. `persisted(key, initial)` creates a `$state.raw` backed by localStorage with automatic write-on-set
- **`loading.svelte.ts`**: Simple `$state<LoadingState>` for global loading overlay
- **`historyState.svelte.ts`**: Two persisted stores (auto-save every 60s, max 30 entries; manual user saves). Dedup by `stateKey`, prepend new entries
- **`migrations.svelte.ts`**: Version-based migration runner applied once on init
- **`autoSync.ts`**: Throttles View re-renders — if a render takes > 150ms, debounces subsequent updates to 1-second intervals
- **`panZoom.ts`**: `PanZoomState` class manages svg-pan-zoom lifecycle, Hammer.js gestures, resize observation, and syncs pan/zoom back to the global state

### Component Architecture

#### Editor (`src/lib/components/Editor.svelte`)
Wraps either `DesktopEditor.svelte` or `MobileEditor.svelte` depending on viewport width. Both use Monaco Editor. The `onUpdate` callback dispatches based on `editorMode` (`'code'` → `updateCode`, `'config'` → `updateConfig`). Shows syntax error panel (after 3-second debounce) with a link to AI Repair on Mermaid Chart.

#### View (`src/lib/components/View.svelte`)
Watches `validatedState.current` via `$effect`. On change:
1. Skips if code/config/rough/panZoom unchanged
2. Optional render throttling via `autoSync.shouldRefreshView()`
3. Renders mermaid to SVG via `mermaid.render(id, code)`
4. If `rough` mode: converts SVG via `svg2roughjs.sketch()`
5. Sets up pan/zoom via `panZoomState.updateElement(graphDiv, state)`
6. Records render time for statistics

Uses a promise chain (`pendingStateChange`) to serialize state changes and avoid race conditions.

#### Editor Chooser Modal (`src/lib/components/migration/EditorChooserModal.svelte`)
Shown once per session on `/edit` to nudge experienced users toward Mermaid Chart (mermaid.ai). Decision persisted in localStorage.

#### Preset (`src/lib/components/Preset.svelte`)
Loads sample diagrams from `@mermaid-js/examples`, grouped by diagram type with default examples first.

#### History (`src/lib/components/History/History.svelte`)
Three tabs: Timeline (auto-saved), History (manual saves), Revisions (gist-loaded). Supports restore, delete, and download/upload.

#### Toolbar Components
- **`PanZoomToolbar.svelte`**: Zoom in/out/reset, pan controls
- **`SyncRoughToolbar.svelte`**: Toggle rough (hand-drawn) mode, toggle auto-sync
- **`VersionSecurityToolbar.svelte`**: Mermaid version display, security info modal
- **`FloatingToolbar.svelte`**: Quick actions floating next to diagram

#### UI Components (`src/lib/components/ui/`)
shadcn-svelte components (button, dialog, input, popover, resizable, separator, sonner, switch, toggle, toggle-group, tooltip). Generated from the shadcn-svelte new-york registry with Tailwind v3-compatible styles.

### Serialization (`src/lib/util/serde.ts`)
Two serde formats:
- **`pako`** (default): `JSON.stringify(state)` → `deflate` (level 9) → Base64
- **`base64`**: Plain Base64 of JSON

URL hash format: `{type}:{encodedState}` (e.g., `pako:eNpVkM2K...`). Legacy format without prefix defaults to base64. `serializeState()` and `deserializeState()` handle both.

### External Integrations
- **Mermaid Renderer** (`MERMAID_RENDERER_URL`): Separate service (default `mermaid.ink`) for PNG/SVG export. Set to empty to disable
- **Kroki** (`MERMAID_KROKI_RENDERER_URL`): Alternative diagram renderer URL
- **Plausible Analytics** (`MERMAID_ANALYTICS_URL`): Self-hosted analytics, debounced
- **Mermaid Chart** (`MERMAID_IS_ENABLED_MERMAID_CHART_LINKS`): Promotional integration — enables "Save diagram" button, AI Repair, editor chooser modal, UTM-tagged links
- **Gist/File Loader**: `/edit?gist={url}` or `/edit?code={url}&config={url}` loads diagrams from GitHub Gists or raw files

### Key Architectural Decisions
1. **Fully client-side**: `ssr = false`, `adapter-static` — no server rendering. The app is deployed as static files served by nginx
2. **Always full-page reload on HMR**: A custom Vite plugin forces full reload on any change because HMR creates state inconsistencies with Svelte 5 runes
3. **Single mutation gateway**: All state writes go through `update()` in `state.svelte.ts` to ensure localStorage persistence and mermaid re-validation are never skipped
4. **Monaco with custom mermaid language**: `monacoExtra.ts` defines a comprehensive Monarch tokenizer with per-diagram-type token rules, completion provider, and light/dark themes. The config tab provides JSON editing via `@codemirror/lang-json`
5. **Security-first config sanitization**: On loading any external state (URL hash), mermaid config is checked against unsafe keys (`secure` array from mermaid defaults) and XSS vectors (`<`, `>`, `url(data:`), with user confirmation for removal
6. **In-source testing**: Vitest is configured with `includeSource` for co-locating tests with their source modules (e.g., `persist.svelte.test.ts`, `serde.test.ts`)
7. **ZenUML workaround**: After rendering ZenUML diagrams, the page forces a full reload because the external diagram plugin causes state corruption

### Testing Patterns
- **Unit tests**: Use Vitest with jsdom. Tests are co-located with source (e.g., `src/lib/util/serde.test.ts`). Import from `vitest`
- **E2E tests**: Playwright with Chromium. Custom fixtures in `tests/test.ts` provide `editPage` with helper methods (`start()`, `checkInEditor()`, `checkTextInView()`, `setEditorMode()`, `loadSampleDiagram()`, etc.). Tests run against `http://localhost:3000`. Clipboard permissions are granted by default
- **Test setup**: `src/tests/setup.ts` provides setup for jsdom environment in Vitest

### Docker
Multi-stage build: dependencies → builder (vite build) → nginx serving `docs/` on port 8080. Dev target (`mermaid-dev`) uses `pnpm dev`. Docker Compose mounts `./src` for hot-reload.
