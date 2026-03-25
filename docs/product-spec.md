# RustKanban Product Spec (Reverse-Engineered)

> **Generated:** 2026-03-24 | **Updated:** 2026-03-24
> **Source:** Codebase analysis of RustKanban v0.2.0
> **Status:** Reverse-engineered from existing code, not a forward-looking plan

---

## Product Summary

RustKanban is a terminal-based (TUI) kanban board for personal task management. It runs fully offline with local SQLite storage, and optionally syncs across machines via a self-hosted or managed server using GitHub OAuth. A companion web app provides browser-based access to the same data.

**Target user:** Developers and terminal-power-users who want a fast, keyboard-driven task board without leaving the terminal.

**Core value prop:** A kanban board that lives in your terminal, works offline, and syncs when you want it to.

---

## Feature Inventory

### Task Management (Core)

| Feature | Description | Location |
|---------|-------------|----------|
| **Create task** | Space key opens modal. Task always goes to active board's Todo column regardless of focused column. Fields: title (500 char limit), description (5000 char limit), priority, tags, due date. | `app/modal.rs`, `ui/modal.rs` |
| **Edit task** | E key opens same modal pre-filled. Ctrl+S saves, Esc cancels. Enter inserts newline in text fields. | `handler.rs`, `ui/modal.rs` |
| **Delete task** | d key with Y/N confirmation dialog. Soft-deletes (marks `deleted=1`). | `app/mod.rs`, `ui/delete_confirm.rs` |
| **Duplicate task** | C key clones task (new UUID, same content) into Todo column. | `app/mod.rs` |
| **Move task between columns** | K to select, H/L to move left/right between Todo/InProgress/Done. Cursor follows the moved task. | `handler.rs` (handle_selected) |
| **View task detail** | Enter key opens read-only detail overlay showing all fields. | `ui/detail.rs` |
| **Priority** | Three levels: Low, Medium, High. Default: Medium. Cycle with P key. Color-coded indicators [L/M/H]. | `model.rs`, `app.rs` |
| **Due dates** | Optional date per task. Year/Month/Day input in modal. Color-coded urgency: overdue (red), today, soon (3 days), far. | `ui/modal.rs`, `ui/board.rs` |
| **Drag and drop** | Mouse drag tasks between columns. Shows skeleton at drop target. Deselects after drop. | `handler.rs` (handle_mouse) |

### Organization

| Feature | Description | Location |
|---------|-------------|----------|
| **Multiple boards** | Up to 5 named boards per user. Tab bar at top. Switch with 1-5 keys. B key for management (create/rename/delete). | `app/boards.rs`, `ui/board_mgmt.rs`, `ui/tab_bar.rs` |
| **Tags** | Global tag list shared across boards. T key for management (create/rename/delete). Tags assigned per-task via checkbox in modal. Deleting a tag silently removes it from all tasks. Max 50 chars per tag name. | `app/tags.rs`, `ui/tag_screen.rs` |
| **Sort** | Global sort across all columns. Two modes: Due Date (soonest first) or Priority (highest first). S key opens sort menu. Persisted in preferences. | `app/mod.rs`, `ui/sort_menu.rs` |
| **Tag filter** | Filter board to show only tasks with a specific tag. Set via sort menu. | `ui/sort_menu.rs` |
| **Search** | `/` key activates live search. Filters tasks by title/description match. Case-insensitive. Highlights matches with underline. Full interaction while filter active. | `app/search.rs`, `ui/search_bar.rs` |
| **Clear done** | Bulk-delete all tasks in the Done column (with confirmation). | `app.rs`, `ui/delete_confirm.rs` |

### Sync & Auth

| Feature | Description | Location |
|---------|-------------|----------|
| **GitHub OAuth login** | `rk login` opens browser for GitHub OAuth. Creates user + device + auth token. | `auth.rs`, server `routes/auth.rs` |
| **Headless login** | For SSH/no-browser environments: shows URL + accepts token manually. Also supports `--token`/`--device-id` flags. | `auth.rs` |
| **Auto-sync** | Auto-pull on TUI startup, auto-push on quit (if logged in). | `main.rs` |
| **Manual sync** | Ctrl+R in TUI or `rk sync` CLI. Combined pull+push in one round trip. | `sync.rs` |
| **Sync status** | Status bar shows sync state: NotLoggedIn, Idle (with last sync time), Syncing, Error. | `app.rs`, `ui/mod.rs` |
| **Multi-device** | Up to 5 devices per account. Each has named identity and last-sync timestamp. | Server schema |
| **Conflict resolution** | Last-write-wins on `updated_at` timestamp. Entire task replaced, not field-level merge. | `sync.rs`, server `routes/sync.rs` |
| **Stale device recovery** | Devices not synced in 90+ days get full re-sync instead of delta. | Server `routes/sync.rs` |

### Web App

| Feature | Description | Location |
|---------|-------------|----------|
| **Board view** | Svelte SPA with kanban layout. Drag-and-drop task cards. Create/edit/delete tasks. | `frontend/src/components/Board.svelte` |
| **Board sharing** | Generate share links (view or edit permission). Guests access via `/shared/{token}` without login. | `frontend/src/components/ShareModal.svelte`, server `routes/shares.rs` |
| **Account management** | View/rename/revoke devices. Create/revoke API tokens. Export data. Delete account. | `frontend/src/routes/Account.svelte` |
| **Tag management** | Create/rename/delete tags via web interface. | `frontend/src/components/TagManager.svelte` |

### Customization

| Feature | Description | Location |
|---------|-------------|----------|
| **Configurable keybindings** | 27 actions mappable to any key combo. TOML config at `~/.config/rustkanban/keys.toml`. O key opens in-app editor. | `keybindings.rs`, `app/options.rs`, `ui/options.rs` |
| **Theme** | 16 configurable colors (borders, priorities, tags, due dates, modals). TOML config at `~/.config/rustkanban/theme.toml`. In-app color cycling in Options. | `theme.rs`, `app/options.rs`, `ui/options.rs` |

### CLI

| Feature | Description | Location |
|---------|-------------|----------|
| **TUI launch** | `rk` (no args) launches the terminal UI. | `main.rs` |
| **Reset** | `rk reset` deletes all tasks, tags, boards. Prompts Y/N. Recreates "Personal" board. | `main.rs` |
| **Export** | `rk export` outputs all data as JSON to stdout. Version 2 format with UUIDs. | `export.rs` |
| **Import** | `rk import <file>` adds tasks/tags/boards from JSON. Additive only (never replaces/modifies existing). Deduplicates by UUID. | `export.rs` |
| **Shell completions** | `rk completions <bash|zsh|fish|powershell>` generates completion scripts. | `main.rs` |
| **Man page** | `rk manpage` outputs man page to stdout. | `main.rs` |
| **Self-update** | `rk update` checks GitHub API for latest release (24h cooldown). Downloads and installs via cargo. `--force` skips cargo detection. | `update.rs` |
| **Sync commands** | `rk login`, `rk logout`, `rk sync`, `rk status` for auth/sync management. | `main.rs`, `auth.rs`, `sync.rs` |
| **Config init** | `rk theme [--init]`, `rk keys [--init]` for printing/creating config files. | `main.rs` |

---

## Page/Screen Map

### TUI Screens (AppMode-driven)

| Screen | Trigger | What User Sees | What User Can Do |
|--------|---------|----------------|------------------|
| **Board** (default) | Launch / Esc from modals | 3 kanban columns (Todo, In Progress, Done) with task cards. Tab bar at top. Status bar at bottom. | Navigate (H/L/Up/Down), create (Space), edit (E), delete (d), select (K), sort (S), search (/), tags (T), boards (B), options (O), detail (Enter), sync (Ctrl+R), quit (Q) |
| **Selected** | K on a task | Same as Board but selected task highlighted differently | Move task (H/L), deselect (K/Esc), cycle priority (P) |
| **NewTask** | Space | Centered modal (60%x70%) with fields: Title, Description, Priority, Tags (checkboxes), Due Date (Y/M/D) | Tab between fields, type text, toggle tags, Ctrl+S save, Esc cancel |
| **EditTask** | E on a task | Same modal, pre-filled with task data | Same as NewTask |
| **DetailView** | Enter on a task | Full-screen overlay with all task fields (read-only) | Esc to close |
| **SortMenu** | S | Overlay listing sort options (Due Date, Priority) and tag filter options | Up/Down to select, Enter to apply, Esc to close |
| **DeleteConfirm** | d on a task | "Delete task? (y/n)" overlay | Y to confirm, N/Esc to cancel |
| **ClearDoneConfirm** | X on Done column | "Clear all done tasks? (y/n)" overlay | Y to confirm, N/Esc to cancel |
| **TagManagement** | T | Full-screen tag list with create/rename/delete | N new, R rename, D delete, Esc back |
| **SearchFilter** | / | Search bar below board with live filtering | Type query, Enter to lock, Esc to clear and close |
| **BoardManagement** | B | Overlay listing boards with create/rename/delete | N new, R rename, D delete, Esc back |
| **BoardDeleteConfirm** | D in BoardManagement | "Delete board? (y/n)" overlay | Y to confirm (deletes board + all its tasks), N/Esc to cancel |
| **Options** | O | Split view: left side keybindings editor, right side theme color editor | Navigate, rebind keys, cycle colors, Ctrl+S save, Esc cancel |

### Web Pages (Svelte SPA)

| Page | URL | What User Sees | What User Can Do |
|------|-----|----------------|------------------|
| **Landing** | `/` (logged out) | Marketing/info page | Log in via GitHub |
| **Home/Dashboard** | `/` (logged in) | Board list | Select a board, navigate to board view |
| **Board View** | `/app` | Kanban board with 3 columns, task cards | Drag-drop tasks, create/edit/delete tasks, manage tags, share board |
| **Shared Board** | `/shared/{token}` | Guest view of a shared board (no login needed) | View tasks; if edit permission: create/edit/delete tasks |
| **Account** | `/account` | Device list, API token list, export/delete options | Rename/revoke devices, create/revoke API tokens, export data, delete account |
| **Login Token** | `/login-token` | Token display for headless CLI login | Copy token + device ID |

---

## User Flow Reconstruction

### 1. First-Time Setup (Offline)
1. Install `rk` via cargo or package manager
2. Run `rk` — SQLite database auto-created at `~/.local/share/rustkanban/kanban.db`
3. Schema auto-migrates; "Personal" board created automatically
4. User sees empty kanban board with 3 columns
5. Press Space to create first task

### 2. Daily Task Management
1. Launch `rk` (auto-pulls from server if logged in)
2. Navigate columns with H/L, tasks with Up/Down
3. Space to create new tasks (always lands in Todo)
4. K to select a task, H/L to move it between columns
5. E to edit, d to delete, P to cycle priority
6. S to sort by due date or priority
7. / to search and filter
8. Q to quit (auto-pushes to server if logged in)

### 3. Multi-Board Workflow
1. Press B to open board management
2. Press N to create a new board (up to 5 total)
3. Press 1-5 or click tab bar to switch between boards
4. Tags are shared across all boards
5. Each board has independent task lists

### 4. Sync Setup (Opt-in)
1. Run `rk login` — browser opens to GitHub OAuth
2. Authorize the app — token saved to `~/.config/rustkanban/credentials.json`
3. Device registered on server with hostname
4. From now on: auto-pull on startup, auto-push on quit
5. Ctrl+R for manual sync anytime
6. `rk status` to check sync state

### 5. Headless/SSH Login
1. Run `rk login` on a machine without a browser
2. App detects no browser, prints a URL
3. User opens URL on another machine, completes OAuth
4. Server shows token + device ID on success page
5. User enters token + device ID back in terminal (or uses `rk login --token T --device-id D`)

### 6. Web Access & Sharing
1. Log in at web app (same GitHub OAuth)
2. View and manage boards/tasks in browser
3. Click Share on a board → generate view or edit link
4. Share link with collaborator → they access `/shared/{token}` without login
5. Manage devices and API tokens in Account page

### 7. Export & Import
1. `rk export > backup.json` — full JSON dump to file
2. `rk import backup.json` — additive import (skips existing UUIDs)
3. Useful for backup, migration, or sharing task sets

### 8. Customization
1. Press O in TUI to open Options screen
2. Left side: rebind any of the 27 actions to new key combos
3. Right side: cycle through preset colors for 16 theme properties
4. Ctrl+S to save (writes to `~/.config/rustkanban/keys.toml` and `theme.toml`)
5. Or manually edit TOML files; `rk theme --init` / `rk keys --init` to create defaults

---

## Discovered Feature Interactions

### Tags ↔ Boards
- Tags are **global** — shared across all boards
- Deleting a tag removes it from tasks across all boards
- Tag filter in sort menu applies to active board only

### Boards ↔ Tasks
- Each task belongs to exactly one board (via `board_id`)
- Deleting a board soft-deletes all its tasks
- New tasks always go to the **active board's** Todo column

### Sync ↔ Soft Deletes
- Sync requires soft deletes so other devices learn about deletions
- Server purge job only removes soft-deleted records after all active devices have synced past the deletion timestamp
- Stale devices (90+ days) are ignored by the purge job

### Sync ↔ Tags
- Tag names are unique per user on the server
- If two devices create a tag with the same name, server deduplicates and returns a UUID mapping
- Client applies the mapping to update local tag UUIDs

### Sync ↔ Boards
- Board names are unique per user
- Board name collisions during sync are rejected (not auto-merged)
- Board UUIDs ensure identity across devices

### Web App ↔ CLI
- Both authenticate via GitHub OAuth (different flows: browser vs headless)
- Both read/write the same server data
- Web uses session cookies; CLI uses Bearer tokens
- Board sharing is web-only (no CLI command for sharing)

### Keybindings ↔ Modal
- Modal has two contexts: Board and Modal
- Some keys are hardcoded in modal (text editing, cursor movement)
- Configurable keys: save, next/prev field, and Board-mode actions

---

## Product Gaps

### Half-Built or Missing Features

| Gap | Evidence | Severity |
|-----|----------|----------|
| **No Critical priority** | Server schema allows arbitrary priority strings, but client only supports Low/Medium/High. No "Critical" or "None" option. | Low |
| **No recurring tasks** | No recurrence field in schema or model. Common kanban feature missing. | Low |
| **No task ordering within column** | Tasks sorted only by sort mode (due date or priority). No manual drag-to-reorder within a column. | Medium |
| **No task assignment** | Single-user focus. No assignee field even though board sharing exists with edit permission. | Medium |
| **Board sharing is web-only** | No CLI command to create/manage share links. Must use web app. | Low |
| **No notifications** | No push notifications for shared board edits or sync conflicts. | Low |
| **Web app has no search** | TUI has search, web app does not appear to have search functionality. | Medium |
| **No undo** | Destructive actions (delete, clear done) have confirmation but no undo. | Low |
| **Tag limit (15) may be restrictive** | Server enforces max 15 tags per user. Power users may hit this. | Low |

---

## Appendix: Data Validation Limits

| Field | Max Length |
|-------|-----------|
| Task title | 500 characters |
| Task description | 5,000 characters |
| Tag name | 50 characters |
| Board name | 50 characters |
| Boards per user | 5 |
| Tasks per user (server) | 200 |
| Tags per user (server) | 15 |
| Devices per user | 5 |
| API tokens per user | 10 |
