use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::prelude::*;
use serde::{Deserialize, Serialize};
use strum::Display;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::{debug, info};

use crate::{
    action::Action,
    config,
    root::{Root, RootState},
    tui::{self, Tui},
};

#[derive(Default, Debug, Display, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Search,
    Filter,
    Picker,
    Popup,
}

// FIXME comments on the fields
#[derive(Debug)]
pub struct App {
    tx: UnboundedSender<Action>,
    root_state: RootState,
}

impl App {
    pub fn new(tx: UnboundedSender<Action>) -> Self {
        Self {
            tx: tx.clone(),
            root_state: RootState::new(tx),
        }
    }

    // The main 'run' function now delegates to the two functions above,
    // to handle TUI events and App actions respectively.
    pub async fn run(&mut self, tui: &mut Tui, mut rx: UnboundedReceiver<Action>) -> Result<()> {
        let mut should_quit = false;
        let tx = self.tx.clone();

        tui.enter()?;

        loop {
            if let Some(e) = tui.next().await {
                self.handle_tui_event(e, &tx).await?;
            }
            while let Ok(action) = rx.try_recv() {
                if let Some(inner_action) = self.root_state.update(action.clone())? {
                    tx.send(inner_action)?
                };
                self.handle_action(action.clone(), tui, &tx).await?;
                if action == Action::Quit {
                    should_quit = true;
                    break;
                }
            }
            if should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_tui_event(
        &mut self,
        e: tui::Event,
        tx: &UnboundedSender<Action>,
    ) -> Result<()> {
        match e {
            tui::Event::Quit => tx.send(Action::Quit)?,
            tui::Event::Tick => tx.send(Action::Tick)?,
            tui::Event::KeyRefresh => tx.send(Action::KeyRefresh)?,
            tui::Event::Render => tx.send(Action::Render)?,
            tui::Event::Resize(x, y) => tx.send(Action::Resize(x, y))?,
            tui::Event::Key(key) => {
                debug!("Received key {:?}", key);
                if let Some(action) = self.root_state.handle_key_events(key)? {
                    tx.send(action)?;
                }
                if let Some(action) = self.handle_key_events_from_config(key) {
                    tx.send(action)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_key_events_from_config(&mut self, key: KeyEvent) -> Option<Action> {
        self.root_state.last_tick_key_events.push(key);
        let config = config::get();
        let action = config
            .key_bindings
            .event_to_action(&self.root_state.mode, &self.root_state.last_tick_key_events);
        if action.is_some() {
            self.root_state.last_tick_key_events.drain(..);
        }
        action
    }

    async fn handle_action(
        &mut self,
        action: Action,
        tui: &mut Tui,
        tx: &UnboundedSender<Action>,
    ) -> Result<()> {
        if action != Action::Tick && action != Action::Render {
            info!("{action:?}");
        }
        match action {
            Action::KeyRefresh => {
                self.root_state.last_tick_key_events.drain(..);
            }
            Action::Resize(w, h) => {
                tui.resize(Rect::new(0, 0, w, h))?;
                tx.send(Action::Render)?;
            }
            Action::Render => {
                tui.draw(|frame| {
                    frame.render_stateful_widget(Root, frame.size(), &mut self.root_state);
                    self.update_prompt(frame);
                })?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn update_prompt(&mut self, frame: &mut Frame<'_>) {
        self.root_state.prompt_state.frame_count(frame.count());
        if let Some(cursor_position) = self.root_state.prompt_state.cursor_position() {
            frame.set_cursor(cursor_position.x, cursor_position.y)
        }
    }
}
