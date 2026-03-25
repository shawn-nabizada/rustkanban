//! Options modal (keybindings + theme).

use super::*;

impl App {
    pub fn open_options(&mut self) {
        self.options_tab = 0;
        self.options_scroll = 0;
        self.options_cursor = 0;
        self.options_rebinding = false;
        self.options_rebind_action = None;
        self.options_rebind_context = None;
        self.mode = AppMode::Options;
    }

    pub fn close_options(&mut self) {
        self.options_rebinding = false;
        self.mode = AppMode::Board;
    }

    pub fn options_next_tab(&mut self) {
        self.options_tab = (self.options_tab + 1) % 2;
        self.options_cursor = 0;
        self.options_scroll = 0;
    }

    pub fn options_prev_tab(&mut self) {
        self.options_tab = if self.options_tab == 0 { 1 } else { 0 };
        self.options_cursor = 0;
        self.options_scroll = 0;
    }

    // --- Keybindings tab helpers ---

    /// Fixed list of board actions shown in the keybindings tab, in display order.
    pub const BOARD_ACTIONS: [crate::keybindings::Action; 24] = {
        use crate::keybindings::Action;
        [
            Action::MoveLeft,
            Action::MoveRight,
            Action::MoveUp,
            Action::MoveDown,
            Action::NewTask,
            Action::EditTask,
            Action::DeleteTask,
            Action::ViewDetail,
            Action::ClearDone,
            Action::DuplicateTask,
            Action::CyclePriority,
            Action::SelectTask,
            Action::Boards,
            Action::Board1,
            Action::Board2,
            Action::Board3,
            Action::Board4,
            Action::Board5,
            Action::Search,
            Action::SortMenu,
            Action::TagManagement,
            Action::Options,
            Action::Quit,
            Action::Sync,
        ]
    };

    /// Fixed list of modal actions shown in the keybindings tab.
    pub const MODAL_ACTIONS: [crate::keybindings::Action; 3] = {
        use crate::keybindings::Action;
        [Action::Save, Action::NextField, Action::PrevField]
    };

    /// Total number of selectable items in the keybindings tab.
    /// Board actions + Modal actions (the "Modal" header is NOT selectable).
    pub fn options_keybindings_count(&self) -> usize {
        Self::BOARD_ACTIONS.len() + Self::MODAL_ACTIONS.len()
    }

    /// Total number of selectable theme properties (flat index 0..15).
    pub fn theme_properties_count(&self) -> usize {
        16
    }

    pub fn set_theme_property(&mut self, property_index: usize, color: ratatui::style::Color) {
        match property_index {
            0 => self.theme.focused_border = color,
            1 => self.theme.unfocused_border = color,
            2 => self.theme.cursor = color,
            3 => self.theme.selected = color,
            4 => self.theme.title = color,
            5 => self.theme.priority_high = color,
            6 => self.theme.priority_medium = color,
            7 => self.theme.priority_low = color,
            8 => self.theme.tag = color,
            9 => self.theme.due_overdue = color,
            10 => self.theme.due_today = color,
            11 => self.theme.due_soon = color,
            12 => self.theme.due_far = color,
            13 => self.theme.modal_border = color,
            14 => self.theme.modal_focused = color,
            15 => self.theme.error = color,
            _ => {}
        }
        crate::theme::save_theme(&self.theme);
    }

    pub fn get_theme_property(&self, property_index: usize) -> ratatui::style::Color {
        use ratatui::style::Color;
        match property_index {
            0 => self.theme.focused_border,
            1 => self.theme.unfocused_border,
            2 => self.theme.cursor,
            3 => self.theme.selected,
            4 => self.theme.title,
            5 => self.theme.priority_high,
            6 => self.theme.priority_medium,
            7 => self.theme.priority_low,
            8 => self.theme.tag,
            9 => self.theme.due_overdue,
            10 => self.theme.due_today,
            11 => self.theme.due_soon,
            12 => self.theme.due_far,
            13 => self.theme.modal_border,
            14 => self.theme.modal_focused,
            15 => self.theme.error,
            _ => Color::White,
        }
    }

    pub fn cycle_theme_color(&mut self) {
        let color = self.get_theme_property(self.options_cursor);
        let next = crate::theme::next_preset_color(color);
        self.set_theme_property(self.options_cursor, next);
    }

    pub fn reset_selected_theme_property(&mut self) {
        let default = crate::theme::Theme::default();
        let color = match self.options_cursor {
            0 => default.focused_border,
            1 => default.unfocused_border,
            2 => default.cursor,
            3 => default.selected,
            4 => default.title,
            5 => default.priority_high,
            6 => default.priority_medium,
            7 => default.priority_low,
            8 => default.tag,
            9 => default.due_overdue,
            10 => default.due_today,
            11 => default.due_soon,
            12 => default.due_far,
            13 => default.modal_border,
            14 => default.modal_focused,
            15 => default.error,
            _ => return,
        };
        self.set_theme_property(self.options_cursor, color);
        self.set_flash("Reset to default".to_string());
    }

    pub fn reset_all_theme_properties(&mut self) {
        self.theme = crate::theme::Theme::default();
        crate::theme::save_theme(&self.theme);
        self.set_flash("All theme colors reset to defaults".to_string());
    }

    pub fn options_cursor_up(&mut self) {
        if self.options_cursor > 0 {
            self.options_cursor -= 1;
        }
    }

    pub fn options_cursor_down(&mut self) {
        let max = if self.options_tab == 0 {
            self.options_keybindings_count().saturating_sub(1)
        } else {
            self.theme_properties_count().saturating_sub(1)
        };
        if self.options_cursor < max {
            self.options_cursor += 1;
        }
    }

    /// Map the current `options_cursor` position to an (Action, KeyContext) pair.
    ///
    /// Board actions are indices `0..BOARD_ACTIONS.len()`.
    /// Modal actions are indices `BOARD_ACTIONS.len()..` (the "Modal" header
    /// separator shown in the UI is purely visual and has no cursor slot).
    pub fn action_at_options_cursor(
        &self,
    ) -> (crate::keybindings::Action, crate::keybindings::KeyContext) {
        use crate::keybindings::KeyContext;
        let board_count = Self::BOARD_ACTIONS.len();
        if self.options_cursor < board_count {
            (Self::BOARD_ACTIONS[self.options_cursor], KeyContext::Board)
        } else {
            let modal_idx = self.options_cursor - board_count;
            (Self::MODAL_ACTIONS[modal_idx], KeyContext::Modal)
        }
    }

    /// Enter rebinding mode for the currently selected keybinding.
    pub fn start_rebinding(&mut self) {
        let (action, context) = self.action_at_options_cursor();
        self.options_rebinding = true;
        self.options_rebind_action = Some(action);
        self.options_rebind_context = Some(context);
    }

    /// Complete a rebinding with the captured key.
    pub fn finish_rebinding(&mut self, key: ratatui::crossterm::event::KeyEvent) {
        use crate::keybindings::KeyBinding;
        if let (Some(action), Some(ctx)) = (self.options_rebind_action, self.options_rebind_context)
        {
            self.keymap.rebind(ctx, action, KeyBinding::from(key));
            crate::keybindings::save_keymap(&self.keymap);
            self.set_flash(format!(
                "Rebound {} to {}",
                action.display_name(),
                crate::keybindings::key_to_string(&key)
            ));
        }
        self.options_rebinding = false;
        self.options_rebind_action = None;
        self.options_rebind_context = None;
    }

    /// Cancel a rebinding in progress.
    pub fn cancel_rebinding(&mut self) {
        self.options_rebinding = false;
        self.options_rebind_action = None;
        self.options_rebind_context = None;
    }

    /// Reset the currently selected keybinding to its default.
    pub fn reset_selected_binding(&mut self) {
        let (action, context) = self.action_at_options_cursor();
        self.keymap.reset_action(context, action);
        crate::keybindings::save_keymap(&self.keymap);
        self.set_flash(format!("Reset {} to default", action.display_name()));
    }

    /// Reset all keybindings to their defaults.
    pub fn reset_all_bindings(&mut self) {
        self.keymap.reset_all();
        crate::keybindings::save_keymap(&self.keymap);
        self.set_flash("All keybindings reset to defaults".to_string());
    }
}
