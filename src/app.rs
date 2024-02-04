use color_eyre::eyre::{Context, Result};
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use tokio::sync::mpsc;

use crate::{
  action::Action,
  picker::Picker,
  tui::{self, Tui},
};

#[derive(Debug, Default)]
pub struct App {
  pub should_quit: bool,
  pub picker: Picker,
  pub last_tick_key_events: Vec<KeyEvent>,
}

impl App {
  pub fn new() -> Result<Self> {
    let picker = Picker::new();
    Ok(Self { picker, ..Default::default() })
  }

  pub async fn run(&mut self, tui: &mut Tui) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();

    self.picker.register_action_handler(action_tx.clone())?;

    tui.enter()?;

    loop {
      if let Some(e) = tui.next().await {
        match e {
          tui::Event::Quit => action_tx.send(Action::Quit)?,
          tui::Event::Tick => action_tx.send(Action::Tick)?,
          tui::Event::Render => action_tx.send(Action::Render)?,
          tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
          tui::Event::Key(key) => {
            log::debug!("Received key {:?}", key);
            if let Some(action) = self.picker.handle_key_events(key, &self.last_tick_key_events)? {
              action_tx.send(action)?;
            }
            self.last_tick_key_events.push(key);
          },
          _ => {},
        }
      }

      while let Ok(action) = action_rx.try_recv() {
        if action != Action::Tick && action != Action::Render {
          log::debug!("{action:?}");
        }
        if let Some(action) = self.picker.update(action.clone())? {
          action_tx.send(action)?
        };
        match action {
          Action::Tick => {
            self.last_tick_key_events.drain(..);
          },
          Action::Quit => self.should_quit = true,
          Action::Resize(w, h) => {
            tui.resize(Rect::new(0, 0, w, h))?;
            action_tx.send(Action::Render)?;
          },
          Action::Render => {
            tui.draw(|f| {
              let r = self.picker.draw(f, f.size());
              if let Err(e) = r {
                action_tx
                  .send(Action::Error(format!("Failed to draw: {:?}", e)))
                  .with_context(|| "Unable to send error message on action channel")
                  .unwrap();
              }
            })?;
          },
          _ => {},
        }
      }
      if self.should_quit {
        tui.stop()?;
        break;
      }
    }
    tui.exit()?;
    Ok(())
  }
}
