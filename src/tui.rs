use std::io::{stdout, Stdout};

use color_eyre::eyre::Result;
use crossterm::{event::*, execute, terminal::*};
use ratatui::prelude::*;

use crate::config;

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> Result<Tui> {
    let backend = init_backend()?;
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;
    Ok(terminal)
}

fn init_backend() -> Result<CrosstermBackend<Stdout>> {
    let backend = CrosstermBackend::new(stdout());
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    if config::get().enable_mouse {
        execute!(stdout(), EnableMouseCapture)?;
    }
    if config::get().enable_paste {
        execute!(stdout(), EnableBracketedPaste)?;
    }
    Ok(backend)
}

pub fn restore_backend() -> Result<()> {
    if config::get().enable_mouse {
        execute!(stdout(), DisableBracketedPaste)?;
    }
    if config::get().enable_mouse {
        execute!(stdout(), DisableMouseCapture)?;
    }
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
