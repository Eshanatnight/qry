mod app;
mod db;
mod event;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use std::io::stdout;

#[derive(Parser)]
#[command(name = "qry", about = "TUI database explorer for SQLite and DuckDB")]
struct Cli {
    /// Path to the database file (.db, .sqlite, .sqlite3, .duckdb, .duck, .ddb)
    path: String,

    /// Open database in read-write mode (DuckDB only, default is read-only)
    #[arg(long)]
    read_write: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let conn = db::Connection::open(&cli.path, cli.read_write)?;
    let mut app = app::App::new(conn, cli.path)?;

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let _ = stdout().execute(EnableMouseCapture);
    let kb_enhanced = stdout()
        .execute(PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
        ))
        .is_ok();
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app);

    app.save_history_file();

    if kb_enhanced {
        let _ = stdout().execute(PopKeyboardEnhancementFlags);
    }
    let _ = stdout().execute(DisableMouseCapture);
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut app::App,
) -> Result<()> {
    while app.running {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Some(ev) = event::poll_event()? {
            match ev {
                Event::Key(key) => {
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        event::handle_key(app, key);
                    }
                }
                Event::Mouse(mouse) => {
                    event::handle_mouse(app, mouse.kind, mouse.column, mouse.row);
                }
                _ => {}
            }
        }
    }
    Ok(())
}
