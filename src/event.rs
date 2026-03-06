use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyEvent, KeyEventKind};

pub fn poll_event(timeout: Duration) -> std::io::Result<Option<KeyEvent>> {
    if event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                return Ok(Some(key));
            }
        }
    }
    Ok(None)
}
