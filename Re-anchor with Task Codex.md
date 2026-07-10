Re-anchor with Task Codex

GOAL: Transform the Synapse Tauri + Rust workspace into the most polished, powerful, beautiful local-first Personal Training & Memory OS possible.

LAST TASK: Post-roadmap frontend depth pass (review session feedback, cloze hints, Prism syntax highlighting, draggable/zoomable graph) and an FSRS-6 scheduler added as a second `Scheduler` alongside SM-2, selectable per-vault in Settings.

CURRENT STATE: All 5 original phases complete and independently verified end-to-end through the real desktop app (not just typechecks). Rust core (`synapse-core`) has domain/scheduler/fsrs/store/settings/window_state/backup/stats/graph/gamification/persistence/error modules, 38/38 tests passing. Tauri command surface in `src-tauri/src/main.rs` covers memories, review sessions, stats, tracks, settings, export/import, backup/restore, knowledge graph, and gamification. Svelte 5 frontend has Dashboard, ReviewSession, CardView, TrackManager, KnowledgeGraph, Insights, SettingsPanel, ConfirmDialog. Release build produces `synapse.exe` (~7MB, LTO-tuned). Full details and a running log of what's been verified vs. assumed: `README.md`.

NEXT: Nothing mandated by the roadmap — it's done. Open, consciously-deferred items (see README "Not yet built"): image asset management, multi-device sync, mobile companion, performance at scale, and real distribution (installer/auto-update/code signing). Don't start any of these without the user picking one explicitly.

PHASE: Post-roadmap.

BLOCKERS: None.

---

Notes on working in this repo, for whoever (human or Claude) resumes next:

- This machine needed `.cargo/config.toml` with `http.check-revoke = false` to get `cargo build` working at all (a Windows/schannel SSL quirk) — already committed, shouldn't need to rediscover this.
- `tauri-cli` is installed (`cargo-tauri.exe` in `~/.cargo/bin`), so `cargo tauri dev`/`cargo tauri build` work directly.
- To actually verify a change in the real app (not just build/typecheck), launch `npm run dev` + `cargo tauri dev`, then drive the window via Windows UI Automation rather than screenshots + mouse coordinates: find the real window handle via `EnumWindows` matching the title (`Get-Process -Name synapse | Select MainWindowHandle` returns a stale/wrong handle here), the window launches minimized so cycle `ShowWindow(hwnd, 6)` → `ShowWindow(hwnd, 9)` → `SetForegroundWindow(hwnd)` → sleep ~800ms before each query to wake WebView2's lazy accessibility tree, then use `FindFirst`/`InvokePattern` or `SelectionItemPattern`/`TogglePattern` (nav tabs and toggle-style buttons don't support `InvokePattern` — check `GetSupportedPatterns()` if unsure) to interact. Always clean up: kill `synapse.exe`/`cargo-tauri.exe`/the vite node process afterward, and reset any real `%APPDATA%\com.synapse.app\` files touched during testing back to their prior state.
- When precision actually matters (like FSRS's ~21 magic-number weights), don't trust memorized/paraphrased values — pull the primary source (e.g. via `gh api repos/<org>/<repo>/contents/<path>`) and verify against it directly.
