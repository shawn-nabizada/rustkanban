# RustKanban Use Cases

Comprehensive reference of every use case in the application, with step-by-step instructions.

---

## Table of Contents

- [1. Application Lifecycle](#1-application-lifecycle)
  - [1.1 Launch the TUI](#11-launch-the-tui)
  - [1.2 Quit the application](#12-quit-the-application)
  - [1.3 Quit while help overlay is open](#13-quit-while-help-overlay-is-open)
  - [1.4 Resize the terminal](#14-resize-the-terminal)
  - [1.5 Terminal too small](#15-terminal-too-small)
- [2. Navigation](#2-navigation)
  - [2.1 Focus a different column (keyboard)](#21-focus-a-different-column-keyboard)
  - [2.2 Focus a different column (mouse)](#22-focus-a-different-column-mouse)
  - [2.3 Move cursor up/down within a column](#23-move-cursor-updown-within-a-column)
  - [2.4 Cursor wrapping](#24-cursor-wrapping)
  - [2.5 Scroll a column with many tasks (keyboard)](#25-scroll-a-column-with-many-tasks-keyboard)
  - [2.6 Scroll a column with many tasks (mouse)](#26-scroll-a-column-with-many-tasks-mouse)
  - [2.7 Click on a specific task (mouse)](#27-click-on-a-specific-task-mouse)
- [3. Task Creation](#3-task-creation)
  - [3.1 Create a new task with title only](#31-create-a-new-task-with-title-only)
  - [3.2 Create a task with all fields](#32-create-a-task-with-all-fields)
  - [3.3 Cancel task creation](#33-cancel-task-creation)
  - [3.4 Attempt to save a task with empty title](#34-attempt-to-save-a-task-with-empty-title)
- [4. Task Editing](#4-task-editing)
  - [4.1 Edit a task from the board](#41-edit-a-task-from-the-board)
  - [4.2 Edit a task from the detail view](#42-edit-a-task-from-the-detail-view)
  - [4.3 Cancel editing](#43-cancel-editing)
- [5. Modal Text Editing](#5-modal-text-editing)
  - [5.1 Navigate between modal fields](#51-navigate-between-modal-fields)
  - [5.2 Type text in title or description](#52-type-text-in-title-or-description)
  - [5.3 Insert a newline in description](#53-insert-a-newline-in-description)
  - [5.4 Delete text (backspace)](#54-delete-text-backspace)
  - [5.5 Move cursor within a text field](#55-move-cursor-within-a-text-field)
  - [5.6 Move cursor up/down across wrapped lines](#56-move-cursor-updown-across-wrapped-lines)
  - [5.7 Cycle priority in the modal](#57-cycle-priority-in-the-modal)
  - [5.8 Set a due date](#58-set-a-due-date)
  - [5.9 Clear a due date](#59-clear-a-due-date)
  - [5.10 Toggle tags in the modal](#510-toggle-tags-in-the-modal)
  - [5.11 Navigate the tag list in the modal](#511-navigate-the-tag-list-in-the-modal)
- [6. Task Detail View](#6-task-detail-view)
  - [6.1 View task details](#61-view-task-details)
  - [6.2 Close the detail view](#62-close-the-detail-view)
  - [6.3 Edit from the detail view](#63-edit-from-the-detail-view)
- [7. Task Deletion](#7-task-deletion)
  - [7.1 Delete a single task](#71-delete-a-single-task)
  - [7.2 Cancel task deletion](#72-cancel-task-deletion)
  - [7.3 Clear all done tasks](#73-clear-all-done-tasks)
  - [7.4 Cancel clearing done tasks](#74-cancel-clearing-done-tasks)
  - [7.5 Attempt to clear done when column is empty](#75-attempt-to-clear-done-when-column-is-empty)
- [8. Task Duplication](#8-task-duplication)
  - [8.1 Duplicate a task](#81-duplicate-a-task)
  - [8.2 Duplicate when no task is under cursor](#82-duplicate-when-no-task-is-under-cursor)
- [9. Task Selection and Movement](#9-task-selection-and-movement)
  - [9.1 Select a task](#91-select-a-task)
  - [9.2 Deselect a task](#92-deselect-a-task)
  - [9.3 Move a selected task between columns](#93-move-a-selected-task-between-columns)
  - [9.4 Move a task at boundary column](#94-move-a-task-at-boundary-column)
  - [9.5 Navigate while a task is selected](#95-navigate-while-a-task-is-selected)
  - [9.6 Drag a task between columns (mouse)](#96-drag-a-task-between-columns-mouse)
  - [9.7 Mouse drag that ends in the same column](#97-mouse-drag-that-ends-in-the-same-column)
- [10. Priority](#10-priority)
  - [10.1 Cycle a task's priority](#101-cycle-a-tasks-priority)
- [11. Sorting](#11-sorting)
  - [11.1 Open the sort/filter menu](#111-open-the-sortfilter-menu)
  - [11.2 Change sort mode to priority](#112-change-sort-mode-to-priority)
  - [11.3 Change sort mode to due date](#113-change-sort-mode-to-due-date)
  - [11.4 Close the sort menu without changing](#114-close-the-sort-menu-without-changing)
  - [11.5 Sort mode persists across sessions](#115-sort-mode-persists-across-sessions)
- [12. Tag Filtering](#12-tag-filtering)
  - [12.1 Filter tasks by a tag](#121-filter-tasks-by-a-tag)
  - [12.2 Clear the tag filter](#122-clear-the-tag-filter)
  - [12.3 Tag filter indicator in status bar](#123-tag-filter-indicator-in-status-bar)
- [13. Tag Management](#13-tag-management)
  - [13.1 Open the tag management screen](#131-open-the-tag-management-screen)
  - [13.2 Create a new tag](#132-create-a-new-tag)
  - [13.3 Rename an existing tag](#133-rename-an-existing-tag)
  - [13.4 Delete a tag](#134-delete-a-tag)
  - [13.5 Cancel tag editing](#135-cancel-tag-editing)
  - [13.6 Navigate the tag list](#136-navigate-the-tag-list)
  - [13.7 Close tag management](#137-close-tag-management)
  - [13.8 Delete a tag that is the active filter](#138-delete-a-tag-that-is-the-active-filter)
- [14. Search](#14-search)
  - [14.1 Open search](#141-open-search)
  - [14.2 Type a search query](#142-type-a-search-query)
  - [14.3 Lock search and return to board](#143-lock-search-and-return-to-board)
  - [14.4 Close search and clear filter](#144-close-search-and-clear-filter)
  - [14.5 Search match highlighting](#145-search-match-highlighting)
  - [14.6 Interact with board while search filter is active](#146-interact-with-board-while-search-filter-is-active)

- [16. Help Overlay](#16-help-overlay)
  - [16.1 Open the help overlay](#161-open-the-help-overlay)
  - [16.2 Close the help overlay (keyboard)](#162-close-the-help-overlay-keyboard)
  - [16.3 Close the help overlay (mouse)](#163-close-the-help-overlay-mouse)
  - [16.4 Keyboard input while help is open](#164-keyboard-input-while-help-is-open)
- [17. Flash Messages](#17-flash-messages)
  - [17.1 Flash message displayed after actions](#171-flash-message-displayed-after-actions)
  - [17.2 Flash message auto-dismissal](#172-flash-message-auto-dismissal)
- [18. Due Date Warnings](#18-due-date-warnings)
  - [18.1 Overdue task display](#181-overdue-task-display)
  - [18.2 Due today display](#182-due-today-display)
  - [18.3 Due soon display (within 3 days)](#183-due-soon-display-within-3-days)
  - [18.4 Far due date display](#184-far-due-date-display)
  - [18.5 No due date display](#185-no-due-date-display)
- [19. Persistent Preferences](#19-persistent-preferences)
  - [19.1 Sort mode remembered across sessions](#191-sort-mode-remembered-across-sessions)
  - [19.2 Focused column remembered across sessions](#192-focused-column-remembered-across-sessions)
- [20. Theme Configuration](#20-theme-configuration)
  - [20.1 Initialize a theme file](#201-initialize-a-theme-file)
  - [20.2 Print the default theme to stdout](#202-print-the-default-theme-to-stdout)
  - [20.3 Customize theme colors](#203-customize-theme-colors)
  - [20.4 Use hex colors in theme](#204-use-hex-colors-in-theme)
  - [20.5 Invalid theme values fallback](#205-invalid-theme-values-fallback)
  - [20.6 Missing theme file fallback](#206-missing-theme-file-fallback)
- [21. Export and Import](#21-export-and-import)
  - [21.1 Export all data to JSON](#211-export-all-data-to-json)
  - [21.2 Import data from a JSON file](#212-import-data-from-a-json-file)
  - [21.3 Import deduplicates tags](#213-import-deduplicates-tags)
  - [21.4 Export with no tasks](#214-export-with-no-tasks)
- [22. Database Reset](#22-database-reset)
  - [22.1 Reset all data](#221-reset-all-data)
  - [22.2 Cancel a reset](#222-cancel-a-reset)
- [23. Shell Completions](#23-shell-completions)
  - [23.1 Generate bash completions](#231-generate-bash-completions)
  - [23.2 Generate zsh completions](#232-generate-zsh-completions)
  - [23.3 Generate fish completions](#233-generate-fish-completions)
  - [23.4 Generate PowerShell completions](#234-generate-powershell-completions)
- [24. Man Page](#24-man-page)
  - [24.1 Generate and view the man page](#241-generate-and-view-the-man-page)
  - [24.2 Install the man page](#242-install-the-man-page)
- [25. Status Bar](#25-status-bar)
  - [25.1 View current sort mode](#251-view-current-sort-mode)
  - [25.2 View active tag filter](#252-view-active-tag-filter)
  - [25.3 View selected task indicator](#253-view-selected-task-indicator)
  - [25.4 View help hint](#254-view-help-hint)
  - [25.5 View sync status](#255-view-sync-status)
- [26. Sync](#26-sync)
  - [26.1 First-time sync setup](#261-first-time-sync-setup)
  - [26.2 Syncing between machines](#262-syncing-between-machines)
  - [26.3 Working offline](#263-working-offline)
  - [26.4 Manual sync from TUI](#264-manual-sync-from-tui)
  - [26.5 Checking sync status](#265-checking-sync-status)
  - [26.6 Logging out](#266-logging-out)

---

## 1. Application Lifecycle

### 1.1 Launch the TUI

**Steps:**
1. Run `rk` in the terminal
2. The kanban board renders with three columns: Todo, In Progress, Done
3. The previously focused column is restored from preferences
4. The previously selected sort mode is restored from preferences
5. All tasks are loaded from `~/.local/share/rustkanban/kanban.db`
6. Mouse capture is enabled

**Notes:**
- The database and its parent directories are created automatically on first launch
- The theme is loaded from `~/.config/rustkanban/theme.toml` if it exists
- If logged in, an automatic sync pull is performed on startup

### 1.2 Quit the application

**Steps:**
1. From the board view, press `Q`, `q`, or `Esc`
2. If logged in, an automatic sync push is performed
3. The currently focused column is saved to preferences
4. Mouse capture is disabled
5. The terminal is restored to its original state

**Notes:**
- Quit is only available from Board mode. It is not available from modals, search, or other overlays.

### 1.3 Quit while help overlay is open

**Steps:**
1. Press `?` to open help
2. Press `Q` or `q` to close help (does not quit)
3. Press `Q` or `q` again to quit the application

**Notes:**
- While help is open, all keys except `Esc`, `?`, `Q/q` are ignored

### 1.4 Resize the terminal

**Steps:**
1. Resize the terminal window while the TUI is running
2. The layout recalculates automatically on the next render cycle (100ms)
3. Column widths adjust to 1/3 each, scroll offsets are recalculated, modal wrap width updates

### 1.5 Terminal too small

**Steps:**
1. Resize the terminal below 80 columns or 24 rows
2. The board is replaced with "Terminal too small (need 80x30)"
3. Resize back to at least 80x30 to restore normal rendering

---

## 2. Navigation

### 2.1 Focus a different column (keyboard)

**Steps:**
1. Press `J` or `Left` to move focus one column to the left
2. Press `L` or `Right` to move focus one column to the right
3. The focused column's border changes to the `focused_border` theme color
4. Unfocused columns show the `unfocused_border` theme color

**Notes:**
- Focus stops at boundaries (cannot go left from Todo or right from Done)

### 2.2 Focus a different column (mouse)

**Steps:**
1. Click anywhere in the target column
2. Focus moves to that column immediately
3. If you clicked on a task, the cursor also moves to that task

### 2.3 Move cursor up/down within a column

**Steps:**
1. Press `Up` or `Shift+Tab` to move the cursor up one task
2. Press `Down` or `Tab` to move the cursor down one task
3. The cursor indicator (`>`) appears next to the current task

### 2.4 Cursor wrapping

**Steps:**
1. When the cursor is on the last task in a column, press `Down` or `Tab`
2. The cursor wraps to the first task
3. When the cursor is on the first task, press `Up` or `Shift+Tab`
4. The cursor wraps to the last task

### 2.5 Scroll a column with many tasks (keyboard)

**Steps:**
1. Navigate up/down in a column with more tasks than fit on screen
2. The column automatically scrolls to keep the cursor visible
3. The scroll offset adjusts to show the full height of the current task

### 2.6 Scroll a column with many tasks (mouse)

**Steps:**
1. Position the mouse over a column
2. Scroll the mouse wheel down to scroll the column down (3 lines per tick)
3. Scroll the mouse wheel up to scroll the column up (3 lines per tick)

**Notes:**
- Scroll offset is clamped to the total content height (cannot scroll past the end)

### 2.7 Click on a specific task (mouse)

**Steps:**
1. Click on a task in any column
2. The column is focused and the cursor moves to the clicked task
3. This accounts for variable task heights (multi-line titles, tags, scroll offset)

---

## 3. Task Creation

### 3.1 Create a new task with title only

**Steps:**
1. Press `Space` from the board view
2. The New Task modal opens with cursor in the Title field
3. Type the task title
4. Press `Ctrl+S` to save
5. The task appears in the Todo column with Medium priority and no due date

**Notes:**
- New tasks always go to the Todo column regardless of which column is focused

### 3.2 Create a task with all fields

**Steps:**
1. Press `Space` from the board view
2. Type a title in the Title field
3. Press `Tab` to move to the Description field
4. Type a description (press `Enter` for newlines)
5. Press `Tab` to move to Priority
6. Press `Space` to cycle priority (Low → Medium → High → Low)
7. Press `Tab` to move to Tags
8. Use `Up`/`Down` to navigate the tag list
9. Press `Space` to toggle tags on/off (selected tags show `[x]`)
10. Press `Tab` to move to Due Date Year
11. Type a 4-digit year
12. Press `Tab` to move to Due Date Month, type 1-2 digit month
13. Press `Tab` to move to Due Date Day, type 1-2 digit day
14. Press `Ctrl+S` to save

### 3.3 Cancel task creation

**Steps:**
1. Press `Space` to open the New Task modal
2. Optionally type some text
3. Press `Esc` to cancel
4. No task is created; all modal input is discarded

### 3.4 Attempt to save a task with empty title

**Steps:**
1. Open the New Task modal
2. Leave the title field empty (or only whitespace)
3. Press `Ctrl+S`
4. An error message "Title is required" appears in the modal
5. The modal remains open for correction

---

## 4. Task Editing

### 4.1 Edit a task from the board

**Steps:**
1. Navigate the cursor to the task you want to edit
2. Press `E`
3. The Edit Task modal opens, pre-populated with the task's current title, description, priority, tags, and due date
4. Modify any fields
5. Press `Ctrl+S` to save changes


### 4.2 Edit a task from the detail view

**Steps:**
1. Press `Enter` on a task to open the detail view
2. Press `E` from the detail view
3. The Edit Task modal opens with the task's current values
4. Modify fields and press `Ctrl+S` to save

### 4.3 Cancel editing

**Steps:**
1. Press `E` on a task to open the Edit modal
2. Make changes
3. Press `Esc` to cancel
4. No changes are saved; the task retains its original values

---

## 5. Modal Text Editing

### 5.1 Navigate between modal fields

**Steps:**
1. In the New Task or Edit Task modal, press `Tab` to move to the next field
2. Press `Shift+Tab` to move to the previous field
3. Field order: Title → Description → Priority → Tags → Year → Month → Day
4. Navigation wraps around (Tab from Day goes to Title)

### 5.2 Type text in title or description

**Steps:**
1. Focus the Title or Description field
2. Type characters; they insert at the cursor position
3. The cursor advances after each character

### 5.3 Insert a newline in description

**Steps:**
1. Focus the Description field
2. Press `Enter` to insert a newline character
3. The text wraps and continues on the next line

**Notes:**
- `Enter` only inserts newlines in the Description field. In other fields, it has no effect.
- Use `Ctrl+S` (not Enter) to save the modal.

### 5.4 Delete text (backspace)

**Steps:**
1. Focus the Title or Description field
2. Press `Backspace` to delete the character before the cursor
3. In due date fields (Year/Month/Day), `Backspace` removes the last digit
4. `Backspace` has no effect on the Priority or Tag fields

### 5.5 Move cursor within a text field

**Steps:**
1. Focus the Title or Description field
2. Press `Left` to move the cursor one character left
3. Press `Right` to move the cursor one character right
4. The cursor stops at the beginning and end of the text

### 5.6 Move cursor up/down across wrapped lines

**Steps:**
1. Focus the Title or Description field with text that wraps
2. Press `Up` to move the cursor up one visual line
3. Press `Down` to move the cursor down one visual line
4. The cursor maintains its column position when moving between lines

**Notes:**
- When focused on the Tag field, `Up`/`Down` navigate the tag list instead

### 5.7 Cycle priority in the modal

**Steps:**
1. Tab to the Priority field
2. Press `Space` to cycle: Low → Medium → High → Low
3. The current priority is displayed in the field

### 5.8 Set a due date

**Steps:**
1. Tab to the Due Date Year field
2. Type a 4-digit year (e.g., `2026`)
3. Tab to Month, type 1-2 digits (e.g., `3` or `03`)
4. Tab to Day, type 1-2 digits (e.g., `15`)
5. The due date is validated on save; invalid dates are ignored (task saves without a due date)

### 5.9 Clear a due date

**Steps:**
1. Open the Edit Task modal for a task with a due date
2. Tab to any due date field
3. Press `Backspace` repeatedly to clear the digits
4. Clear all three fields (Year, Month, Day) to remove the due date
5. Press `Ctrl+S` to save

### 5.10 Toggle tags in the modal

**Steps:**
1. Tab to the Tags field in the modal
2. Navigate to a tag with `Up`/`Down`
3. Press `Space` to toggle the tag on or off
4. Selected tags show `[x]`, unselected show `[ ]`
5. Multiple tags can be selected simultaneously

### 5.11 Navigate the tag list in the modal

**Steps:**
1. Tab to the Tags field
2. Press `Down` to move down the tag list
3. Press `Up` to move up the tag list
4. The cursor wraps at boundaries (stops at first/last tag)

---

## 6. Task Detail View

### 6.1 View task details

**Steps:**
1. Navigate the cursor to a task
2. Press `Enter`
3. A read-only detail overlay shows the task's title, description, priority, due date, tags, created date, and updated date

### 6.2 Close the detail view

**Steps:**
1. From the detail view, press `Esc`, `Enter`, `Q`, or `q`
2. Returns to the board view

### 6.3 Edit from the detail view

**Steps:**
1. From the detail view, press `E`
2. The Edit Task modal opens with the task's values pre-populated
3. Make changes and save with `Ctrl+S`

---

## 7. Task Deletion

### 7.1 Delete a single task

**Steps:**
1. Navigate the cursor to the task you want to delete
2. Press `d`
3. A confirmation dialog appears: "Delete '<title>'? (Y/N)"
4. Press `Y` to confirm
5. The task is deleted from the database

7. A flash message confirms the deletion

### 7.2 Cancel task deletion

**Steps:**
1. Press `d` on a task
2. The confirmation dialog appears
3. Press `N` or `Esc`
4. The task is not deleted; returns to board view

### 7.3 Clear all done tasks

**Steps:**
1. Press `Shift+D` from the board view
2. A confirmation dialog appears: "Clear all done tasks? (Y/N)"
3. Press `Y` to confirm
4. All tasks in the Done column are deleted
5. A flash message shows the count of cleared tasks

**Notes:**


### 7.4 Cancel clearing done tasks

**Steps:**
1. Press `Shift+D`
2. Press `N` or `Esc`
3. No tasks are deleted

### 7.5 Attempt to clear done when column is empty

**Steps:**
1. With no tasks in the Done column, press `Shift+D`
2. Nothing happens (the confirmation dialog does not open)

---

## 8. Task Duplication

### 8.1 Duplicate a task

**Steps:**
1. Navigate the cursor to the task you want to duplicate
2. Press `C`
3. A new task is created in the same column with identical title, description, priority, due date, and tags
4. The cursor moves to the newly created duplicate
5. A flash message says "Duplicated '<title>'"


### 8.2 Duplicate when no task is under cursor

**Steps:**
1. Focus an empty column
2. Press `C`
3. Nothing happens (no task to duplicate)

---

## 9. Task Selection and Movement

### 9.1 Select a task

**Steps:**
1. Navigate the cursor to a task
2. Press `K`
3. The task is highlighted with the `selected` theme color
4. The status bar shows "SELECTED" with movement hints
5. The mode changes to Selected

### 9.2 Deselect a task

**Steps:**
1. While a task is selected, press `K` or `Esc`
2. The task returns to normal display
3. The mode returns to Board

### 9.3 Move a selected task between columns

**Steps:**
1. Select a task with `K`
2. Press `L` or `Right` to move the task one column to the right
3. Press `J` or `Left` to move the task one column to the left
4. The task moves in the database, the cursor follows the task to its new column


### 9.4 Move a task at boundary column

**Steps:**
1. Select a task in the Todo column
2. Press `J` or `Left`
3. Nothing happens (cannot move left from Todo)
4. Similarly, cannot move right from Done

### 9.5 Navigate while a task is selected

**Steps:**
1. Select a task with `K`
2. Press `Up`/`Down`/`Tab`/`Shift+Tab` to move the cursor
3. Cursor movement works the same as in Board mode
4. The selected task remains selected

### 9.6 Drag a task between columns (mouse)

**Steps:**
1. Click and hold on a task (left mouse button down)
2. Drag the mouse to a different column
3. Release the mouse button (left mouse button up)
4. The task moves to the target column

6. If the task was in Selected mode, it is deselected after the drag

### 9.7 Mouse drag that ends in the same column

**Steps:**
1. Click and hold on a task
2. Release the mouse button in the same column
3. Nothing happens (the task stays in place)

---

## 10. Priority

### 10.1 Cycle a task's priority

**Steps:**
1. Navigate the cursor to a task
2. Press `P`
3. The priority cycles: Low → Medium → High → Low
4. The priority indicator changes (L/M/H) with corresponding color


---

## 11. Sorting

### 11.1 Open the sort/filter menu

**Steps:**
1. Press `S` from the board view
2. The sort menu overlay appears with options:
   - Due Date (sort)
   - Priority (sort)
   - Filter by Tag: [list of tags] (if tags exist)
   - Clear filter (if tags exist)

### 11.2 Change sort mode to priority

**Steps:**
1. Press `S` to open the sort menu
2. Navigate to "Priority" with `Down`
3. Press `Enter`
4. All columns re-sort by priority (High first, then Medium, then Low)
5. The sort mode is saved to preferences for next session

### 11.3 Change sort mode to due date

**Steps:**
1. Press `S` to open the sort menu
2. Navigate to "Due Date" (it's the first option)
3. Press `Enter`
4. All columns re-sort by due date (soonest first, tasks without due dates last)

### 11.4 Close the sort menu without changing

**Steps:**
1. Press `S` to open the sort menu
2. Press `Esc`, `Q`, or `q`
3. The menu closes with no changes

### 11.5 Sort mode persists across sessions

**Steps:**
1. Change the sort mode to Priority
2. Quit the application
3. Relaunch with `rk`
4. The sort mode is still Priority (loaded from the preferences table)

---

## 12. Tag Filtering

### 12.1 Filter tasks by a tag

**Steps:**
1. Press `S` to open the sort menu
2. Navigate down past the sort options to the tag list
3. Select a tag and press `Enter`
4. Only tasks with that tag are displayed in all columns
5. The status bar shows "Tag: <name>"
6. The sort menu closes

### 12.2 Clear the tag filter

**Steps:**
1. Press `S` to open the sort menu
2. Navigate to "Clear filter" (last option)
3. Press `Enter`
4. All tasks are displayed again

### 12.3 Tag filter indicator in status bar

**Steps:**
1. Apply a tag filter
2. The status bar shows "Tag: <tag name>" in yellow

---

## 13. Tag Management

### 13.1 Open the tag management screen

**Steps:**
1. Press `T` from the board view
2. The tag management overlay appears listing all tags

### 13.2 Create a new tag

**Steps:**
1. Open the tag management screen
2. Press `Space` or `A`
3. A text input field appears
4. Type the tag name
5. Press `Enter` to confirm
6. The tag is created and appears in the list

**Notes:**
- Empty names are silently discarded (the edit mode closes with no creation)

### 13.3 Rename an existing tag

**Steps:**
1. Open the tag management screen
2. Navigate to the tag with `Up`/`Down`
3. Press `E` or `Enter`
4. The tag name appears in an editable field
5. Modify the name
6. Press `Enter` to confirm
7. The tag is renamed everywhere (including on tasks that reference it)

### 13.4 Delete a tag

**Steps:**
1. Open the tag management screen
2. Navigate to the tag
3. Press `D`
4. The tag is deleted
5. The tag is silently removed from all tasks that had it (cascade delete)
6. Tasks are reloaded to reflect the change

### 13.5 Cancel tag editing

**Steps:**
1. Start creating or renaming a tag
2. Press `Esc`
3. The edit is cancelled; no changes are made

### 13.6 Navigate the tag list

**Steps:**
1. In the tag management screen, press `Up` to move up
2. Press `Down` to move down
3. The cursor stops at the first and last tag

### 13.7 Close tag management

**Steps:**
1. Press `Esc`, `Q`, or `q`
2. Returns to the board view

### 13.8 Delete a tag that is the active filter

**Steps:**
1. Apply a tag filter via the sort menu
2. Open tag management with `T`
3. Delete the tag that is currently used as a filter
4. The filter is automatically cleared
5. All tasks become visible again

---

## 14. Search

### 14.1 Open search

**Steps:**
1. Press `/` from the board view
2. The search bar appears at the bottom of the screen
3. The cursor is in the search input field

### 14.2 Type a search query

**Steps:**
1. Open search with `/`
2. Type characters to filter
3. Tasks are filtered live as you type — only tasks whose title or description contains the query (case-insensitive) are shown
4. Matching text in titles is underlined
5. Press `Backspace` to delete the last character

### 14.3 Lock search and return to board

**Steps:**
1. Type a search query
2. Press `Enter`
3. The search bar remains visible showing the active filter
4. You return to Board mode with full keyboard navigation while the filter stays active
5. You can navigate, select, move, edit, delete, etc. while filtered

### 14.4 Close search and clear filter

**Steps:**
1. From the search input (or from the board with an active filter), press `Esc` while in SearchFilter mode
2. If you're in Board mode with a locked search, press `/` to re-enter search mode, then `Esc` to clear
3. The filter is removed, all tasks are shown again, the search bar disappears

### 14.5 Search match highlighting

**Steps:**
1. Type a search query
2. Task titles that match show the matching substring underlined
3. Highlighting is case-insensitive and works across word-wrapped lines
4. Highlighting handles Unicode text safely

### 14.6 Interact with board while search filter is active

**Steps:**
1. Open search with `/`, type a query, press `Enter` to lock
2. Navigate columns with `J`/`L`, move cursor with `Up`/`Down`
3. Select tasks with `K`, move them with `J`/`L`
4. Create, edit, delete, duplicate tasks — all operations work normally
5. Only filtered (matching) tasks are visible and interactive

---

## 16. Help Overlay

### 16.1 Open the help overlay

**Steps:**
1. Press `?` from the board view
2. A centered overlay appears listing all keybindings organized by section:
   - Navigation (J/L, Up/Down)
   - Tasks (Space, Enter, E, d, D, P)
   - Selection (K, J/L)
   - Other (S, T, /, ?, Esc/Q)

### 16.2 Close the help overlay (keyboard)

**Steps:**
1. While help is open, press `Esc`, `?`, `Q`, or `q`
2. The overlay closes, returns to board view

### 16.3 Close the help overlay (mouse)

**Steps:**
1. While help is open, click anywhere with the mouse
2. The overlay closes

### 16.4 Keyboard input while help is open

**Steps:**
1. Open help with `?`
2. Press any key other than `Esc`, `?`, `Q`/`q`
3. The key is ignored — no board actions occur while help is visible

---

## 17. Flash Messages

### 17.1 Flash message displayed after actions

Flash messages appear in the status bar after these actions:
- Task deleted: "Deleted '<title>'"
- Task duplicated: "Duplicated '<title>'"
- Done column cleared: "Cleared N done task(s)"


### 17.2 Flash message auto-dismissal

**Steps:**
1. Perform an action that shows a flash message
2. The message appears in the status bar in green
3. After 3 seconds, the message automatically disappears
4. The tick method checks elapsed time each render cycle

---

## 18. Due Date Warnings

### 18.1 Overdue task display

**Steps:**
1. Create a task with a due date in the past
2. The due date renders in the `due_overdue` theme color (default: red)
3. The text shows "Due: YYYY-MM-DD (overdue)"

### 18.2 Due today display

**Steps:**
1. Create a task with today's date
2. The due date renders in the `due_today` theme color (default: red)
3. The text shows "Due: YYYY-MM-DD (today)"

### 18.3 Due soon display (within 3 days)

**Steps:**
1. Create a task with a due date 1-3 days from now
2. The due date renders in the `due_soon` theme color (default: yellow)
3. The text shows "Due: YYYY-MM-DD (N days)"

### 18.4 Far due date display

**Steps:**
1. Create a task with a due date more than 3 days from now
2. The due date renders in the `due_far` theme color (default: gray)
3. The text shows "Due: YYYY-MM-DD"

### 18.5 No due date display

**Steps:**
1. Create a task without setting a due date
2. The due date line shows a dimmed dash or is omitted
3. When sorting by due date, tasks without due dates appear last

---

## 19. Persistent Preferences

### 19.1 Sort mode remembered across sessions

**Steps:**
1. Open the sort menu and change the sort mode
2. Quit the application
3. Relaunch with `rk`
4. The sort mode is the same as when you quit

**Notes:**
- Stored in the `preferences` table with key `sort_mode`
- Default is `DueDate` if no preference is saved

### 19.2 Focused column remembered across sessions

**Steps:**
1. Navigate to a column (e.g., In Progress)
2. Quit the application
3. Relaunch with `rk`
4. The In Progress column is focused

**Notes:**
- Stored in the `preferences` table with key `focused_column`
- Saved on quit, loaded on startup
- Default is Todo if no preference is saved

---

## 20. Theme Configuration

### 20.1 Initialize a theme file

**Steps:**
1. Run `rk theme --init`
2. A `theme.toml` file is created at `~/.config/rustkanban/theme.toml`
3. The file contains all configurable color settings with default values
4. The output confirms: "Theme written to <path>"

### 20.2 Print the default theme to stdout

**Steps:**
1. Run `rk theme`
2. The default theme TOML content is printed to stdout
3. No file is created

### 20.3 Customize theme colors

**Steps:**
1. Run `rk theme --init` to create the file
2. Open `~/.config/rustkanban/theme.toml` in a text editor
3. Change color values using named colors:
   - `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `Gray`
   - `DarkGray`, `LightRed`, `LightGreen`, `LightYellow`, `LightBlue`, `LightMagenta`, `LightCyan`, `White`
4. Save the file
5. Relaunch `rk` to see the new colors

**Configurable sections:**
- `[board]`: focused_border, unfocused_border, cursor, selected, title
- `[priority]`: high, medium, low
- `[tags]`: color
- `[due_date]`: overdue, today, soon, far
- `[modal]`: border, focused, error

### 20.4 Use hex colors in theme

**Steps:**
1. In `theme.toml`, set a color to a hex value: `cursor = "#FF5500"`
2. The format must be `#RRGGBB` (6 hex digits with `#` prefix)
3. Relaunch `rk`

### 20.5 Invalid theme values fallback

**Steps:**
1. Set a theme value to an invalid string (e.g., `cursor = "notacolor"`)
2. Relaunch `rk`
3. That specific color falls back to its default value
4. Other valid colors are still applied

### 20.6 Missing theme file fallback

**Steps:**
1. Delete or never create `~/.config/rustkanban/theme.toml`
2. Launch `rk`
3. All default colors are used

---

## 21. Export and Import

### 21.1 Export all data to JSON

**Steps:**
1. Run `rk export`
2. All tasks and tags are serialized to JSON and printed to stdout
3. Redirect to a file: `rk export > backup.json`

**JSON structure (v2):**
```json
{
  "version": 2,
  "tasks": [
    {
      "uuid": "550e8400-e29b-41d4-a716-446655440000",
      "title": "...",
      "description": "...",
      "priority": "High",
      "column": "todo",
      "due_date": "2026-03-15",
      "tags": ["bug", "urgent"]
    }
  ],
  "tags": ["bug", "urgent", "feature"]
}
```

**Notes:**
- Export now produces v2 format with UUIDs
- v1 imports (without UUIDs) are still supported and backward compatible

### 21.2 Import data from a JSON file

**Steps:**
1. Run `rk import backup.json`
2. All tasks from the file are added to the database
3. Tags referenced in the file are created if they don't already exist
4. Output: "Imported N task(s)."

**Notes:**
- Import is additive — existing tasks are not modified or removed
- Invalid priorities default to Medium; invalid columns default to Todo
- Invalid due dates are silently ignored

### 21.3 Import deduplicates tags

**Steps:**
1. Create a tag "bug" in the app
2. Export data that also contains the tag "bug"
3. Import the file
4. Only one "bug" tag exists (no duplicates)

### 21.4 Export with no tasks

**Steps:**
1. Run `rk export` with an empty database
2. Output is valid JSON with `"tasks": []` and `"tags": []`

---

## 22. Database Reset

### 22.1 Reset all data

**Steps:**
1. Run `rk reset`
2. Prompt: "Delete all tasks and tags? (Y/N)"
3. Type `Y` and press Enter
4. All tasks, tags, task-tag associations, and preferences are deleted
5. Output: "All data cleared."

### 22.2 Cancel a reset

**Steps:**
1. Run `rk reset`
2. Prompt: "Delete all tasks and tags? (Y/N)"
3. Type `N` (or anything other than Y) and press Enter
4. Output: "Aborted."
5. No data is deleted

---

## 23. Shell Completions

### 23.1 Generate bash completions

**Steps:**
1. Run `rk completions bash >> ~/.bashrc`
2. Restart your shell or run `source ~/.bashrc`
3. Tab completion works for `rk` subcommands

### 23.2 Generate zsh completions

**Steps:**
1. Run `rk completions zsh >> ~/.zshrc`
2. Restart your shell or run `source ~/.zshrc`

### 23.3 Generate fish completions

**Steps:**
1. Run `rk completions fish > ~/.config/fish/completions/rk.fish`

### 23.4 Generate PowerShell completions

**Steps:**
1. Run `rk completions powershell >> $PROFILE`
2. Restart PowerShell

---

## 24. Man Page

### 24.1 Generate and view the man page

**Steps:**
1. Run `rk manpage | man -l -`
2. The man page renders in your terminal's man viewer
3. Includes synopsis, description, and all subcommand documentation

### 24.2 Install the man page

**Steps:**
1. Run `rk manpage > rk.1`
2. Copy to man directory: `sudo cp rk.1 /usr/local/share/man/man1/`
3. Update man database: `sudo mandb`
4. View with `man rk`

---

## 25. Status Bar

### 25.1 View current sort mode

The status bar always shows "Sort: Due Date" or "Sort: Priority" in cyan.

### 25.2 View active tag filter

When a tag filter is active, the status bar shows "Tag: <name>" in yellow after the sort indicator.

### 25.3 View selected task indicator

When a task is selected (K mode), the status bar shows "SELECTED" with a yellow background and movement hints: "J/L: move task  K/Esc: deselect".

### 25.4 View help hint

The right side of the status bar always shows "?: help" in cyan/gray.

### 25.5 View sync status

When logged in, the status bar shows the sync state:
- Green: synced (last sync completed successfully)
- Yellow: syncing (sync in progress)
- Red: offline (server unreachable or not logged in)

---

## 26. Sync

### 26.1 First-time sync setup

**Steps:**
1. Run `rk login`
2. A browser window opens to the GitHub OAuth page
3. Authenticate with your GitHub account
4. The CLI confirms successful login
5. Credentials are stored at `~/.config/rustkanban/credentials.json`

**Notes:**
- Sync is purely opt-in. The app works fully offline without an account.
- Existing local data is preserved and will sync on the next push.

### 26.2 Syncing between machines

**Steps:**
1. Run `rk login` on both machines
2. Tasks and tags sync automatically: pull on TUI startup, push on quit
3. For immediate sync, press `Ctrl+R` from within the TUI
4. Conflicts are resolved with last-write-wins

**Notes:**
- The default sync server is `https://sync.rustkanban.com`
- UUIDs are used to identify tasks and tags across machines

### 26.3 Working offline

**Steps:**
1. Use the app normally while offline (no network needed)
2. All changes are saved locally to SQLite
3. When network is available again, changes sync on the next TUI startup, quit, or `Ctrl+R`

**Notes:**
- The app never blocks on network failures. Sync errors are shown briefly and do not interrupt workflow.

### 26.4 Manual sync from TUI

**Steps:**
1. From the board view, press `Ctrl+R`
2. The status bar shows syncing state (yellow)
3. After sync completes, the board reloads with any new data from the server
4. The status bar returns to synced state (green)

### 26.5 Checking sync status

**Steps:**
1. Run `rk status`
2. Output shows: device name, sync server URL, last sync time, and login state

### 26.6 Logging out

**Steps:**
1. Run `rk logout`
2. Credentials are removed from `~/.config/rustkanban/credentials.json`
3. Local data is preserved (nothing is deleted)
4. Sync stops until you log in again
