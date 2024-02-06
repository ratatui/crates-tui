use std::{
    io::{stdout, Stdout},
    ops::{Deref, DerefMut},
};

use color_eyre::eyre::Result;
use crossterm::{event::*, execute, terminal::*};
use ratatui::prelude::*;

use crate::config;

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    pub fn init() -> Result<Self> {
        let backend = init_backend()?;
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        terminal.hide_cursor()?;
        Ok(Self { terminal })
    }
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

impl Deref for Tui {
    type Target = Terminal<CrosstermBackend<Stdout>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Tui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        restore_backend().unwrap();
    }
}
