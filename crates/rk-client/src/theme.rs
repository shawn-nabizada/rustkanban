use ratatui::style::Color;
use serde::Deserialize;

use crate::model::Priority;

#[derive(Debug, Clone)]
pub struct Theme {
    pub focused_border: Color,
    pub unfocused_border: Color,
    pub cursor: Color,
    pub selected: Color,
    pub title: Color,
    pub priority_high: Color,
    pub priority_medium: Color,
    pub priority_low: Color,
    pub tag: Color,
    pub due_overdue: Color,
    pub due_today: Color,
    pub due_soon: Color,
    pub due_far: Color,
    pub modal_border: Color,
    pub modal_focused: Color,
    pub error: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            focused_border: Color::Green,
            unfocused_border: Color::Gray,
            cursor: Color::Cyan,
            selected: Color::Yellow,
            title: Color::White,
            priority_high: Color::Red,
            priority_medium: Color::Yellow,
            priority_low: Color::Green,
            tag: Color::Cyan,
            due_overdue: Color::Red,
            due_today: Color::Red,
            due_soon: Color::Yellow,
            due_far: Color::Gray,
            modal_border: Color::Cyan,
            modal_focused: Color::Yellow,
            error: Color::Red,
        }
    }
}

impl Theme {
    pub fn priority_color(&self, p: &Priority) -> Color {
        match p {
            Priority::High => self.priority_high,
            Priority::Medium => self.priority_medium,
            Priority::Low => self.priority_low,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ThemeConfig {
    board: BoardConfig,
    priority: PriorityConfig,
    tags: TagConfig,
    due_date: DueDateConfig,
    modal: ModalConfig,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct BoardConfig {
    focused_border: Option<String>,
    unfocused_border: Option<String>,
    cursor: Option<String>,
    selected: Option<String>,
    title: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct PriorityConfig {
    high: Option<String>,
    medium: Option<String>,
    low: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct TagConfig {
    color: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct DueDateConfig {
    overdue: Option<String>,
    today: Option<String>,
    soon: Option<String>,
    far: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ModalConfig {
    border: Option<String>,
    focused: Option<String>,
    error: Option<String>,
}

fn parse_color(s: &str) -> Option<Color> {
    match s.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        _ if s.starts_with('#') && s.len() == 7 => {
            let r = u8::from_str_radix(&s[1..3], 16).ok()?;
            let g = u8::from_str_radix(&s[3..5], 16).ok()?;
            let b = u8::from_str_radix(&s[5..7], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

fn apply(target: &mut Color, value: &Option<String>) {
    if let Some(ref s) = value {
        if let Some(c) = parse_color(s) {
            *target = c;
        }
    }
}

pub fn load_theme() -> Theme {
    let path = theme_path();
    if !path.exists() {
        return Theme::default();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Theme::default(),
    };
    let config: ThemeConfig = match toml::from_str(&content) {
        Ok(c) => c,
        Err(_) => return Theme::default(),
    };

    let mut theme = Theme::default();
    apply(&mut theme.focused_border, &config.board.focused_border);
    apply(&mut theme.unfocused_border, &config.board.unfocused_border);
    apply(&mut theme.cursor, &config.board.cursor);
    apply(&mut theme.selected, &config.board.selected);
    apply(&mut theme.title, &config.board.title);
    apply(&mut theme.priority_high, &config.priority.high);
    apply(&mut theme.priority_medium, &config.priority.medium);
    apply(&mut theme.priority_low, &config.priority.low);
    apply(&mut theme.tag, &config.tags.color);
    apply(&mut theme.due_overdue, &config.due_date.overdue);
    apply(&mut theme.due_today, &config.due_date.today);
    apply(&mut theme.due_soon, &config.due_date.soon);
    apply(&mut theme.due_far, &config.due_date.far);
    apply(&mut theme.modal_border, &config.modal.border);
    apply(&mut theme.modal_focused, &config.modal.focused);
    apply(&mut theme.error, &config.modal.error);
    theme
}

pub fn theme_path() -> std::path::PathBuf {
    let config_dir = dirs::config_dir().expect("Could not determine config directory");
    config_dir.join("rustkanban").join("theme.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_parse_named_colors() {
        assert_eq!(parse_color("Red"), Some(Color::Red));
        assert_eq!(parse_color("cyan"), Some(Color::Cyan));
        assert_eq!(parse_color("YELLOW"), Some(Color::Yellow));
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_color("#FF0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
    }

    #[test]
    fn test_parse_invalid_color() {
        assert_eq!(parse_color("notacolor"), None);
        assert_eq!(parse_color("#GG0000"), None);
        assert_eq!(parse_color(""), None);
    }

    #[test]
    fn test_default_theme() {
        let theme = Theme::default();
        assert_eq!(theme.cursor, Color::Cyan);
        assert_eq!(theme.priority_high, Color::Red);
    }

    #[test]
    fn test_priority_color() {
        let theme = Theme::default();
        assert_eq!(
            theme.priority_color(&crate::model::Priority::High),
            Color::Red
        );
        assert_eq!(
            theme.priority_color(&crate::model::Priority::Low),
            Color::Green
        );
    }
}

pub fn default_theme_toml() -> &'static str {
    r##"# RustKanban Theme Configuration
# Colors: Black, Red, Green, Yellow, Blue, Magenta, Cyan, Gray,
#         DarkGray, LightRed, LightGreen, LightYellow, LightBlue,
#         LightMagenta, LightCyan, White, or hex "#RRGGBB"

[board]
focused_border = "Green"
unfocused_border = "Gray"
cursor = "Cyan"
selected = "Yellow"
title = "White"

[priority]
high = "Red"
medium = "Yellow"
low = "Green"

[tags]
color = "Cyan"

[due_date]
overdue = "Red"
today = "Red"
soon = "Yellow"
far = "Gray"

[modal]
border = "Cyan"
focused = "Yellow"
error = "Red"
"##
}
