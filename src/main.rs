mod app;
mod db;
mod event;
mod handler;
mod model;
mod undo;
mod ui;

use std::io::{self, Write};
use std::time::Duration;

use clap::{Parser, Subcommand};

use app::App;

#[derive(Parser)]
#[command(name = "rk", about = "A terminal kanban board")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Delete all tasks and tags
    Reset,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let db_path = db::db_path();
    let conn = db::init_db(&db_path)?;

    match cli.command {
        Some(Commands::Reset) => {
            print!("Delete all tasks and tags? (Y/N) ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if input.trim().eq_ignore_ascii_case("y") {
                db::reset_db(&conn)?;
                println!("All data cleared.");
            } else {
                println!("Aborted.");
            }
        }
        None => {
            run_tui(conn)?;
        }
    }

    Ok(())
}

fn run_tui(conn: rusqlite::Connection) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    let mut app = App::new(conn);

    loop {
        let size = terminal.size()?;

        // Update modal wrap width from terminal size for cursor up/down
        let modal_inner = (size.width as f32 * 0.6) as usize;
        app.modal.wrap_width = modal_inner.saturating_sub(4).max(1);

        // Update column scroll offsets
        // Column inner width = (terminal_width / 3) - 2 for borders
        let col_width = (size.width / 3).saturating_sub(2) as usize;
        // Board height = terminal_height - status_bar(1) - optional_bars - column_borders(2)
        let extra_bars =
            (app.search_active || app.mode == app::AppMode::SearchFilter) as u16;
        let col_height = size.height.saturating_sub(1 + extra_bars + 2) as usize;
        app.update_scroll(col_width, col_height);

        terminal.draw(|frame| ui::render(frame, &app))?;

        if let Some(key) = event::poll_event(Duration::from_millis(100))? {
            handler::handle_event(&mut app, key);
        }

        app.tick();

        if !app.running {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}
