# RustKanban

[![CI](https://github.com/shawn-nabizada/rustkanban/actions/workflows/ci.yml/badge.svg)](https://github.com/shawn-nabizada/rustkanban/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/rustkanban)](https://crates.io/crates/rustkanban)
[![License: BSL-1.1](https://img.shields.io/badge/license-BSL--1.1-blue.svg)](LICENSE)

A Rust terminal (TUI) kanban board with vim-inspired navigation, tags, search, SQLite persistence, and optional cross-machine sync.

![Demo](demo.gif)

## Features

- **3-column board** -- Todo, In Progress, Done
- **Vim-inspired navigation** -- J/L for columns, Up/Down/Tab to move between tasks
- **Task management** -- create, edit, delete, duplicate, move between columns, cycle priority
- **Multiple tags** -- create/rename/delete tags, toggle multiple per task, filter by tag
- **Search** -- live filter tasks by title or description, with match highlighting
- **Mouse support** -- click to focus/select, scroll wheel, drag tasks between columns
- **Sorting** -- sort by due date (default) or priority
- **Due date warnings** -- color-coded urgency (red for overdue, yellow for soon)

- **Export / Import** -- JSON export and import for backup or migration
- **Theme configuration** -- customizable colors via TOML config file
- **Cross-machine sync** -- opt-in sync via GitHub OAuth (works fully offline without an account)
- **Persistent preferences** -- sort mode and focused column remembered across sessions
- **SQLite persistence** -- data stored at `~/.local/share/rustkanban/kanban.db`

## Install

### Quick install (Linux / macOS)

```sh
curl -sSL https://raw.githubusercontent.com/shawn-nabizada/rustkanban/main/install.sh | sh
```

### Pre-built binaries (no Rust required)

Download the latest binary for your platform from [Releases](https://github.com/shawn-nabizada/rustkanban/releases/latest):

| Platform | File |
|----------|------|
| Linux (x86_64) | `rk-linux-x86_64` |
| Linux (ARM64) | `rk-linux-aarch64` |
| macOS (Intel) | `rk-macos-x86_64` |
| macOS (Apple Silicon) | `rk-macos-aarch64` |
| Windows | `rk-windows-x86_64.exe` |

**Linux:**

```sh
curl -L -o rk https://github.com/shawn-nabizada/rustkanban/releases/latest/download/rk-linux-x86_64
chmod +x rk
sudo mv rk /usr/local/bin/
```

**macOS (Apple Silicon):**

```sh
curl -L -o rk https://github.com/shawn-nabizada/rustkanban/releases/latest/download/rk-macos-aarch64
chmod +x rk
sudo mv rk /usr/local/bin/
```

**macOS (Intel):**

```sh
curl -L -o rk https://github.com/shawn-nabizada/rustkanban/releases/latest/download/rk-macos-x86_64
chmod +x rk
sudo mv rk /usr/local/bin/
```

**Windows:**

1. Download `rk-windows-x86_64.exe` from the [releases page](https://github.com/shawn-nabizada/rustkanban/releases/latest)
2. Rename it to `rk.exe`
3. Create a folder for it, e.g. `C:\Tools`
4. Move `rk.exe` into `C:\Tools`
5. Add `C:\Tools` to your PATH:
   - Press `Win + R`, type `sysdm.cpl`, press Enter
   - Go to the **Advanced** tab, click **Environment Variables**
   - Under **User variables**, select **Path** and click **Edit**
   - Click **New** and add `C:\Tools`
   - Click **OK** on all dialogs
6. Open a new terminal and run `rk`

### From source (requires Rust)

```
cargo install rustkanban
```

## Usage

```
rk                  # launch the TUI
rk reset            # delete all tasks and tags
rk export           # export tasks and tags to JSON (stdout)
rk import <file>    # import tasks and tags from a JSON file
rk theme            # print default theme config
rk theme --init     # create theme file at ~/.config/rustkanban/theme.toml
rk completions <sh> # generate shell completions (bash, zsh, fish, powershell)
rk manpage          # output man page to stdout
rk login            # authenticate with sync service (GitHub OAuth)
rk logout           # log out from sync service
rk sync             # sync with server (pull + push)
rk status           # show sync status
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
| C | Duplicate task |
| d | Delete task |
| Shift+D | Clear done column |
| P | Cycle priority |
| K | Select / deselect task |
| S | Sort / filter menu |
| T | Tag management |
| / | Search |
| Ctrl+R | Sync with server |

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
| Space (on Tags) | Toggle tag on/off |
| Up / Down (on Tags) | Navigate tag list |
| Arrow keys | Navigate text cursor |
| Ctrl+S | Save |
| Esc | Cancel |

### Mouse

| Action | Effect |
|--------|--------|
| Click column | Focus that column |
| Click task | Move cursor to task |
| Scroll wheel | Scroll column up/down |
| Drag task to column | Move task between columns |

## Theme

Customize colors by creating a theme file:

```sh
rk theme --init   # creates ~/.config/rustkanban/theme.toml
```

Supports named colors (`Red`, `Cyan`, `LightGreen`, etc.) and hex (`#FF5500`). See the generated file for all options.

## Export / Import

```sh
rk export > backup.json        # export all tasks and tags
rk import backup.json          # import (additive, deduplicates tags)
```

## Shell Completions

Generate tab completions for your shell:

```sh
# Bash
rk completions bash >> ~/.bashrc

# Zsh
rk completions zsh >> ~/.zshrc

# Fish
rk completions fish > ~/.config/fish/completions/rk.fish

# PowerShell
rk completions powershell >> $PROFILE
```

## Sync

RustKanban supports optional cross-machine sync. Sync is purely opt-in -- the app works fully offline without an account.

```sh
rk login             # opens browser for GitHub OAuth
rk sync              # manual sync (pull + push)
rk status            # show device, server, and last sync time
rk logout            # log out (local data is preserved)
```

Once logged in, the TUI automatically pulls on startup and pushes on quit. Press `Ctrl+R` for a manual sync during a session. The status bar shows sync state: green for synced, yellow for syncing, red for offline.

Conflicts are resolved with last-write-wins. Credentials are stored at `~/.config/rustkanban/credentials.json`.

## License

Business Source License 1.1 — see [LICENSE](LICENSE) for details.

Non-commercial use is permitted. After 4 years from each release, the code converts to Apache 2.0.
