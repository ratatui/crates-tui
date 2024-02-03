use std::{cell::RefCell, rc::Rc};

use color_eyre::eyre::{Context, Result};
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
  action::Action,
  config::Config,
  mode::Mode,
  picker::Picker,
  tui::{self, Tui},
};

pub struct App {
  pub should_quit: bool,
  pub mode: Rc<RefCell<Mode>>,
  pub picker: Picker,
}

impl App {
  pub fn new() -> Result<Self> {
    let mode = Rc::new(RefCell::new(Mode::PickerSearchQueryEditing));
    let picker = Picker::new(mode.clone());
    Ok(Self { should_quit: Default::default(), mode, picker })
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
            if let Some(action) = self.picker.handle_events(e.clone())? {
              action_tx.send(action)?;
            }
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
          Action::Tick => {},
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
