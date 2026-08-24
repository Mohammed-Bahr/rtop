mod action;
mod app;
mod config;
mod event;
mod system;
mod ui;
mod utils;

use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event as ct_event;
use crossterm::event::Event;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

/// Command line interface.
#[derive(Parser, Debug)]
#[command(
    name = "rtop",
    version,
    about = "A modern Linux terminal system monitor",
    after_help = "Press ? inside the TUI for all keyboard shortcuts."
)]
struct Cli {
    /// Refresh interval in milliseconds (overrides config).
    #[arg(short, long, value_name = "MS")]
    interval: Option<u64>,
    /// Path to a TOML config file.
    #[arg(long, value_name = "FILE")]
    config: Option<std::path::PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let (mut cfg, warning) = match &cli.config {
        Some(path) => match config::Config::load(path) {
            Ok(cfg) => (cfg.sanitized(), None),
            Err(e) => (
                config::Config::default().sanitized(),
                Some(format!("Ignoring invalid config: {e}")),
            ),
        },
        None => config::Config::load_default(),
    };
    if let Some(ms) = cli.interval {
        cfg.refresh_ms = ms.clamp(100, 60_000);
    }

    if let Some(w) = &warning {
        eprintln!("{w}");
    }

    let mut terminal = match setup_terminal() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to initialise terminal: {e}");
            return;
        }
    };

    let result = run_app(&mut terminal, cfg);

    if let Err(e) = restore_terminal(&mut terminal) {
        eprintln!("Failed to restore terminal: {e}");
    }

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    cfg: config::Config,
) -> io::Result<()> {
    let mut app = app::App::new(cfg);
    let tick = Duration::from_millis(app.config.refresh_ms);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Wait for input or the next tick, whichever comes first.
        let timeout = tick.saturating_sub(last_tick.elapsed());
        if ct_event::poll(timeout)? {
            // Drain every pending event so we stay responsive under load.
            match ct_event::read()? {
                Event::Key(key) => app.on_key(key),
                Event::Resize(_, _) => { /* next draw repaints */ }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick {
            last_tick = Instant::now();
            app.tick();
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
