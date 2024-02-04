use color_eyre::eyre::{Context, Result};
use ratatui::{prelude::Rect, widgets::Block};
use tokio::sync::mpsc;

use crate::{
  action::Action,
  root::Root,
  tui::{self, key_event_to_string, Tui},
};

pub async fn run(tui: &mut Tui) -> Result<()> {
  let (tx, mut rx) = mpsc::unbounded_channel();

  let mut root = Root::new(tx.clone());

  let mut should_quit = false;
  let mut last_tick_key_events = vec![];

  tui.enter()?;

  loop {
    if let Some(e) = tui.next().await {
      match e {
        tui::Event::Quit => tx.send(Action::Quit)?,
        tui::Event::Tick => tx.send(Action::Tick)?,
        tui::Event::Render => tx.send(Action::Render)?,
        tui::Event::Resize(x, y) => tx.send(Action::Resize(x, y))?,
        tui::Event::Key(key) => {
          log::debug!("Received key {:?}", key);
          if let Some(action) = root.handle_key_events(key, &last_tick_key_events)? {
            tx.send(action)?;
          }
          last_tick_key_events.push(key);
        },
        _ => {},
      }
    }

    while let Ok(action) = rx.try_recv() {
      if action != Action::Tick && action != Action::Render {
        log::info!("{action:?}");
      }
      if let Some(action) = root.update(action.clone())? {
        tx.send(action)?
      };
      match action {
        Action::Tick => {
          last_tick_key_events.drain(..);
        },
        Action::Quit => should_quit = true,
        Action::Resize(w, h) => {
          tui.resize(Rect::new(0, 0, w, h))?;
          tx.send(Action::Render)?;
        },
        Action::Render => {
          tui.draw(|f| {
            let r = root.draw(f, f.size());
            if let Err(e) = r {
              tx.send(Action::Error(format!("Failed to draw: {:?}", e)))
                .with_context(|| "Unable to send error message on action channel")
                .unwrap();
            }
            f.render_widget(
              Block::default()
                .title(format!("{:?}", last_tick_key_events.iter().map(|k| key_event_to_string(k)).collect::<Vec<_>>()))
                .title_position(ratatui::widgets::block::Position::Bottom)
                .title_alignment(ratatui::layout::Alignment::Right),
              f.size(),
            );
          })?;
        },
        _ => {},
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
