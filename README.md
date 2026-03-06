# RustKanban

A Rust terminal (TUI) kanban board with vim-inspired navigation, tags, search, and SQLite persistence.

## Features

- **3-column board** -- Todo, In Progress, Done
- **Vim-inspired navigation** -- J/L for columns, Up/Down/Tab to move between tasks
- **Task management** -- create, edit, delete, move between columns, cycle priority
- **Tags** -- create/rename/delete tags, assign to tasks, filter by tag
- **Search** -- live filter tasks by title or description
- **Sorting** -- sort by due date (default) or priority
- **Due date warnings** -- color-coded urgency (red for overdue, yellow for soon)
- **Undo** -- undo up to 20 actions (move, edit, delete, priority change)
- **SQLite persistence** -- data stored at `~/.local/share/rustkanban/kanban.db`

## Install

```
cargo install rustkanban
```

## Usage

```
rk          # launch the TUI
rk reset    # delete all tasks and tags
```

## Keybindings

### Board

| Key | Action |
|-----|--------|
| J / Left | Focus left column |
| L / Right | Focus right column |
| Up / Down / Tab / Shift+Tab | Move cursor (wraps around) |
| Space | New task |
| Enter | View task details |
| E | Edit task |
| d | Delete task |
| Shift+D | Clear done column |
| P | Cycle priority |
| K | Select / deselect task |
| S | Sort / filter menu |
| T | Tag management |
| / | Search |
| Ctrl+Z | Undo |
| ? | Help |
| Esc / Q | Quit |

### Selected Task

| Key | Action |
|-----|--------|
| J / L | Move task between columns |
| K / Esc | Deselect |

### New / Edit Task Modal

| Key | Action |
|-----|--------|
| Tab / Shift+Tab | Next / previous field |
| Space (on Priority) | Cycle Low / Medium / High |
| Space (on Tag) | Cycle through tags |
| Arrow keys | Navigate text cursor |
| Ctrl+S | Save |
| Esc | Cancel |

## License

MIT
