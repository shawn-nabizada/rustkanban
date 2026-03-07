# Changelog

All notable changes to this project will be documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added
- Duplicate task (C key) with proper undo support
- Mouse support: click to focus/select, scroll wheel, drag tasks between columns
- Search match highlighting (underline) in task titles
- Persistent preferences: sort mode and focused column remembered across sessions
- Man page generation (`rk manpage`)
- Homebrew formula and AUR PKGBUILD
- Install script SHA256 checksum verification
- CI format check (`cargo fmt -- --check`)
- Pre-commit hook (fmt + clippy)
- CLAUDE.md development guide
- Panic-safe terminal restore (mouse capture cleanup)

## [0.1.0] - 2026-03-06

### Added
- 3-column kanban board (Todo, In Progress, Done)
- Vim-inspired navigation (J/L columns, Up/Down/Tab tasks)
- Task management (create, edit, delete, move, cycle priority)
- Multiple tags per task with toggle selection in modal
- Tag management screen (create, rename, delete)
- Tag filtering via sort menu
- Live search filtering by title or description
- Sort by due date (default) or priority
- Due date warnings with color-coded urgency
- Undo up to 20 actions (move, edit, delete, priority change)
- SQLite persistence at ~/.local/share/rustkanban/kanban.db
- JSON export (`rk export`) and import (`rk import <file>`)
- Theme configuration via ~/.config/rustkanban/theme.toml
- Shell completions (bash, zsh, fish, powershell)
- Cross-platform binaries (Linux x86/ARM, macOS Intel/Silicon, Windows)
- Automated releases via GitHub Actions
