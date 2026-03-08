use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};

pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

pub fn poll_event(timeout: Duration) -> std::io::Result<Option<AppEvent>> {
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                return Ok(Some(AppEvent::Key(key)));
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(_)
                | MouseEventKind::Up(_)
                | MouseEventKind::ScrollDown
                | MouseEventKind::ScrollUp => {
                    return Ok(Some(AppEvent::Mouse(mouse)));
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(None)
}
