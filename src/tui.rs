use std::{
    io::{stdout, Stdout},
    ops::{Deref, DerefMut},
    pin::Pin,
    time::Duration,
};

use color_eyre::eyre::Result;
use crossterm::{
    event::{Event as CrosstermEvent, *},
    execute,
    terminal::*,
};
use futures::{Stream, StreamExt};
use ratatui::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::time::interval;
use tokio_stream::{wrappers::IntervalStream, StreamMap};

use crate::config;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    Init,
    Quit,
    Error,
    Closed,
    Tick,
    KeyRefresh,
    Render,
    FocusGained,
    FocusLost,
    Paste(String),
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    streams: StreamMap<String, Pin<Box<dyn Stream<Item = Event>>>>,
}

impl Tui {
    pub fn init() -> Result<Self> {
        let backend = init_backend()?;
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        terminal.hide_cursor()?;
        let streams = create_streams();
        Ok(Self { terminal, streams })
    }

    pub async fn next(&mut self) -> Option<Event> {
        match self.streams.next().await {
            Some((_name, event)) => Some(event),
            None => None,
        }
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

fn create_streams() -> StreamMap<String, Pin<Box<dyn Stream<Item = Event>>>> {
    StreamMap::from_iter([
        ("ticks".to_string(), tick_stream()),
        ("key_refresh".to_string(), key_refresh_stream()),
        ("render".to_string(), render_stream()),
        ("crossterm".to_string(), crossterm_stream()),
    ])
}

fn tick_stream() -> Pin<Box<dyn Stream<Item = Event>>> {
    let tick_delay = Duration::from_secs_f64(1.0 / config::get().tick_rate);
    let tick_interval = interval(tick_delay);
    Box::pin(IntervalStream::new(tick_interval).map(|_| Event::Tick))
}

fn key_refresh_stream() -> Pin<Box<dyn Stream<Item = Event>>> {
    let key_refresh_delay = Duration::from_secs_f64(1.0 / config::get().key_refresh_rate);
    let key_refresh_interval = interval(key_refresh_delay);
    Box::pin(IntervalStream::new(key_refresh_interval).map(|_| Event::KeyRefresh))
}

fn render_stream() -> Pin<Box<dyn Stream<Item = Event>>> {
    let render_delay = Duration::from_secs_f64(1.0 / config::get().frame_rate);
    let render_interval = interval(render_delay);
    Box::pin(IntervalStream::new(render_interval).map(|_| Event::Render))
}

fn crossterm_stream() -> Pin<Box<dyn Stream<Item = Event>>> {
    Box::pin(EventStream::new().fuse().filter_map(|event| async move {
        match event {
            Ok(event) => match event {
                CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                    Some(Event::Key(key))
                }
                CrosstermEvent::Mouse(mouse) => Some(Event::Mouse(mouse)),
                CrosstermEvent::Resize(x, y) => Some(Event::Resize(x, y)),
                CrosstermEvent::FocusLost => Some(Event::FocusLost),
                CrosstermEvent::FocusGained => Some(Event::FocusGained),
                CrosstermEvent::Paste(s) => Some(Event::Paste(s)),
                _ => None,
            },
            Err(_) => Some(Event::Error),
        }
    }))
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
