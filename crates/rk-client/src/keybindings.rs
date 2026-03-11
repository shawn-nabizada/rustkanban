use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    NewTask,
    EditTask,
    DeleteTask,
    ViewDetail,
    ClearDone,
    DuplicateTask,
    CyclePriority,
    SelectTask,
    Boards,
    Board1,
    Board2,
    Board3,
    Board4,
    Board5,
    Search,
    SortMenu,
    TagManagement,
    Options,
    Quit,
    Sync,
    Save,
    NextField,
    PrevField,
}

impl Action {
    #[allow(dead_code)]
    pub const ALL: [Action; 27] = [
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
        Action::Save,
        Action::NextField,
        Action::PrevField,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Action::MoveLeft => "move_left",
            Action::MoveRight => "move_right",
            Action::MoveUp => "move_up",
            Action::MoveDown => "move_down",
            Action::NewTask => "new_task",
            Action::EditTask => "edit_task",
            Action::DeleteTask => "delete_task",
            Action::ViewDetail => "view_detail",
            Action::ClearDone => "clear_done",
            Action::DuplicateTask => "duplicate_task",
            Action::CyclePriority => "cycle_priority",
            Action::SelectTask => "select_task",
            Action::Boards => "boards",
            Action::Board1 => "board_1",
            Action::Board2 => "board_2",
            Action::Board3 => "board_3",
            Action::Board4 => "board_4",
            Action::Board5 => "board_5",
            Action::Search => "search",
            Action::SortMenu => "sort_menu",
            Action::TagManagement => "tag_management",
            Action::Options => "options",
            Action::Quit => "quit",
            Action::Sync => "sync",
            Action::Save => "save",
            Action::NextField => "next_field",
            Action::PrevField => "prev_field",
        }
    }

    pub fn from_str(s: &str) -> Option<Action> {
        match s {
            "move_left" => Some(Action::MoveLeft),
            "move_right" => Some(Action::MoveRight),
            "move_up" => Some(Action::MoveUp),
            "move_down" => Some(Action::MoveDown),
            "new_task" => Some(Action::NewTask),
            "edit_task" => Some(Action::EditTask),
            "delete_task" => Some(Action::DeleteTask),
            "view_detail" => Some(Action::ViewDetail),
            "clear_done" => Some(Action::ClearDone),
            "duplicate_task" => Some(Action::DuplicateTask),
            "cycle_priority" => Some(Action::CyclePriority),
            "select_task" => Some(Action::SelectTask),
            "boards" => Some(Action::Boards),
            "board_1" => Some(Action::Board1),
            "board_2" => Some(Action::Board2),
            "board_3" => Some(Action::Board3),
            "board_4" => Some(Action::Board4),
            "board_5" => Some(Action::Board5),
            "search" => Some(Action::Search),
            "sort_menu" => Some(Action::SortMenu),
            "tag_management" => Some(Action::TagManagement),
            "options" => Some(Action::Options),
            "quit" => Some(Action::Quit),
            "sync" => Some(Action::Sync),
            "save" => Some(Action::Save),
            "next_field" => Some(Action::NextField),
            "prev_field" => Some(Action::PrevField),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Action::MoveLeft => "Move Left",
            Action::MoveRight => "Move Right",
            Action::MoveUp => "Move Up",
            Action::MoveDown => "Move Down",
            Action::NewTask => "New Task",
            Action::EditTask => "Edit Task",
            Action::DeleteTask => "Delete Task",
            Action::ViewDetail => "View Detail",
            Action::ClearDone => "Clear Done",
            Action::DuplicateTask => "Duplicate Task",
            Action::CyclePriority => "Cycle Priority",
            Action::SelectTask => "Select Task",
            Action::Boards => "Boards",
            Action::Board1 => "Board 1",
            Action::Board2 => "Board 2",
            Action::Board3 => "Board 3",
            Action::Board4 => "Board 4",
            Action::Board5 => "Board 5",
            Action::Search => "Search",
            Action::SortMenu => "Sort Menu",
            Action::TagManagement => "Tag Management",
            Action::Options => "Options",
            Action::Quit => "Quit",
            Action::Sync => "Sync",
            Action::Save => "Save",
            Action::NextField => "Next Field",
            Action::PrevField => "Previous Field",
        }
    }
}

/// Parse a human-readable key string into a crossterm `KeyEvent`.
///
/// Supports modifiers (`Ctrl+`, `Shift+`, `Alt+`), special keys (`Space`, `Enter`,
/// `Esc`, `Tab`, `Up`, `Down`, `Left`, `Right`, `Backspace`, `F1`-`F12`), and
/// single characters (`a`, `/`, `?`).
///
/// `"Shift+Tab"` is parsed as `KeyCode::BackTab` with `KeyModifiers::SHIFT`.
pub fn parse_key_string(s: &str) -> Option<KeyEvent> {
    if s.is_empty() {
        return None;
    }

    let parts: Vec<&str> = s.split('+').collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = KeyModifiers::NONE;

    let key_part = if parts.len() == 1 {
        parts[0]
    } else {
        // All parts except the last are modifiers
        for &modifier in &parts[..parts.len() - 1] {
            match modifier.to_lowercase().as_str() {
                "ctrl" => modifiers |= KeyModifiers::CONTROL,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                "alt" => modifiers |= KeyModifiers::ALT,
                _ => return None,
            }
        }
        match parts.last() {
            Some(k) if !k.is_empty() => k,
            _ => return None,
        }
    };

    let code = parse_key_code(key_part)?;

    // Special case: Shift+Tab becomes BackTab
    if code == KeyCode::Tab && modifiers.contains(KeyModifiers::SHIFT) {
        return Some(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    }

    Some(KeyEvent::new(code, modifiers))
}

/// Parse the key code portion of a key string (after modifiers are stripped).
fn parse_key_code(s: &str) -> Option<KeyCode> {
    // Check for special key names (case-insensitive)
    match s.to_lowercase().as_str() {
        "space" => return Some(KeyCode::Char(' ')),
        "enter" => return Some(KeyCode::Enter),
        "esc" | "escape" => return Some(KeyCode::Esc),
        "tab" => return Some(KeyCode::Tab),
        "backtab" => return Some(KeyCode::BackTab),
        "backspace" => return Some(KeyCode::Backspace),
        "up" => return Some(KeyCode::Up),
        "down" => return Some(KeyCode::Down),
        "left" => return Some(KeyCode::Left),
        "right" => return Some(KeyCode::Right),
        "delete" | "del" => return Some(KeyCode::Delete),
        "insert" | "ins" => return Some(KeyCode::Insert),
        "home" => return Some(KeyCode::Home),
        "end" => return Some(KeyCode::End),
        "pageup" => return Some(KeyCode::PageUp),
        "pagedown" => return Some(KeyCode::PageDown),
        _ => {}
    }

    // Check for function keys F1-F12
    let lower = s.to_lowercase();
    if let Some(num_str) = lower.strip_prefix('f') {
        if let Ok(n) = num_str.parse::<u8>() {
            if (1..=12).contains(&n) {
                return Some(KeyCode::F(n));
            }
        }
    }

    // Single character
    let mut chars = s.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        // More than one character and not a recognized special key
        return None;
    }

    Some(KeyCode::Char(c))
}

/// Convert a crossterm `KeyEvent` back to a human-readable key string.
///
/// `KeyCode::BackTab` is represented as `"Shift+Tab"`.
pub fn key_to_string(key: &KeyEvent) -> String {
    let mut parts = Vec::new();

    // BackTab is special: always represented as Shift+Tab
    if key.code == KeyCode::BackTab {
        return "Shift+Tab".to_string();
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl".to_string());
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt".to_string());
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift".to_string());
    }

    let key_str = match key.code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::F(n) => format!("F{n}"),
        KeyCode::BackTab => unreachable!(), // handled above
        _ => format!("{:?}", key.code),
    };

    parts.push(key_str);
    parts.join("+")
}

// ---------------------------------------------------------------------------
// KeyContext
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyContext {
    Board, // Board + Selected modes (the main view)
    Modal, // NewTask + EditTask modes
}

// ---------------------------------------------------------------------------
// KeyBinding
// ---------------------------------------------------------------------------

/// Wrapper around the two fields of `KeyEvent` that we care about for keybinding
/// lookup. `KeyEvent` itself has additional fields (`kind`, `state`) that make
/// it inconvenient for use as a `HashMap` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl Hash for KeyBinding {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.code.hash(state);
        self.modifiers.bits().hash(state);
    }
}

impl KeyBinding {
    /// Convert this `KeyBinding` to a human-readable key string (e.g. `"Ctrl+s"`).
    pub fn to_key_string(self) -> String {
        let event = KeyEvent::new(self.code, self.modifiers);
        key_to_string(&event)
    }
}

/// Normalize modifiers so that only SHIFT, CONTROL, and ALT are kept.
/// Some terminals (kitty protocol, etc.) report extra bits (SUPER, HYPER, META)
/// that would cause HashMap lookups to fail. Also strip SHIFT for character keys
/// since the character case already reflects shift state (e.g. 'Q' vs 'q').
fn normalize_modifiers(code: KeyCode, mods: KeyModifiers) -> KeyModifiers {
    let mut m = mods & (KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT);
    // For character keys, SHIFT is encoded in the char itself ('q' vs 'Q')
    if matches!(code, KeyCode::Char(_)) {
        m -= KeyModifiers::SHIFT;
    }
    // BackTab already implies SHIFT
    if matches!(code, KeyCode::BackTab) {
        m -= KeyModifiers::SHIFT;
    }
    m
}

impl From<KeyEvent> for KeyBinding {
    fn from(event: KeyEvent) -> Self {
        Self {
            code: event.code,
            modifiers: normalize_modifiers(event.code, event.modifiers),
        }
    }
}

impl From<&KeyEvent> for KeyBinding {
    fn from(event: &KeyEvent) -> Self {
        Self {
            code: event.code,
            modifiers: normalize_modifiers(event.code, event.modifiers),
        }
    }
}

// ---------------------------------------------------------------------------
// KeyMap
// ---------------------------------------------------------------------------

pub struct KeyMap {
    /// (context, key) -> action
    bindings: HashMap<(KeyContext, KeyBinding), Action>,
}

impl KeyMap {
    /// Look up what action a key triggers in the given context.
    pub fn action_for(&self, ctx: KeyContext, key: &KeyEvent) -> Option<Action> {
        let binding = KeyBinding::from(key);
        self.bindings.get(&(ctx, binding)).copied()
    }

    /// Look up the first key bound to an action in a context (for display in UI).
    pub fn key_for(&self, ctx: KeyContext, action: Action) -> Option<KeyBinding> {
        self.bindings
            .iter()
            .find(|(&(c, _), &a)| c == ctx && a == action)
            .map(|(&(_, kb), _)| kb)
    }

    /// Get all bindings for a context (for rendering the keybindings tab).
    pub fn bindings_for_context(&self, ctx: KeyContext) -> Vec<(Action, KeyBinding)> {
        self.bindings
            .iter()
            .filter(|(&(c, _), _)| c == ctx)
            .map(|(&(_, kb), &a)| (a, kb))
            .collect()
    }

    /// Rebind: remove any existing binding for this action in this context,
    /// remove any existing binding for this key in this context (conflict),
    /// then insert the new binding.
    pub fn rebind(&mut self, ctx: KeyContext, action: Action, key: KeyBinding) {
        // Remove all existing bindings for this action in this context
        self.bindings
            .retain(|&(c, _), &mut a| !(c == ctx && a == action));
        // Remove any existing binding for this key in this context (conflict)
        self.bindings.remove(&(ctx, key));
        // Insert the new binding
        self.bindings.insert((ctx, key), action);
    }

    /// Reset a single action to its default binding.
    pub fn reset_action(&mut self, ctx: KeyContext, action: Action) {
        // Remove all current bindings for this action in this context
        self.bindings
            .retain(|&(c, _), &mut a| !(c == ctx && a == action));
        // Re-insert the defaults for this action
        let defaults = KeyMap::default();
        for (&(c, kb), &a) in &defaults.bindings {
            if c == ctx && a == action {
                // Remove any conflicting binding for this key
                self.bindings.remove(&(ctx, kb));
                self.bindings.insert((ctx, kb), action);
            }
        }
    }

    /// Reset all bindings to defaults.
    pub fn reset_all(&mut self) {
        *self = KeyMap::default();
    }

    /// Helper to insert a binding during construction.
    fn bind(&mut self, ctx: KeyContext, code: KeyCode, modifiers: KeyModifiers, action: Action) {
        let kb = KeyBinding {
            code,
            modifiers: normalize_modifiers(code, modifiers),
        };
        self.bindings.insert((ctx, kb), action);
    }
}

impl Default for KeyMap {
    fn default() -> Self {
        let mut km = KeyMap {
            bindings: HashMap::new(),
        };

        // ---- Board context ----
        let b = KeyContext::Board;

        // MoveLeft: j, Left
        km.bind(b, KeyCode::Char('j'), KeyModifiers::NONE, Action::MoveLeft);
        km.bind(b, KeyCode::Left, KeyModifiers::NONE, Action::MoveLeft);

        // MoveRight: l, Right
        km.bind(b, KeyCode::Char('l'), KeyModifiers::NONE, Action::MoveRight);
        km.bind(b, KeyCode::Right, KeyModifiers::NONE, Action::MoveRight);

        // MoveUp: Up, BackTab
        km.bind(b, KeyCode::Up, KeyModifiers::NONE, Action::MoveUp);
        km.bind(b, KeyCode::BackTab, KeyModifiers::SHIFT, Action::MoveUp);

        // MoveDown: Down, Tab
        km.bind(b, KeyCode::Down, KeyModifiers::NONE, Action::MoveDown);
        km.bind(b, KeyCode::Tab, KeyModifiers::NONE, Action::MoveDown);

        // NewTask: Space
        km.bind(b, KeyCode::Char(' '), KeyModifiers::NONE, Action::NewTask);

        // EditTask: e
        km.bind(b, KeyCode::Char('e'), KeyModifiers::NONE, Action::EditTask);

        // DeleteTask: d
        km.bind(
            b,
            KeyCode::Char('d'),
            KeyModifiers::NONE,
            Action::DeleteTask,
        );

        // ViewDetail: Enter
        km.bind(b, KeyCode::Enter, KeyModifiers::NONE, Action::ViewDetail);

        // ClearDone: Ctrl+d
        km.bind(
            b,
            KeyCode::Char('d'),
            KeyModifiers::CONTROL,
            Action::ClearDone,
        );

        // DuplicateTask: c
        km.bind(
            b,
            KeyCode::Char('c'),
            KeyModifiers::NONE,
            Action::DuplicateTask,
        );

        // CyclePriority: p
        km.bind(
            b,
            KeyCode::Char('p'),
            KeyModifiers::NONE,
            Action::CyclePriority,
        );

        // SelectTask: k
        km.bind(
            b,
            KeyCode::Char('k'),
            KeyModifiers::NONE,
            Action::SelectTask,
        );

        // Boards: b
        km.bind(b, KeyCode::Char('b'), KeyModifiers::NONE, Action::Boards);

        // Board1-5: 1-5
        km.bind(b, KeyCode::Char('1'), KeyModifiers::NONE, Action::Board1);
        km.bind(b, KeyCode::Char('2'), KeyModifiers::NONE, Action::Board2);
        km.bind(b, KeyCode::Char('3'), KeyModifiers::NONE, Action::Board3);
        km.bind(b, KeyCode::Char('4'), KeyModifiers::NONE, Action::Board4);
        km.bind(b, KeyCode::Char('5'), KeyModifiers::NONE, Action::Board5);

        // Search: /
        km.bind(b, KeyCode::Char('/'), KeyModifiers::NONE, Action::Search);

        // SortMenu: s
        km.bind(b, KeyCode::Char('s'), KeyModifiers::NONE, Action::SortMenu);

        // TagManagement: t
        km.bind(
            b,
            KeyCode::Char('t'),
            KeyModifiers::NONE,
            Action::TagManagement,
        );

        // Options: o
        km.bind(b, KeyCode::Char('o'), KeyModifiers::NONE, Action::Options);

        // Quit: q, Esc
        km.bind(b, KeyCode::Char('q'), KeyModifiers::NONE, Action::Quit);
        km.bind(b, KeyCode::Esc, KeyModifiers::NONE, Action::Quit);

        // Sync: Ctrl+r
        km.bind(b, KeyCode::Char('r'), KeyModifiers::CONTROL, Action::Sync);

        // ---- Modal context ----
        let m = KeyContext::Modal;

        // Save: Ctrl+s, Ctrl+Enter
        km.bind(m, KeyCode::Char('s'), KeyModifiers::CONTROL, Action::Save);
        km.bind(m, KeyCode::Enter, KeyModifiers::CONTROL, Action::Save);

        // NextField: Tab
        km.bind(m, KeyCode::Tab, KeyModifiers::NONE, Action::NextField);

        // PrevField: BackTab
        km.bind(m, KeyCode::BackTab, KeyModifiers::SHIFT, Action::PrevField);

        km
    }
}

// ---------------------------------------------------------------------------
// TOML config loading and saving
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
struct KeysConfig {
    board: HashMap<String, String>,
    modal: HashMap<String, String>,
}

/// Return the path to the keybindings config file: `~/.config/rustkanban/keys.toml`.
pub fn keys_path() -> std::path::PathBuf {
    dirs::config_dir()
        .expect("config dir")
        .join("rustkanban")
        .join("keys.toml")
}

/// Load a `KeyMap` from the user's `keys.toml`, merging overrides onto the defaults.
///
/// If the config file doesn't exist or can't be parsed, the default `KeyMap` is returned.
pub fn load_keymap() -> KeyMap {
    let path = keys_path();
    if !path.exists() {
        return KeyMap::default();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return KeyMap::default(),
    };
    let config: KeysConfig = match toml::from_str(&content) {
        Ok(c) => c,
        Err(_) => return KeyMap::default(),
    };
    let mut keymap = KeyMap::default();
    apply_config_section(&mut keymap, KeyContext::Board, &config.board);
    apply_config_section(&mut keymap, KeyContext::Modal, &config.modal);
    keymap
}

/// Apply a TOML config section (action-name -> key-string map) to a `KeyMap`.
///
/// Unknown action names and unparseable key strings are silently skipped
/// (with a warning printed to stderr).
fn apply_config_section(keymap: &mut KeyMap, ctx: KeyContext, section: &HashMap<String, String>) {
    for (action_str, key_str) in section {
        let action = match Action::from_str(action_str) {
            Some(a) => a,
            None => {
                eprintln!("Warning: unknown action '{}'", action_str);
                continue;
            }
        };
        let key = match parse_key_string(key_str) {
            Some(k) => k,
            None => {
                eprintln!("Warning: invalid key '{}' for '{}'", key_str, action_str);
                continue;
            }
        };
        keymap.rebind(ctx, action, KeyBinding::from(key));
    }
}

/// Save a `KeyMap` to `~/.config/rustkanban/keys.toml`.
pub fn save_keymap(keymap: &KeyMap) {
    let config = keymap_to_config(keymap);
    let toml_str = toml::to_string_pretty(&config).unwrap_or_default();
    let path = keys_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, toml_str);
}

/// Convert a `KeyMap` into a serializable `KeysConfig`.
/// Only saves actions that differ from defaults, so unchanged actions
/// keep their default multi-key bindings on reload.
fn keymap_to_config(keymap: &KeyMap) -> KeysConfig {
    let defaults = KeyMap::default();
    let mut config = KeysConfig::default();

    // For each context, only save actions whose bindings differ from defaults
    for ctx in [KeyContext::Board, KeyContext::Modal] {
        let section = match ctx {
            KeyContext::Board => &mut config.board,
            KeyContext::Modal => &mut config.modal,
        };
        let current = keymap.bindings_for_context(ctx);
        let default = defaults.bindings_for_context(ctx);

        // Build sets of (action, binding) for comparison
        let current_set: std::collections::HashSet<_> = current.iter().cloned().collect();
        let default_set: std::collections::HashSet<_> = default.iter().cloned().collect();

        if current_set != default_set {
            // Something changed — save all current bindings (deduped by action)
            for (action, binding) in &current {
                section
                    .entry(action.as_str().to_string())
                    .or_insert_with(|| binding.to_key_string());
            }
        }
    }
    config
}

/// Convert a `KeyMap` into a full `KeysConfig` with all bindings (for default_keys_toml).
fn keymap_to_full_config(keymap: &KeyMap) -> KeysConfig {
    let mut config = KeysConfig::default();
    for (action, binding) in keymap.bindings_for_context(KeyContext::Board) {
        config
            .board
            .entry(action.as_str().to_string())
            .or_insert_with(|| binding.to_key_string());
    }
    for (action, binding) in keymap.bindings_for_context(KeyContext::Modal) {
        config
            .modal
            .entry(action.as_str().to_string())
            .or_insert_with(|| binding.to_key_string());
    }
    config
}

/// Generate a default `keys.toml` string with a header comment.
pub fn default_keys_toml() -> String {
    let keymap = KeyMap::default();
    let config = keymap_to_full_config(&keymap);
    let mut out = String::from("# RustKanban Keybindings\n# Format: action = \"key\"\n\n");
    out.push_str(&toml::to_string_pretty(&config).unwrap_or_default());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_char() {
        let key = parse_key_string("j").unwrap();
        assert_eq!(key.code, KeyCode::Char('j'));
        assert_eq!(key.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_parse_ctrl_modifier() {
        let key = parse_key_string("Ctrl+r").unwrap();
        assert_eq!(key.code, KeyCode::Char('r'));
        assert!(key.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn test_parse_special_keys() {
        assert_eq!(parse_key_string("Space").unwrap().code, KeyCode::Char(' '));
        assert_eq!(parse_key_string("Enter").unwrap().code, KeyCode::Enter);
        assert_eq!(parse_key_string("Up").unwrap().code, KeyCode::Up);
        assert_eq!(parse_key_string("F1").unwrap().code, KeyCode::F(1));
    }

    #[test]
    fn test_roundtrip() {
        for s in &[
            "j",
            "Ctrl+r",
            "Space",
            "F5",
            "Up",
            "Esc",
            "Tab",
            "Shift+Tab",
            "D",
            "/",
            "?",
        ] {
            let key = parse_key_string(s).unwrap();
            let result = key_to_string(&key);
            let key2 = parse_key_string(&result).unwrap();
            assert_eq!(
                key.code, key2.code,
                "roundtrip failed for {}: got {}",
                s, result
            );
            assert_eq!(
                key.modifiers, key2.modifiers,
                "modifier roundtrip failed for {}",
                s
            );
        }
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_key_string("").is_none());
        assert!(parse_key_string("Ctrl+").is_none());
    }

    #[test]
    fn test_action_str_roundtrip() {
        for action in Action::ALL {
            let s = action.as_str();
            assert_eq!(
                Action::from_str(s),
                Some(action),
                "roundtrip failed for {:?}",
                action
            );
        }
    }

    #[test]
    fn test_parse_shift_tab_is_backtab() {
        let key = parse_key_string("Shift+Tab").unwrap();
        assert_eq!(key.code, KeyCode::BackTab);
        assert!(key.modifiers.contains(KeyModifiers::SHIFT));
    }

    #[test]
    fn test_backtab_to_string() {
        let key = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(key_to_string(&key), "Shift+Tab");
    }

    #[test]
    fn test_parse_case_insensitive_modifiers() {
        let key = parse_key_string("ctrl+r").unwrap();
        assert_eq!(key.code, KeyCode::Char('r'));
        assert!(key.modifiers.contains(KeyModifiers::CONTROL));

        let key = parse_key_string("CTRL+r").unwrap();
        assert_eq!(key.code, KeyCode::Char('r'));
        assert!(key.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn test_parse_case_insensitive_special_keys() {
        assert_eq!(parse_key_string("space").unwrap().code, KeyCode::Char(' '));
        assert_eq!(parse_key_string("ENTER").unwrap().code, KeyCode::Enter);
        assert_eq!(parse_key_string("esc").unwrap().code, KeyCode::Esc);
    }

    #[test]
    fn test_parse_uppercase_char() {
        let key = parse_key_string("D").unwrap();
        assert_eq!(key.code, KeyCode::Char('D'));
        assert_eq!(key.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_parse_function_keys() {
        for n in 1..=12 {
            let s = format!("F{n}");
            let key = parse_key_string(&s).unwrap();
            assert_eq!(key.code, KeyCode::F(n));
        }
    }

    #[test]
    fn test_parse_alt_modifier() {
        let key = parse_key_string("Alt+x").unwrap();
        assert_eq!(key.code, KeyCode::Char('x'));
        assert!(key.modifiers.contains(KeyModifiers::ALT));
    }

    #[test]
    fn test_parse_multiple_modifiers() {
        let key = parse_key_string("Ctrl+Alt+x").unwrap();
        assert_eq!(key.code, KeyCode::Char('x'));
        assert!(key.modifiers.contains(KeyModifiers::CONTROL));
        assert!(key.modifiers.contains(KeyModifiers::ALT));
    }

    #[test]
    fn test_key_to_string_simple() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(key_to_string(&key), "a");
    }

    #[test]
    fn test_key_to_string_ctrl() {
        let key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(key_to_string(&key), "Ctrl+s");
    }

    #[test]
    fn test_key_to_string_space() {
        let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        assert_eq!(key_to_string(&key), "Space");
    }

    #[test]
    fn test_action_all_count() {
        assert_eq!(Action::ALL.len(), 27);
    }

    #[test]
    fn test_action_display_names() {
        assert_eq!(Action::MoveLeft.display_name(), "Move Left");
        assert_eq!(Action::Quit.display_name(), "Quit");
        assert_eq!(Action::PrevField.display_name(), "Previous Field");
        assert_eq!(Action::TagManagement.display_name(), "Tag Management");
    }

    #[test]
    fn test_parse_arrow_keys() {
        assert_eq!(parse_key_string("Left").unwrap().code, KeyCode::Left);
        assert_eq!(parse_key_string("Right").unwrap().code, KeyCode::Right);
        assert_eq!(parse_key_string("Down").unwrap().code, KeyCode::Down);
    }

    #[test]
    fn test_parse_backspace() {
        assert_eq!(
            parse_key_string("Backspace").unwrap().code,
            KeyCode::Backspace
        );
    }

    #[test]
    fn test_parse_invalid_modifier() {
        assert!(parse_key_string("Super+x").is_none());
    }

    #[test]
    fn test_parse_invalid_multichar() {
        assert!(parse_key_string("abc").is_none());
    }

    // -----------------------------------------------------------------------
    // KeyBinding tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_keybinding_from_key_event() {
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        let kb = KeyBinding::from(event);
        assert_eq!(kb.code, KeyCode::Char('a'));
        assert_eq!(kb.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_keybinding_from_ref_key_event() {
        let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let kb = KeyBinding::from(&event);
        assert_eq!(kb.code, KeyCode::Enter);
        assert_eq!(kb.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_keybinding_hash_eq() {
        use std::collections::HashSet;
        let kb1 = KeyBinding {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
        };
        let kb2 = KeyBinding {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
        };
        let kb3 = KeyBinding {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::CONTROL,
        };
        let mut set = HashSet::new();
        set.insert(kb1);
        assert!(set.contains(&kb2));
        assert!(!set.contains(&kb3));
    }

    // -----------------------------------------------------------------------
    // KeyMap default bindings tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_default_keymap_board_actions() {
        let km = KeyMap::default();
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(
            km.action_for(KeyContext::Board, &key),
            Some(Action::MoveLeft)
        );
    }

    #[test]
    fn test_default_keymap_ctrl_modifier() {
        let km = KeyMap::default();
        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(km.action_for(KeyContext::Board, &key), Some(Action::Sync));
    }

    #[test]
    fn test_default_keymap_move_left_alternatives() {
        let km = KeyMap::default();
        let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let left = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(km.action_for(KeyContext::Board, &j), Some(Action::MoveLeft));
        assert_eq!(
            km.action_for(KeyContext::Board, &left),
            Some(Action::MoveLeft)
        );
    }

    #[test]
    fn test_default_keymap_move_right() {
        let km = KeyMap::default();
        let l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
        let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(
            km.action_for(KeyContext::Board, &l),
            Some(Action::MoveRight)
        );
        assert_eq!(
            km.action_for(KeyContext::Board, &right),
            Some(Action::MoveRight)
        );
    }

    #[test]
    fn test_default_keymap_move_up_down() {
        let km = KeyMap::default();
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)
            ),
            Some(Action::MoveUp)
        );
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)
            ),
            Some(Action::MoveUp)
        );
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)
            ),
            Some(Action::MoveDown)
        );
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)
            ),
            Some(Action::MoveDown)
        );
    }

    #[test]
    fn test_default_keymap_new_task() {
        let km = KeyMap::default();
        let space = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        assert_eq!(
            km.action_for(KeyContext::Board, &space),
            Some(Action::NewTask)
        );
    }

    #[test]
    fn test_default_keymap_quit_multiple_keys() {
        let km = KeyMap::default();
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)
            ),
            Some(Action::Quit)
        );
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
            ),
            Some(Action::Quit)
        );
    }

    #[test]
    fn test_default_keymap_board_numbers() {
        let km = KeyMap::default();
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE)
            ),
            Some(Action::Board1)
        );
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE)
            ),
            Some(Action::Board5)
        );
    }

    #[test]
    fn test_default_keymap_all_board_actions_present() {
        let km = KeyMap::default();
        let board_actions = [
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
        ];
        for action in board_actions {
            assert!(
                km.key_for(KeyContext::Board, action).is_some(),
                "Board action {:?} should have at least one binding",
                action
            );
        }
    }

    #[test]
    fn test_default_keymap_modal_save() {
        let km = KeyMap::default();
        let ctrl_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        let ctrl_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL);
        assert_eq!(
            km.action_for(KeyContext::Modal, &ctrl_s),
            Some(Action::Save)
        );
        assert_eq!(
            km.action_for(KeyContext::Modal, &ctrl_enter),
            Some(Action::Save)
        );
    }

    #[test]
    fn test_default_keymap_modal_fields() {
        let km = KeyMap::default();
        let tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let backtab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(
            km.action_for(KeyContext::Modal, &tab),
            Some(Action::NextField)
        );
        assert_eq!(
            km.action_for(KeyContext::Modal, &backtab),
            Some(Action::PrevField)
        );
    }

    #[test]
    fn test_default_keymap_no_cross_context() {
        let km = KeyMap::default();
        // 'j' is MoveLeft in Board, should not be anything in Modal
        let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(km.action_for(KeyContext::Modal, &j), None);
    }

    #[test]
    fn test_default_keymap_unbound_key() {
        let km = KeyMap::default();
        let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
        assert_eq!(km.action_for(KeyContext::Board, &key), None);
    }

    // -----------------------------------------------------------------------
    // action_for tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_action_for_returns_none_for_unknown() {
        let km = KeyMap::default();
        let key = KeyEvent::new(KeyCode::F(12), KeyModifiers::ALT);
        assert_eq!(km.action_for(KeyContext::Board, &key), None);
    }

    // -----------------------------------------------------------------------
    // key_for (reverse lookup) tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_key_for_reverse_lookup() {
        let km = KeyMap::default();
        let binding = km.key_for(KeyContext::Board, Action::Search);
        assert!(binding.is_some());
        assert_eq!(binding.unwrap().code, KeyCode::Char('/'));
    }

    #[test]
    fn test_key_for_returns_none_for_wrong_context() {
        let km = KeyMap::default();
        // Search is only in Board context
        assert!(km.key_for(KeyContext::Modal, Action::Search).is_none());
    }

    // -----------------------------------------------------------------------
    // bindings_for_context tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_bindings_for_context_board() {
        let km = KeyMap::default();
        let bindings = km.bindings_for_context(KeyContext::Board);
        // Should have many bindings (multiple keys per action)
        assert!(bindings.len() > 20);
        // All should be board actions (not modal)
        for (action, _) in &bindings {
            assert!(
                !matches!(action, Action::Save | Action::NextField | Action::PrevField),
                "Board context should not contain modal-only action {:?}",
                action
            );
        }
    }

    #[test]
    fn test_bindings_for_context_modal() {
        let km = KeyMap::default();
        let bindings = km.bindings_for_context(KeyContext::Modal);
        // Should have exactly 4 bindings: Ctrl+s, Ctrl+Enter, Tab, BackTab
        assert_eq!(bindings.len(), 4);
    }

    // -----------------------------------------------------------------------
    // rebind tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_rebind() {
        let mut km = KeyMap::default();
        let new_key = KeyBinding {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
        };
        km.rebind(KeyContext::Board, Action::DeleteTask, new_key);
        // Old key no longer works
        let old = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(km.action_for(KeyContext::Board, &old), None);
        // New key works
        let new = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!(
            km.action_for(KeyContext::Board, &new),
            Some(Action::DeleteTask)
        );
    }

    #[test]
    fn test_rebind_removes_all_alternate_keys() {
        let mut km = KeyMap::default();
        // MoveLeft has two keys: j and Left
        let new_key = KeyBinding {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
        };
        km.rebind(KeyContext::Board, Action::MoveLeft, new_key);
        // Both old keys should be gone
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)
            ),
            None
        );
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)
            ),
            None
        );
        // New key works
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)
            ),
            Some(Action::MoveLeft)
        );
    }

    #[test]
    fn test_rebind_resolves_conflict() {
        let mut km = KeyMap::default();
        // 'q' is bound to Quit. Rebind Search to 'q' — should remove the Quit binding.
        let q_key = KeyBinding {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        };
        km.rebind(KeyContext::Board, Action::Search, q_key);
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(km.action_for(KeyContext::Board, &q), Some(Action::Search));
        // Quit should still work via Esc (conflict only removes the specific key)
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(km.action_for(KeyContext::Board, &esc), Some(Action::Quit));
    }

    #[test]
    fn test_rebind_does_not_affect_other_context() {
        let mut km = KeyMap::default();
        let tab = KeyBinding {
            code: KeyCode::Tab,
            modifiers: KeyModifiers::NONE,
        };
        // Tab is MoveDown in Board and NextField in Modal
        km.rebind(KeyContext::Board, Action::Search, tab);
        // Modal should still have Tab -> NextField
        assert_eq!(
            km.action_for(
                KeyContext::Modal,
                &KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)
            ),
            Some(Action::NextField)
        );
    }

    // -----------------------------------------------------------------------
    // reset_action tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_reset_action() {
        let mut km = KeyMap::default();
        let new_key = KeyBinding {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
        };
        km.rebind(KeyContext::Board, Action::Quit, new_key);
        // Verify rebind worked
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(km.action_for(KeyContext::Board, &q), None);
        // Reset
        km.reset_action(KeyContext::Board, Action::Quit);
        assert_eq!(km.action_for(KeyContext::Board, &q), Some(Action::Quit));
    }

    #[test]
    fn test_reset_action_restores_all_defaults() {
        let mut km = KeyMap::default();
        let new_key = KeyBinding {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
        };
        km.rebind(KeyContext::Board, Action::MoveLeft, new_key);
        km.reset_action(KeyContext::Board, Action::MoveLeft);
        // Both default keys should work again
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)
            ),
            Some(Action::MoveLeft)
        );
        assert_eq!(
            km.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)
            ),
            Some(Action::MoveLeft)
        );
    }

    // -----------------------------------------------------------------------
    // reset_all tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_reset_all() {
        let mut km = KeyMap::default();
        // Rebind several things
        km.rebind(
            KeyContext::Board,
            Action::Quit,
            KeyBinding {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::NONE,
            },
        );
        km.rebind(
            KeyContext::Board,
            Action::Search,
            KeyBinding {
                code: KeyCode::Char('y'),
                modifiers: KeyModifiers::NONE,
            },
        );
        km.reset_all();
        // Defaults should be restored
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(km.action_for(KeyContext::Board, &q), Some(Action::Quit));
        let slash = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        assert_eq!(
            km.action_for(KeyContext::Board, &slash),
            Some(Action::Search)
        );
    }

    // -----------------------------------------------------------------------
    // KeyBinding::to_key_string tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_keybinding_to_key_string() {
        let kb = KeyBinding {
            code: KeyCode::Char('s'),
            modifiers: KeyModifiers::CONTROL,
        };
        assert_eq!(kb.to_key_string(), "Ctrl+s");
    }

    #[test]
    fn test_keybinding_to_key_string_simple() {
        let kb = KeyBinding {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(kb.to_key_string(), "q");
    }

    #[test]
    fn test_keybinding_to_key_string_backtab() {
        let kb = KeyBinding {
            code: KeyCode::BackTab,
            modifiers: KeyModifiers::SHIFT,
        };
        assert_eq!(kb.to_key_string(), "Shift+Tab");
    }

    // -----------------------------------------------------------------------
    // TOML config loading and saving tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_custom_config() {
        // Test that apply_config_section correctly overrides defaults
        let mut keymap = KeyMap::default();
        let mut section = HashMap::new();
        section.insert("move_left".to_string(), "h".to_string());
        apply_config_section(&mut keymap, KeyContext::Board, &section);

        let h_key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
        assert_eq!(
            keymap.action_for(KeyContext::Board, &h_key),
            Some(Action::MoveLeft)
        );
        // Old default 'j' should no longer be MoveLeft
        let j_key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(keymap.action_for(KeyContext::Board, &j_key), None);
    }

    #[test]
    fn test_invalid_config_ignored() {
        let mut keymap = KeyMap::default();
        let mut section = HashMap::new();
        section.insert("not_real_action".to_string(), "x".to_string());
        section.insert("quit".to_string(), "BADKEY!!!".to_string());
        apply_config_section(&mut keymap, KeyContext::Board, &section);
        // quit should still have its default
        let q_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(
            keymap.action_for(KeyContext::Board, &q_key),
            Some(Action::Quit)
        );
    }

    #[test]
    fn test_default_keys_toml_not_empty() {
        let toml = default_keys_toml();
        assert!(toml.contains("[board]"));
        assert!(toml.contains("quit"));
    }

    #[test]
    fn test_default_keys_toml_contains_modal() {
        let toml = default_keys_toml();
        assert!(toml.contains("[modal]"));
        assert!(toml.contains("save"));
    }

    #[test]
    fn test_default_keys_toml_has_header_comment() {
        let toml = default_keys_toml();
        assert!(toml.starts_with("# RustKanban Keybindings"));
    }

    #[test]
    fn test_keymap_to_config_roundtrip() {
        let keymap = KeyMap::default();
        let config = keymap_to_config(&keymap);

        // The config serializes one binding per action (HashMap key = action name).
        // Verify every action in the config can be loaded back and maps correctly.
        let mut keymap2 = KeyMap::default();
        apply_config_section(&mut keymap2, KeyContext::Board, &config.board);
        apply_config_section(&mut keymap2, KeyContext::Modal, &config.modal);

        // Every action that was in the config should be reachable via the key
        // listed in the config.
        for (action_str, key_str) in &config.board {
            let action = Action::from_str(action_str).unwrap();
            let key = parse_key_string(key_str).unwrap();
            assert_eq!(
                keymap2.action_for(KeyContext::Board, &key),
                Some(action),
                "Board action '{}' not found via key '{}' after roundtrip",
                action_str,
                key_str
            );
        }
        for (action_str, key_str) in &config.modal {
            let action = Action::from_str(action_str).unwrap();
            let key = parse_key_string(key_str).unwrap();
            assert_eq!(
                keymap2.action_for(KeyContext::Modal, &key),
                Some(action),
                "Modal action '{}' not found via key '{}' after roundtrip",
                action_str,
                key_str
            );
        }
    }

    #[test]
    fn test_keys_config_deserialize_empty() {
        let config: KeysConfig = toml::from_str("").unwrap();
        assert!(config.board.is_empty());
        assert!(config.modal.is_empty());
    }

    #[test]
    fn test_keys_config_deserialize_partial() {
        let toml_str = r#"
[board]
quit = "x"
"#;
        let config: KeysConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.board.get("quit"), Some(&"x".to_string()));
        assert!(config.modal.is_empty());
    }

    #[test]
    fn test_apply_config_section_multiple_overrides() {
        let mut keymap = KeyMap::default();
        let mut section = HashMap::new();
        section.insert("move_left".to_string(), "a".to_string());
        section.insert("move_right".to_string(), "d".to_string());
        section.insert("quit".to_string(), "x".to_string());
        apply_config_section(&mut keymap, KeyContext::Board, &section);

        assert_eq!(
            keymap.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)
            ),
            Some(Action::MoveLeft)
        );
        assert_eq!(
            keymap.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE)
            ),
            Some(Action::MoveRight)
        );
        assert_eq!(
            keymap.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)
            ),
            Some(Action::Quit)
        );
        // Old defaults should be gone
        assert_eq!(
            keymap.action_for(
                KeyContext::Board,
                &KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)
            ),
            None
        );
    }

    #[test]
    fn test_apply_config_section_modal() {
        let mut keymap = KeyMap::default();
        let mut section = HashMap::new();
        section.insert("save".to_string(), "Ctrl+w".to_string());
        apply_config_section(&mut keymap, KeyContext::Modal, &section);

        let ctrl_w = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL);
        assert_eq!(
            keymap.action_for(KeyContext::Modal, &ctrl_w),
            Some(Action::Save)
        );
        // Old Ctrl+s should no longer be Save
        let ctrl_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(keymap.action_for(KeyContext::Modal, &ctrl_s), None);
    }

    #[test]
    fn test_keys_path() {
        let path = keys_path();
        assert!(path.ends_with("rustkanban/keys.toml"));
    }
}
