mod app;
mod db;
mod event;
mod export;
mod handler;
mod model;
mod theme;
mod ui;
mod undo;

use std::io::{self, Write};
use std::time::Duration;

use clap::{CommandFactory, Parser, Subcommand};

use app::App;

#[derive(Parser)]
#[command(name = "rk", about = "A terminal kanban board", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Delete all tasks and tags
    Reset,
    /// Generate shell completions
    Completions {
        /// Shell to generate for (bash, zsh, fish, powershell)
        shell: clap_complete::Shell,
    },
    /// Export tasks and tags to JSON
    Export,
    /// Import tasks and tags from a JSON file
    Import {
        /// Path to JSON file
        file: std::path::PathBuf,
    },
    /// Generate man page to stdout
    Manpage,
    /// Print or initialize theme configuration
    Theme {
        /// Create theme file at default config location
        #[arg(long)]
        init: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let db_path = db::db_path();
    let conn = db::init_db(&db_path)?;

    match cli.command {
        Some(Commands::Completions { shell }) => {
            clap_complete::generate(shell, &mut Cli::command(), "rk", &mut io::stdout());
        }
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
        Some(Commands::Manpage) => {
            let cmd = Cli::command();
            let man = clap_mangen::Man::new(cmd);
            man.render(&mut io::stdout())?;
        }
        Some(Commands::Export) => {
            let json = export::export_json(&conn)?;
            println!("{}", json);
        }
        Some(Commands::Import { file }) => {
            let json = std::fs::read_to_string(&file)?;
            let count = export::import_json(&conn, &json)?;
            println!(
                "Imported {} task{}.",
                count,
                if count == 1 { "" } else { "s" }
            );
        }
        Some(Commands::Theme { init }) => {
            if init {
                let path = theme::theme_path();
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, theme::default_theme_toml())?;
                println!("Theme written to {}", path.display());
            } else {
                print!("{}", theme::default_theme_toml());
            }
        }
        None => {
            run_tui(conn)?;
        }
    }

    Ok(())
}

fn restore_terminal() {
    use ratatui::crossterm::event::DisableMouseCapture;
    use ratatui::crossterm::execute;
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
}

fn run_tui(conn: rusqlite::Connection) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();

    use ratatui::crossterm::event::EnableMouseCapture;
    use ratatui::crossterm::execute;
    execute!(std::io::stdout(), EnableMouseCapture)?;

    // Install panic hook that restores terminal before printing panic info
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    let t = theme::load_theme();
    let mut app = App::new(conn, t);

    loop {
        let size = terminal.size()?;

        app.terminal_width = size.width;
        app.terminal_height = size.height;

        // Update modal wrap width from terminal size for cursor up/down
        let modal_inner = (size.width as f32 * 0.6) as usize;
        app.modal.wrap_width = modal_inner.saturating_sub(4).max(1);

        // Update column scroll offsets
        // Column inner width = (terminal_width / 3) - 2 for borders
        let col_width = (size.width / 3).saturating_sub(2) as usize;
        // Board height = terminal_height - status_bar(1) - optional_bars - column_borders(2)
        let extra_bars = (app.search_active || app.mode == app::AppMode::SearchFilter) as u16;
        let col_height = size.height.saturating_sub(1 + extra_bars + 2) as usize;
        app.update_scroll(col_width, col_height);

        terminal.draw(|frame| ui::render(frame, &app))?;

        if let Some(ev) = event::poll_event(Duration::from_millis(100))? {
            match ev {
                event::AppEvent::Key(key) => handler::handle_event(&mut app, key),
                event::AppEvent::Mouse(mouse) => handler::handle_mouse(&mut app, mouse),
            }
        }

        app.tick();

        if !app.running {
            break;
        }
    }

    restore_terminal();
    Ok(())
}
