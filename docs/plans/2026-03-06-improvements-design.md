# RustKanban Improvements Design

## License Change: BSL 1.1

Switch from MIT to Business Source License 1.1.

Parameters:
- **Additional Use Grant**: Non-commercial use permitted
- **Change Date**: 4 years after each release
- **Change License**: Apache 2.0

Replace LICENSE file. Update Cargo.toml `license` field to `BSL-1.1`. Update README badge.

---

## Phase 1: Developer Infrastructure

### 1. Unit & Integration Tests

All DB tests use in-memory SQLite (`:memory:`). Test modules inline (`#[cfg(test)] mod tests`) in each file:

- **db.rs**: init_db, insert/load/update/delete tasks, CRUD tags, task_tags junction, reset, preferences table
- **undo.rs**: push/pop, max capacity overflow (oldest dropped), empty pop returns None
- **export.rs**: export produces valid JSON, import round-trip preserves data, import with duplicate tags reuses existing, import preserves tag assignments
- **app.rs**: `tasks_for_column` sorting (due date and priority), search filtering, tag filtering, `clamp_cursor` edge cases
- **theme.rs**: `parse_color` for named colors, hex codes, invalid input returns None, load from TOML string

### 2. `cargo fmt` in CI

Add `cargo fmt -- --check` step to `.github/workflows/ci.yml`.

### 3. CHANGELOG.md

Manual format using Keep a Changelog convention. Initial entry for v0.1.0 listing all current features. Future entries maintained manually.

### 4. Pre-commit Hooks

`.githooks/pre-commit` shell script:
- Runs `cargo fmt -- --check`
- Runs `cargo clippy -- -D warnings`
- Users enable with `git config core.hooksPath .githooks`

### 5. CLAUDE.md

Project-root file with architecture overview, module map, conventions, and "don't do" rules.

---

## Phase 2: UX Features

### 1. Duplicate Task

- Key: `C` on board mode
- Copies: title, description, priority, tags, due date
- Creates in same column
- Cursor moves to clone
- Flash message: "Duplicated 'title'"
- Undo support: undo removes the duplicate (reuses DeleteTask undo variant)

### 2. Mouse Support (Full)

**Event layer**: Change `event.rs` to return `AppEvent` enum:
```rust
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}
```
Main loop dispatches accordingly.

**Click to focus/cursor**: Click column area -> focus that column. Click task -> move cursor to that task. Column determined by x-coordinate (width / 3 boundaries). Task determined by walking task heights from scroll offset.

**Scroll**: Scroll wheel up/down -> adjust scroll offset for column under cursor.

**Drag to move**: MouseDown on task -> starts drag (store task_id + source column in `App.drag_state`). MouseUp in different column -> moves task there + pushes undo. MouseUp in same column -> cancels drag. Enable mouse capture via crossterm.

**Layout info**: Store terminal size in App (updated each frame) so mouse handler can compute column/task hit detection.

### 3. Persistent Preferences

New DB table:
```sql
CREATE TABLE IF NOT EXISTS preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

Stored keys: `sort_mode`, `focused_column`.
- `db.rs`: add `get_preference(key) -> Option<String>`, `set_preference(key, value)`
- `App::new()`: loads preferences after DB init
- Saved on change: sort mode saved immediately, focused column saved on quit

### 4. Search Match Highlighting

When `search_active` and query is non-empty, board.rs title rendering splits text into segments: `[before_match, MATCH, after_match, ...]`. Match segments styled with bold + underline. Case-insensitive, handles multiple matches per title. Only affects board card rendering.

---

## Phase 3: Distribution

### 1. VHS Demo

Create `demo.tape` script: launch -> create task -> navigate -> move between columns -> search -> quit. Output `demo.gif` referenced in README after badges.

### 2. Update README

- Feature list: add export/import, theme config, multiple tags, mouse support, duplicate task
- Usage section: `rk export`, `rk import <file>`, `rk theme [--init]`
- Keybindings: `C` for duplicate, mouse note, fix Tag description (toggle not cycle)
- Add Theme section with config path and `rk theme --init`
- Update license badge to BSL-1.1

### 3. Homebrew Formula

Create `HomebrewFormula/rk.rb` with binary download from GitHub releases. Auto-detect macOS Intel / Apple Silicon. User creates separate `homebrew-rustkanban` tap repo.

### 4. Man Page

Add `clap_mangen` build dependency. Add `rk manpage` subcommand that writes `rk.1` to stdout. Release workflow generates and includes man page in release assets.

### 5. AUR Package

Create `aur/PKGBUILD` that downloads Linux binary from GitHub releases.

### 6. Install Script Checksums

Release workflow generates `checksums.sha256` and uploads as release asset. `install.sh` downloads checksums, verifies binary SHA256 after download. Fails with clear error on mismatch.
