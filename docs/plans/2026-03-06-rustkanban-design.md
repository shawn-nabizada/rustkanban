# RustKanban TUI - Application Design

## Context

Design document for a Rust-based terminal UI kanban board application. The goal is a fast, keyboard-driven task manager with three fixed columns, local SQLite storage, and a rich visual style. This document captures the full design before any code is written.

---

## Overview

RustKanban is a terminal-based kanban board. Tasks live in three hardcoded columns — **Todo**, **In Progress**, and **Done** — and are moved between them with keyboard shortcuts. All data is persisted in a local SQLite database. The UI is rich and colorful with Unicode box-drawing characters and color-coded priority indicators.

---

## Data Model

### Task
| Field       | Type              | Notes                                      |
|-------------|-------------------|---------------------------------------------|
| id          | INTEGER (PK)      | Auto-incrementing                           |
| title       | TEXT              | Required, displayed on the card             |
| description | TEXT              | Optional, shown in detail/edit view         |
| priority    | TEXT              | "Low" / "Medium" / "High"                  |
| column      | TEXT              | "todo" / "in_progress" / "done"             |
| due_date    | TEXT (ISO 8601)   | Optional, e.g. "2026-03-15"                |
| created_at  | TEXT (ISO 8601)   | Set automatically on creation               |
| updated_at  | TEXT (ISO 8601)   | Updated on every edit/move                  |

### Tag
| Field | Type         | Notes                    |
|-------|--------------|--------------------------|
| id    | INTEGER (PK) | Auto-incrementing        |
| name  | TEXT UNIQUE   | The tag label            |

### Task-Tag (join table)
| Field   | Type    | Notes               |
|---------|---------|----------------------|
| task_id | INTEGER | FK to Task           |
| tag_id  | INTEGER | FK to Tag            |

---

## Layout

```
+-------------------+-------------------+-------------------+
|      Todo (3)     |  In Progress (1)  |      Done (5)     |
+-------------------+-------------------+-------------------+
| > [H] Fix login   | [M] Write tests   | [L] Setup CI      |
|   [M] Add search  |                   | [H] Auth module    |
|   [L] Docs update |                   | [M] Refactor DB    |
|                   |                   | [L] README         |
|                   |                   | [L] Linting        |
+-------------------+-------------------+-------------------+
                                              Sort: Priority
```

- Three equal-width columns that scale with terminal width
- Each column header shows column name and task count
- Each task card shows: priority indicator `[H/M/L]`, title, and optionally due date and tags
- The currently highlighted task has a visible cursor marker (`>`)
- The currently focused column header is highlighted
- A small sort mode indicator in the bottom-right
- When a column has more tasks than fit on screen, the column scrolls to keep the highlighted task visible. Scroll indicators (arrows) show when there are tasks above or below the visible area

---

## Color Scheme

| Element              | Color                          |
|----------------------|--------------------------------|
| High priority `[H]`  | Red / bold                     |
| Medium priority `[M]` | Yellow                         |
| Low priority `[L]`    | Green                          |
| Selected task (K'd)   | Bright/inverse highlight       |
| Cursor/highlighted    | Underline or subtle background |
| Column headers        | Bold, focused column brighter  |
| Due today             | Bold red                       |
| Due within 3 days     | Orange / yellow                 |
| Overdue               | Red + strikethrough            |
| Due date (no urgency) | Dim/white                      |
| Tags                  | Cyan                           |

---

## Keybindings

### Board Navigation (default mode)
| Key         | Action                                          |
|-------------|-------------------------------------------------|
| H / Left    | Move focus to column on the left                |
| L / Right   | Move focus to column on the right               |
| Up          | Move cursor up within current column            |
| Down        | Move cursor down within current column          |
| K           | Select highlighted task (enters "selected" mode)|
| Enter       | Open detail view for highlighted task           |
| Space       | Open "New Task" modal                           |
| E           | Open "Edit Task" modal for highlighted task     |
| P           | Cycle priority of highlighted task              |
| D           | Delete highlighted task (with Y/N confirmation) |
| S           | Open sort/filter menu                           |
| T           | Open tag management screen                      |
| /           | Open search/filter bar (type to filter tasks)   |
| Ctrl+Z      | Undo last action                                |
| ?           | Toggle help bar at the bottom                   |
| Q           | Quit the application                            |

### Selected Mode (after pressing K on a task)
| Key         | Action                                          |
|-------------|-------------------------------------------------|
| H / Left    | Move selected task one column to the left       |
| L / Right   | Move selected task one column to the right      |
| K           | Deselect task (return to board navigation)      |
| Esc         | Deselect task (return to board navigation)      |

When a task is moved, the cursor **follows the task** to the destination column (task remains selected, so you can tap L L to move two columns at once).

### Modal (New/Edit Task)
| Key         | Action                                          |
|-------------|-------------------------------------------------|
| Tab         | Move to next field                              |
| Shift+Tab   | Move to previous field                          |
| Enter       | Newline (in text fields)                        |
| Ctrl+Enter  | Save and close modal                            |
| Esc         | Cancel and close modal                          |

---

## Screens & Modals

### 1. Main Board
The default view. Three columns side by side, column + task cursor navigation.

**Navigation details:**
- Empty columns can be focused (header highlighted) but have no task cursor. Space always creates tasks in the **Todo** column regardless of which column is focused.
- Cursor stops at the top/bottom of a column (no wrapping)
- Each column remembers its cursor position — switching back to a column puts you where you left off
- Default sort on startup: **Due Date (Soonest first)**
- Sort mode is **global** (applies to all three columns at once)

### 2. New/Edit Task Modal
A centered overlay modal with the following fields:
- **Title** — single-line text input (required). If empty on save, the field is highlighted red with an inline error and the modal stays open.
- **Description** — multi-line text area (optional). Enter inserts newlines. Ctrl+Enter saves.
- **Priority** — cycle selector: Low / Medium / High. Default for new tasks: **Medium**.
- **Due Date** — structured fields: Year / Month / Day (optional, each is a small input)
- **Tags** — pick from managed tag list via checkboxes/toggle

The modal is the same for creating and editing. When editing, fields are pre-populated. New tasks are always created in the **Todo** column.

### 3. Sort/Filter Menu
A small popup triggered by `S` with these options:
- Sort by Priority (High first)
- Sort by Due Date (Soonest first)
- Filter by Tag (shows sub-list of tags to pick)

The active sort/filter is indicated on the main board.

### 4. Tag Management Screen
Full-screen overlay triggered by `T`:
- List of all existing tags
- Keybindings: `Space` to add new tag, `D` to delete tag, `E` to rename tag
- Deleting a tag silently removes it from all tasks that had it (no per-task confirmation)
- Changes are reflected immediately in the database

### 5. Delete Confirmation
A small centered dialog: "Delete task '{title}'? (Y/N)"

### 6. Task Detail View
Opened by pressing `Enter` on a highlighted task. A read-only overlay showing:
- Title, description (full text), priority, due date, tags, created/updated timestamps
- Press `Esc` or `Enter` to close and return to the board
- Press `E` from detail view to jump directly into edit modal

### 7. Search/Filter Bar
Opened by pressing `/`. A text input appears at the top or bottom of the board:
- Type to filter tasks across all columns by title or description (case-insensitive substring match)
- Matching tasks are shown; non-matching tasks are hidden
- Press `Enter` to lock the filter and navigate results
- Press `Esc` to clear the filter and return to the full board
- While the filter is active, all actions work normally on visible tasks (move, edit, delete, etc.)

### 8. Help Bar
Toggled with `?`. Appears at the bottom of the screen showing context-sensitive keybindings for the current mode (board navigation, selected mode, or modal).

---

## Undo

- **Ctrl+Z** undoes the last action
- Undo stack holds up to 20 actions (in-memory, not persisted across sessions)
- Undoable actions: task move, task delete, task edit, priority change
- Undo restores the previous state of the affected task (or re-creates it if deleted)
- A brief flash message confirms what was undone (e.g., "Undone: moved 'Fix login' back to Todo")

---

## Due Date Warnings

Tasks with due dates get visual treatment based on urgency:
- **Overdue**: Red text + strikethrough on the due date. The date is shown prominently on the task card.
- **Due today**: Bold red date indicator
- **Due within 3 days**: Orange/yellow date indicator
- **No urgency / far out**: Dim white date, shown subtly

These indicators are visible on task cards in the main board view and in the detail view.

---

## Task Movement & Ordering

- Tasks move **only left and right** between columns using H and L (while selected with K)
- A task cannot move left of "Todo" or right of "Done"
- When a task lands in a new column, its position within that column is determined by the **currently active sort/filter**
- If sorted by priority: High tasks appear at top, Low at bottom
- If sorted by due date: Soonest deadlines appear at top, tasks without due dates at bottom
- If filtered by tag: Only tasks with the selected tag are shown; order falls back to priority

---

## Storage

- **Database**: SQLite via `rusqlite`
- **Location**: `~/.local/share/rustkanban/kanban.db`
- The directory and database are created automatically on first run
- Schema is created via migrations on startup
- All operations (create, edit, move, delete) are persisted immediately

---

## CLI

Binary name: **`rk`** (cargo package: `rustkanban`)

### Commands
| Command                        | Description                                     |
|--------------------------------|-------------------------------------------------|
| `rk`                           | Launch the TUI kanban board                     |
| `rk reset`                     | Delete all tasks and tags (with Y/N confirmation)|

CLI parsing via `clap`.

---

## Terminal Behavior

- Layout auto-resizes when the terminal is resized
- If terminal is below a minimum size (e.g., 80 columns x 24 rows), display a centered "Terminal too small" message instead of a broken layout
- Columns are equal-width and scale with available terminal width

---

## Technology Choices

| Concern       | Choice         | Rationale                                  |
|---------------|----------------|---------------------------------------------|
| Language      | Rust           | User requirement                            |
| TUI framework | Ratatui        | Most actively maintained Rust TUI library   |
| Event loop    | crossterm      | Ratatui's recommended backend               |
| Database      | rusqlite       | Lightweight, no server, widely used         |
| Date handling | chrono         | Standard Rust date/time library             |
| CLI parsing   | clap           | Standard Rust CLI argument parser           |

---

## Verification

Once implemented, verify by:
1. `cargo build` compiles without errors
2. `rk` launches the TUI with three empty columns
3. Space creates a task in the Todo column (regardless of focused column)
4. H/L moves between columns (including empty ones), Up/Down moves between tasks
5. Each column remembers its cursor position when switching away and back
6. Cursor stops at top/bottom of columns (no wrap)
7. K selects a task, H/L moves it between columns (cursor follows), K deselects
8. Enter opens detail view for a task, E from detail view opens edit modal
9. E opens edit modal; Enter inserts newlines in description; Ctrl+Enter saves
10. Saving with empty title shows red highlight error and modal stays open
11. New tasks default to Medium priority
12. P cycles priority and re-sorts
13. D prompts for confirmation, Y deletes
14. S opens sort menu, changing sort reorders tasks (global across all columns)
15. Default sort on startup is Due Date (Soonest first)
16. / opens search bar, typing filters tasks; all actions work on filtered results
17. T opens tag management; deleting a tag silently removes it from all tasks
18. ? toggles the help bar
19. Q quits cleanly
20. Restarting the app shows all previously created tasks (SQLite persistence)
21. `rk reset` prompts for confirmation, then clears the database
22. Resizing terminal below 80x24 shows "Terminal too small" message
23. Columns with overflow show scroll indicators (arrows)
24. Ctrl+Z undoes the last task action (move, delete, edit, priority change)
25. Tasks with due dates show appropriate urgency coloring (overdue, today, within 3 days)
