# Synapse

A local-first, offline-capable desktop app for spaced-repetition training — built with a Rust core, a Tauri shell, and a Svelte 5 frontend. Track hard skills (Rust, algorithms, biology, whatever you're drilling), review on a schedule that adapts to how well you actually remember things, and see the data behind your own learning.

Everything lives on disk as JSON in your OS's app-data directory. Nothing leaves your machine.

## Features

- **Spaced repetition** — SM-2 scheduling with lapse/leech tracking, so cards you keep failing get flagged instead of silently piling up.
- **Multiple card types** — Basic (prompt/answer), Cloze deletion, Code snippets, and Image cards.
- **Knowledge graph** — link related memories together and see the connections.
- **Analytics** — retention rate, review streaks, a GitHub-style review heatmap, retention-over-time, and per-item forgetting-curve projections.
- **Gamification** — XP, levels, and achievements derived live from your review history (nothing to get out of sync — there's no separate XP counter to corrupt).
- **Two themes** — a clean "neural" default and an optional "blackbeard" pirate theme, both built on the same accessible color system.
- **Backup/restore** — manual snapshots, automatic pre-import/pre-restore safety backups, rotating history (last 10 kept).
- **Keyboard-driven review** — reveal, score 0-5, and move to the next card without touching the mouse.

## Tech stack

| Layer | Tech |
|---|---|
| Core logic | Rust (`synapse-core`): domain model, SM-2 scheduler, stats, persistence |
| Desktop shell | [Tauri](https://tauri.app/) v1 (`src-tauri`) |
| Frontend | [Svelte 5](https://svelte.dev/) (runes) + TypeScript + [Tailwind CSS](https://tailwindcss.com/) v4 |
| Build tool | [Vite](https://vitejs.dev/) |

## Project structure

```
synapse/
├── synapse-core/       # Pure Rust domain logic — no Tauri/UI dependency
│   └── src/
│       ├── domain.rs        # MemoryItem, CardContent, ReviewLogEntry
│       ├── scheduler.rs     # Scheduler trait + SM-2 implementation
│       ├── store.rs         # MemoryStore trait, atomic JSON persistence
│       ├── settings.rs      # User preferences (review limit, theme)
│       ├── window_state.rs  # Remembered window size/position
│       ├── backup.rs        # Rotating snapshots, restore
│       ├── stats.rs         # Retention, streaks, heatmap, forgetting curve
│       ├── graph.rs         # Knowledge graph (linking, traversal)
│       ├── gamification.rs  # XP/level/title/achievements
│       ├── persistence.rs   # Shared atomic read/write JSON helpers
│       └── error.rs         # SynapseError (typed, frontend-serializable)
├── src-tauri/           # Tauri command surface + app bootstrap
│   └── src/main.rs
├── src/                 # Svelte frontend
│   └── lib/
│       ├── api.ts            # Typed wrapper around Tauri's invoke()
│       └── components/       # Dashboard, ReviewSession, TrackManager,
│                              # KnowledgeGraph, Insights, SettingsPanel, ...
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
# or, for a full installer bundle:
cargo tauri build
```

The release profile (workspace `Cargo.toml`) enables LTO, single codegen unit, and symbol stripping for a smaller, faster binary.

## Data location

Synapse stores its data in the OS-standard app-data directory (via Tauri's `app_data_dir`), e.g. `%APPDATA%/com.synapse.app` on Windows. Inside:

- `memories.json` — your vault
- `settings.json` — preferences
- `window_state.json` — last window size/position
- `backups/` — rotating snapshots (kept: last 10)

## Project status

Built in phases, per the original brief in `Synapse_Task_Codex.docx`:

- [x] **Phase 0** — Foundation & architecture (module boundaries, typed errors, atomic persistence)
- [x] **Phase 1** — Core hardening (stats engine, leech tracking, import/export)
- [x] **Phase 2** — Tauri backend (settings, track management, review-session orchestration)
- [x] **Phase 3** — Frontend bootstrap (Svelte 5 + Tailwind, dashboard, keyboard-driven review, theming)
- [x] **Phase 4** — Advanced features (card types, knowledge graph, analytics, gamification)
- [ ] **Phase 5** — Hardening, performance & distribution (in progress)
