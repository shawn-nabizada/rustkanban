# RustKanban Development Guide

## Build & Test
- `cargo build` — compile
- `cargo test` — run all tests
- `cargo test test_name` — run a single test
- `cargo clippy -- -D warnings` — lint (must pass with zero warnings)
- `cargo fmt` — format
- `cargo fmt -- --check` — verify formatting (CI runs this)
- `cargo install --path crates/rk-client` — install locally as `rk`
- `vhs demo.tape` — regenerate the demo GIF (requires [vhs](https://github.com/charmbracelet/vhs))
- `rk manpage | man -l -` — preview the man page

## Justfile (recommended)
All common tasks are available via [just](https://github.com/casey/just):
- `just build` — compile
- `just test` / `just test test_name` — run tests
- `just lint` — clippy with zero warnings
- `just fmt` — format
- `just check` — fmt + clippy + test (mirrors CI)
- `just install` — install locally as `rk`
- `just release X.Y.Z` — create release commit + tag
- `just dev` — start local sync server on port 3000
- `just demo` — regenerate demo GIF
- `just manpage` — preview man page
- `just clean` — clean build artifacts

## Local Server Development
1. Copy `crates/rk-server/.env.example` to `.env` in the project root
2. Fill in `GITHUB_CLIENT_ID` and `GITHUB_CLIENT_SECRET` (create an OAuth app at https://github.com/settings/developers, callback URL: `http://localhost:3000/auth/callback`)
3. Run `just dev` — starts PostgreSQL + Axum server on `http://localhost:3000`
4. Server auto-runs migrations on startup

## CLI Subcommands
- `rk` — launch TUI
- `rk reset` — delete all tasks, tags, and boards (prompts Y/N, recreates Personal board)
- `rk export` — JSON to stdout
- `rk import <file>` — import from JSON file
- `rk theme` — print default theme TOML
- `rk theme --init` — create `~/.config/rustkanban/theme.toml`
- `rk completions <shell>` — generate shell completions (bash/zsh/fish/powershell)
- `rk manpage` — output man page to stdout
- `rk login` — authenticate with sync service (GitHub OAuth)
- `rk logout` — log out from sync service
- `rk sync` — sync with server (pull + push)
- `rk status` — show sync status
- `rk update` — self-update to the latest version (`--force` to skip cargo install detection)

## Pre-commit Hook
```sh
git config core.hooksPath .githooks
```
Runs `cargo fmt --check` and `cargo clippy -- -D warnings` before each commit.

## Architecture
Project is a Cargo workspace with three crates:
- `crates/rk-client` — TUI app (binary: `rk`)
- `crates/rk-server` — Axum sync server (binary: `rk-server`)
- `crates/rk-shared` — Shared sync types

Single-threaded TUI app. Event loop: render → poll (100ms) → handle → tick → repeat.
Mouse capture enabled on start, disabled on quit (with panic hook for safe cleanup).
Sync is opt-in. The app works fully offline without an account. If logged in, auto-pull on startup, auto-push on quit.

### Module Map
- `main.rs` — CLI (clap), terminal setup, event loop, panic-safe restore
- `app.rs` — App state struct, all business logic methods
- `db.rs` — SQLite CRUD (rusqlite), preferences table. All mutations go through DB first, then `reload_tasks()`
- `model.rs` — Task, Tag, Board, Priority, Column types
- `handler.rs` — Input dispatch: matches AppMode, delegates to App methods. Includes `handle_mouse()`
- `event.rs` — `AppEvent` enum (Key | Mouse), crossterm event polling

- `export.rs` — JSON export/import (serde)
- `theme.rs` — Theme config from ~/.config/rustkanban/theme.toml
- `auth.rs` — Credential management, GitHub OAuth login flow
- `sync.rs` — Sync client (pull/push/combined via ureq)
- `update.rs` — Version check (GitHub API, 24h cooldown) and self-update logic
- `ui/` — All rendering. `mod.rs` is entry point, delegates to submodules (board, modal, detail, sort_menu, tag_screen, search_bar, help_bar, delete_confirm, tab_bar, board_mgmt)
- `crates/rk-server/static/` — Static assets (CSS) served by Axum
- `crates/rk-server/templates/` — Askama HTML templates

### Key Patterns
- **State machine**: `AppMode` enum drives which handler + UI overlay is active
- **DB-first**: mutate DB, call `reload_tasks()`, never cache separately
- **Theme**: `app.theme` has all colors. Use `app.theme.priority_color(&p)` not hardcoded colors
- **Tests**: use `db::init_db_memory()` for in-memory SQLite
- **Preferences**: use `PREF_*` constants in `app.rs` for key names, `SortMode::as_str()`/`from_str()` for serialization
- **Move task**: use `app.move_task_to_column()` — shared by keyboard selection and mouse drag
- **Task height**: use `task_visual_height()` — shared by scroll calculation and mouse hit detection
- **Search highlight**: `highlight_matches()` uses char-level byte-offset mapping for Unicode safety
- **Soft deletes**: `soft_delete_task()`/`soft_delete_tag()` set `deleted=1`, `load_tasks()`/`load_tags()` filter them out. `load_all_tasks()`/`load_all_tags()` include deleted.
- **UUIDs**: All tasks, tags, and boards have UUIDs (v4). Used for sync identity and export dedup.
- **Boards**: Up to 5 named boards per user. Tasks belong to a board via `board_id`. Tags are global (shared across boards). Tab bar at top, `B` key for management, `1-5` to switch.

### Key Data Paths
- Database: `~/.local/share/rustkanban/kanban.db`
- Theme: `~/.config/rustkanban/theme.toml`
- Preferences: `preferences` table in the SQLite database (key-value)
- Credentials: `~/.config/rustkanban/credentials.json`
- Default sync server: `https://sync.rustkanban.com`

## Environment
- **Required**: Rust stable toolchain. SQLite is bundled via rusqlite (no system install needed).
- **Optional**: [vhs](https://github.com/charmbracelet/vhs) for demo GIF recording
- No `.env` required. Fully offline by default. Network access used only for opt-in sync.
- Minimum terminal size: 80×30 (shows error message if smaller)

## Conventions
- No `unwrap()` in production code (ok in tests)
- Keep `handler.rs` thin — it maps keys to App methods, no logic
- All user-facing strings in render code, not in App methods (except flash messages)
- `cargo clippy` and `cargo fmt --check` must pass before commit

## Gotchas
- New tasks go to **active board's Todo** column regardless of which column is focused
- In modal: `Enter` = newline in description field, `Ctrl+S` = save (not Enter)

- **Tag deletion** soft-deletes the tag and silently removes it from all tasks

- **Import** is additive — never replaces or modifies existing tasks/tags
- Mouse drag deselects the task after moving (returns to Board mode)
- `Ctrl+R` triggers manual sync (pull + push) from within the TUI
- Auto-pull on TUI startup, auto-push on quit (if logged in)
- Schema auto-migrates v1→v2 (backfills UUIDs) and v2→v3 (adds boards table, backfills tasks into "Personal" board)
- Text validation limits: 500 chars for title, 5000 chars for description, 50 chars for tag name
- Last-write-wins conflict resolution for sync

## Documentation Maintenance
After any feature change, bug fix, or behavioral modification, review and update these files as needed:
- `README.md` — features list, keybindings tables, usage section, install instructions
- `CHANGELOG.md` — add entry under `[Unreleased]` describing the change
- `docs/USE_CASES.md` — add/update use case steps for affected functionality
- `demo.tape` — update VHS recording script if the UI or workflow changed

