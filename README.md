# Synapse

A local-first, offline-capable desktop app for spaced-repetition training — built with a Rust core, a Tauri shell, and a Svelte 5 frontend. Track hard skills (Rust, algorithms, biology, whatever you're drilling), review on a schedule that adapts to how well you actually remember things, and see the data behind your own learning.

Everything lives on disk as JSON in your OS's app-data directory. Nothing leaves your machine.

## Features

- **Two schedulers, your choice** — classic SM-2, or [FSRS-6](https://github.com/open-spaced-repetition), a modern difficulty/stability model that generally schedules more accurately. Switchable per-vault in Settings; existing cards keep whatever scheduler last reviewed them, so switching is never destructive.
- **Lapse & leech tracking** — cards you keep failing get flagged instead of silently piling up.
- **Multiple card types** — Basic (prompt/answer), Cloze deletion (with Anki-style `{{c1::answer::hint}}` hints), syntax-highlighted Code snippets, and Image cards (picked via a native file dialog and copied into the vault's own data directory, so they survive the source file moving or being deleted).
- **Knowledge graph** — link related memories and explore the connections in a draggable, zoomable force-directed graph.
- **Analytics** — retention rate, review streaks, a GitHub-style review heatmap, retention-over-time, and per-item forgetting-curve projections (using real FSRS stability when available, a synthetic estimate otherwise).
- **Gamification** — XP, levels, and achievements derived live from your review history (nothing to get out of sync — there's no separate XP counter to corrupt).
- **Two themes** — a clean "neural" default and an optional "blackbeard" pirate theme, both built on the same accessible color system.
- **Backup/restore** — manual snapshots, automatic pre-import/pre-restore safety backups, rotating history (last 10 kept).
- **Keyboard-driven review** — reveal, score 0-5, and move to the next card without touching the mouse. Post-score feedback shows the resulting interval before advancing.

## Tech stack

| Layer | Tech |
|---|---|
| Core logic | Rust (`synapse-core`): domain model, SM-2 + FSRS-6 schedulers, stats, persistence |
| Desktop shell | [Tauri](https://tauri.app/) v1 (`src-tauri`) |
| Frontend | [Svelte 5](https://svelte.dev/) (runes) + TypeScript + [Tailwind CSS](https://tailwindcss.com/) v4 + [Prism.js](https://prismjs.com/) (code highlighting) |
| Build tool | [Vite](https://vitejs.dev/) |

## Project structure

```
synapse/
├── synapse-core/       # Pure Rust domain logic — no Tauri/UI dependency
│   └── src/
│       ├── domain.rs        # MemoryItem, CardContent, ReviewLogEntry
│       ├── scheduler.rs     # Scheduler trait + Sm2Scheduler
│       ├── fsrs.rs          # FsrsScheduler (FSRS-6, ported from the official reference)
│       ├── store.rs         # MemoryStore trait, atomic JSON persistence
│       ├── settings.rs      # User preferences (review limit, theme, scheduler choice)
│       ├── window_state.rs  # Remembered window size/position
│       ├── backup.rs        # Rotating snapshots, restore
│       ├── stats.rs         # Retention, streaks, heatmap, forgetting curve
│       ├── graph.rs         # Knowledge graph (linking, traversal)
│       ├── gamification.rs  # XP/level/title/achievements
│       ├── assets.rs        # Copies picked images into the vault's own data dir
│       ├── persistence.rs   # Shared atomic read/write JSON helpers
│       └── error.rs         # SynapseError (typed, frontend-serializable)
├── src-tauri/           # Tauri command surface + app bootstrap (package: "synapse")
│   └── src/main.rs
├── src/                 # Svelte frontend
│   └── lib/
│       ├── api.ts            # Typed wrapper around Tauri's invoke()
│       ├── colors.ts          # Validated categorical palette (track/graph colors)
│       └── components/       # Dashboard, ReviewSession, CardView, TrackManager,
│                              # KnowledgeGraph, Insights, SettingsPanel, ConfirmDialog, ...
└── Synapse_Task_Codex.docx   # Original project brief / phased roadmap
```

## Getting started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) 18+ and npm
- `tauri-cli`: `cargo install tauri-cli --version "^1"`

On Windows, if `cargo` fails to reach crates.io with an SSL/revocation error, add this to `.cargo/config.toml` (already included in this repo):
```toml
[http]
check-revoke = false
```

### Development

```bash
npm install
npm run dev           # starts the Vite dev server on :1420
cargo tauri dev        # in a second terminal — launches the desktop window
```

### Testing

```bash
cargo test --workspace   # Rust unit tests (synapse-core)
npx svelte-check          # frontend type checking
npm run build              # production frontend build
```

### Building a release binary

```bash
cargo build --release --workspace
```

The release profile (workspace `Cargo.toml`) enables LTO, single codegen unit, and symbol stripping for a smaller, faster binary (`target/release/synapse.exe` on Windows, ~7MB). `cargo tauri build` should produce a full platform installer using the same profile, but that path hasn't been exercised end-to-end yet — see "Not yet built" below.

## Data location

Synapse stores its data in the OS-standard app-data directory (via Tauri's `app_data_dir`), e.g. `%APPDATA%/com.synapse.app` on Windows. Inside:

- `memories.json` — your vault
- `settings.json` — preferences
- `window_state.json` — last window size/position
- `backups/` — rotating snapshots (kept: last 10)

## Project status

Built in phases, per the original brief in `Synapse_Task_Codex.docx`. All five are complete:

- [x] **Phase 0** — Foundation & architecture (module boundaries, typed errors, atomic persistence)
- [x] **Phase 1** — Core hardening (stats engine, leech tracking, import/export)
- [x] **Phase 2** — Tauri backend (settings, track management, review-session orchestration)
- [x] **Phase 3** — Frontend bootstrap (Svelte 5 + Tailwind, dashboard, keyboard-driven review, theming)
- [x] **Phase 4** — Advanced features (card types, knowledge graph, analytics, gamification)
- [x] **Phase 5** — Hardening, performance & distribution (release profile, robust backup/restore, window-state persistence, docs)

### Beyond the original roadmap

Two follow-up passes after Phase 5 wrapped:

- **Frontend depth pass** — post-score interval feedback, Anki-style cloze hints, real Prism.js syntax highlighting, and a draggable/zoomable knowledge graph.
- **FSRS-6 scheduler** — a second `Scheduler` implementation alongside SM-2, selectable per-vault in Settings. Ported directly from the official reference implementation's source, not a paraphrase (see `synapse-core/src/fsrs.rs` for why that distinction mattered).
- **Image asset management** — picked images are copied into the vault's own data directory instead of referenced by their original path (`synapse-core/src/assets.rs`).

### Not yet built

Reviewed and consciously deferred, not overlooked:

- **Multi-device sync** — by design this app is local-first; the only cross-device path today is manual export/import of an unencrypted JSON file.
- **Mobile companion** — this app targets Tauri v1 (desktop). A mobile build would mean Tauri v2, a separate migration.
- **Performance at scale** — untested with thousands of items. The JSON-full-load-into-memory architecture is simple and fine for personal use, but hasn't been benchmarked at scale.
- **Real distribution** — `cargo tauri build` (installer bundling), auto-update, and code signing are unfinished. Signing in particular needs a purchased identity/certificate — a deliberate decision, not a default next step.
