use std::io::{stdout, Stdout};

use color_eyre::eyre::Result;
use crossterm::{event::*, execute, terminal::*};
use ratatui::prelude::*;

use crate::config;

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> Result<Tui> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    if config::get().enable_mouse {
        execute!(stdout(), EnableMouseCapture)?;
    }
    if config::get().enable_paste {
        execute!(stdout(), EnableBracketedPaste)?;
    }
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    terminal.hide_cursor()?;
    Ok(terminal)
}

pub fn restore() -> Result<()> {
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
