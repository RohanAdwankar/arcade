mod app;
mod games;
mod hud;
mod menu;

use std::error::Error;
use std::io;
use std::panic;

use app::App;
use crossterm::ExecutableCommand;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;

fn main() -> Result<(), Box<dyn Error>> {
    install_panic_hook();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = App::default().run(&mut terminal);
    disable_raw_mode()?;
    drop(terminal);
    let mut stdout = io::stdout();
    stdout.execute(LeaveAlternateScreen)?;
    stdout.execute(DisableMouseCapture)?;
    result
}

fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = stdout.execute(LeaveAlternateScreen);
        let _ = stdout.execute(DisableMouseCapture);
        eprintln!("Application crashed: {info}");
    }));
}
