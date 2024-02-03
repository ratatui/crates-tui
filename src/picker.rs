use std::{
  cell::RefCell,
  collections::HashMap,
  rc::Rc,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
  },
  time::Duration,
};

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use num_format::{Locale, ToFormattedString};
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tui_input::backend::crossterm::EventHandler;

use crate::{action::Action, color, config::Config, mode::Mode, tui::Event};

#[derive(Default)]
pub struct Picker {
  command_tx: Option<UnboundedSender<Action>>,
  action_tx: Option<UnboundedSender<Action>>,
  config: Config,
  mode: Rc<RefCell<Mode>>,
  last_events: Vec<KeyEvent>,
  loading_status: Arc<AtomicBool>,
  search: String,
  search_horizontal_scroll: usize,
  filter: String,
  filter_horizontal_scroll: usize,
  filtered_crates: Vec<crates_io_api::Crate>,
  crates: Arc<Mutex<Vec<crates_io_api::Crate>>>,
  crate_info: Arc<Mutex<Option<crates_io_api::Crate>>>,
  state: TableState,
  input: tui_input::Input,
  page: u64,
  row_height: usize,
}

impl Picker {
  pub fn new(mode: Rc<RefCell<Mode>>) -> Self {
    Self { page: 1, row_height: 1, mode, ..Self::default() }
  }

  pub fn render_info_widget(&mut self, f: &mut Frame, area: Rect) {
    let crate_info = self.crate_info.lock().unwrap().clone();
    let crate_info = if let Some(ci) = crate_info {
      ci
    } else {
      f.render_widget(Block::default().borders(Borders::ALL).title("crates.io info"), area);
      return;
    };
    let name = crate_info.name.clone();

    let mut rows = vec![];

    rows.push(Row::new(vec![Cell::from("Name"), Cell::from(name.clone())]));
    if let Some(description) = crate_info.description {
      rows.push(Row::new(vec![Cell::from("Description"), Cell::from(description)]));
    }
    if let Some(homepage) = crate_info.homepage {
      rows.push(Row::new(vec![Cell::from("Homepage"), Cell::from(homepage)]));
    }
    if let Some(repository) = crate_info.repository {
      rows.push(Row::new(vec![Cell::from("Repository"), Cell::from(repository)]));
    }
    if let Some(recent_downloads) = crate_info.recent_downloads {
      rows.push(Row::new(vec![Cell::from("Recent Downloads"), Cell::from(recent_downloads.to_string())]));
    }
    rows.push(Row::new(vec![Cell::from("Max Version"), Cell::from(crate_info.max_version)]));
    if let Some(max_stable_version) = crate_info.max_stable_version {
      rows.push(Row::new(vec![Cell::from("Max Stable Version"), Cell::from(max_stable_version)]));
    }
    rows.push(Row::new(vec![
      Cell::from("Created At"),
      Cell::from(crate_info.created_at.format("%Y-%m-%d %H:%M:%S").to_string()),
    ]));
    rows.push(Row::new(vec![
      Cell::from("Updated At"),
      Cell::from(crate_info.created_at.format("%Y-%m-%d %H:%M:%S").to_string()),
    ]));

    let widths = [Constraint::Min(20), Constraint::Percentage(100)];
    let table_widget =
      Table::new(rows, widths).block(Block::default().borders(Borders::ALL).title(format!("crates.io info - {name}")));
    f.render_widget(table_widget, area);
  }

  pub fn render_table_widget(&mut self, f: &mut Frame, area: Rect) {
    let selected_style = Style::default();
    let normal_style = Style::default().bg(Color::White).fg(Color::Black);
    let ncrates = self.filtered_crates.len();
    let header = Row::new(
      ["Name", "Description", "Downloads", "Last Updated"]
        .iter()
        .map(|h| Text::from(vec![Line::from(""), Line::from(h.bold()), Line::from("")])),
    )
    .bg(color::GRAY_900)
    .height(3);
    let highlight_symbol = if *self.mode.borrow() == Mode::Picker { " \u{2022} " } else { "   " };
    let loading_status = if self.loading_status.load(Ordering::SeqCst) {
      format!("Loaded {}...", ncrates)
    } else {
      format!("{}/{}", self.state.selected().map_or(0, |n| n + 1), ncrates)
    };

    let crates = self.filtered_crates.clone();
    let rows = crates.iter().enumerate().map(|(i, item)| {
      Row::new([
        Text::from(vec![Line::from(""), Line::from(item.name.clone()), Line::from("")]),
        Text::from(vec![Line::from(""), Line::from(item.description.clone().unwrap_or_default()), Line::from("")]),
        Text::from(vec![Line::from(""), Line::from(item.downloads.to_formatted_string(&Locale::en)), Line::from("")]),
        Text::from(vec![
          Line::from(""),
          Line::from(item.updated_at.format("%Y-%m-%d %H:%M:%S").to_string()),
          Line::from(""),
        ]),
      ])
      .bg(match i % 2 {
        0 => color::GRAY_900,
        1 => color::GRAY_800,
        _ => unreachable!("Cannot reach this line"),
      })
      .height(3)
    });
    let widths = [Constraint::Min(20), Constraint::Percentage(100), Constraint::Min(10), Constraint::Min(15)];
    let table_widget = Table::new(rows, widths)
      .header(header)
      .column_spacing(1)
      .highlight_style(selected_style)
      .highlight_symbol(Text::from(vec!["".into(), " █ ".into(), "".into()]))
      .highlight_spacing(HighlightSpacing::Always);
    f.render_stateful_widget(table_widget, area, &mut self.state);
  }

  pub fn next(&mut self) {
    if self.filtered_crates.len() == 0 {
      self.state.select(None)
    } else {
      let i = match self.state.selected() {
        Some(i) => {
          if i >= self.filtered_crates.len() - 1 {
            self.row_height / 2
          } else {
            i + self.row_height
          }
        },
        None => self.row_height / 2,
      };
      self.state.select(Some(i));
    }
  }

  pub fn previous(&mut self) {
    if self.filtered_crates.len() == 0 {
      self.state.select(None)
    } else {
      let i = match self.state.selected() {
        Some(i) => {
          if i == (self.row_height / 2) {
            self.filtered_crates.len() - 1
          } else {
            i - self.row_height
          }
        },
        None => 0,
      };
      self.state.select(Some(i));
    }
  }

  pub fn top(&mut self) {
    if self.filtered_crates.len() == 0 {
      self.state.select(None)
    } else {
      self.state.select(Some(0))
    }
  }

  pub fn bottom(&mut self) {
    if self.filtered_crates.len() == 0 {
      self.state.select(None)
    } else {
      self.state.select(Some(self.filtered_crates.len() - 1));
    }
  }

  fn render_filter_widget(&mut self, f: &mut Frame, area: Rect) {
    let scroll = if *self.mode.borrow() == Mode::PickerSearchQueryEditing {
      self.search_horizontal_scroll
    } else if *self.mode.borrow() == Mode::PickerFilterEditing {
      self.filter_horizontal_scroll
    } else {
      0
    };

    let block = Block::default()
      .borders(Borders::ALL)
      .title(
        block::Title::from(Line::from(vec![
          "Query ".into(),
          "(Press ".into(),
          "?".bold(),
          " to search, ".into(),
          "/".bold(),
          " to filter, ".into(),
          "Enter".bold(),
          " to submit)".into(),
        ]))
        .alignment(Alignment::Left),
      )
      .border_style(match *self.mode.borrow() {
        Mode::PickerSearchQueryEditing => Style::default().fg(color::GREEN_400),
        Mode::PickerFilterEditing => Style::default().fg(color::RED_400),
        _ => Style::default().add_modifier(Modifier::DIM),
      });
    f.render_widget(block, area);

    let paragraph = Paragraph::new(self.input.value()).scroll((0, scroll as u16));
    f.render_widget(paragraph, area.inner(&Margin { horizontal: 2, vertical: 2 }));
  }

  fn mode_mut(&mut self) -> std::cell::RefMut<'_, Mode> {
    std::cell::RefCell::<_>::borrow_mut(&self.mode)
  }

  fn reload_data(&mut self) {
    self.state.select(None);
    *self.crate_info.lock().unwrap() = None;
    let crates = self.crates.clone();
    let search = self.search.clone();
    let loading_status = self.loading_status.clone();
    let action_tx = self.action_tx.clone();
    let page = self.page.clamp(1, u64::MAX);
    tokio::spawn(async move {
      crates.lock().unwrap().drain(0..);
      loading_status.store(true, Ordering::SeqCst);
      let client =
        crates_io_api::AsyncClient::new("crates-tui (crates-tui@kdheepak.com)", std::time::Duration::from_millis(1000))
          .unwrap();
      let mut query = crates_io_api::CratesQueryBuilder::default();
      query = query.search(search);
      query = query.page(page);
      query = query.page_size(100);
      query = query.sort(crates_io_api::Sort::Relevance);
      let query = query.build();
      let page = client.crates(query).await.unwrap();
      let mut all_crates = vec![];
      for _crate in page.crates.iter() {
        all_crates.push(_crate.clone())
      }
      all_crates.sort_by(|a, b| b.downloads.cmp(&a.downloads));
      *crates.lock().unwrap() = all_crates;
      if let Some(action_tx) = action_tx {
        action_tx.send(Action::Tick).unwrap_or_default();
        action_tx.send(Action::MoveSelectionNext).unwrap_or_default();
      }
      loading_status.store(false, Ordering::SeqCst);
    });
  }

  fn get_info(&mut self) {
    let name = if let Some(index) = self.state.selected() {
      if self.filtered_crates.len() > 0 {
        self.filtered_crates[index].name.clone()
      } else {
        return;
      }
    } else if self.filtered_crates.len() > 0 {
      self.state.select(Some(0));
      self.filtered_crates[0].name.clone()
    } else {
      return;
    };
    if !name.is_empty() {
      let crate_info = self.crate_info.clone();
      tokio::spawn(async move {
        let client = crates_io_api::AsyncClient::new(
          "crates-tui (crates-tui@kdheepak.com)",
          std::time::Duration::from_millis(1000),
        )
        .unwrap();
        match client.get_crate(&name).await {
          Ok(_crate_info) => *crate_info.lock().unwrap() = Some(_crate_info.crate_data),
          Err(err) => {},
        }
      });
    }
  }

  fn tick(&mut self) {
    self.last_events.drain(..);
    let filter = self.filter.clone();
    let filter_words = filter.split_whitespace().collect::<Vec<_>>();
    self.filtered_crates = self
      .crates
      .lock()
      .unwrap()
      .iter()
      .filter(|c| {
        filter_words.iter().all(|word| {
          c.name.to_lowercase().contains(word)
            || c.description.clone().unwrap_or_default().to_lowercase().contains(word)
        })
      })
      .map(|c| c.clone())
      .collect();
  }
}

impl Picker {
  pub fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
    f.render_widget(Block::default().bg(color::GRAY_900), area);

    let [table_rect, filter_rect] = Layout::default()
      .constraints([Constraint::Percentage(100), Constraint::Min(5)])
      .split(area)
      .to_vec()
      .try_into()
      .unwrap();

    self.render_table_widget(f, table_rect);
    self.render_filter_widget(f, filter_rect);

    if *self.mode.borrow() == Mode::PickerSearchQueryEditing || *self.mode.borrow() == Mode::PickerFilterEditing {
      f.set_cursor(
        (filter_rect.x + 2 + self.input.cursor() as u16).min(filter_rect.x + filter_rect.width - 2),
        filter_rect.y + 2,
      )
    }

    Ok(())
  }

  pub fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
    self.action_tx = Some(tx);
    Ok(())
  }

  pub fn register_config_handler(&mut self, config: Config) -> Result<()> {
    self.config = config;
    Ok(())
  }

  pub fn update(&mut self, action: Action) -> Result<Option<Action>> {
    match action {
      Action::Tick => {
        self.tick();
      },
      Action::MoveSelectionNext => {
        self.next();
        return Ok(Some(Action::GetInfo));
      },
      Action::MoveSelectionPrevious => {
        self.previous();
        return Ok(Some(Action::GetInfo));
      },
      Action::MoveSelectionTop => {
        self.top();
        return Ok(Some(Action::GetInfo));
      },
      Action::MoveSelectionBottom => {
        self.bottom();
        return Ok(Some(Action::GetInfo));
      },
      Action::EnterSearchQueryInsert => {
        *self.mode_mut() = Mode::PickerSearchQueryEditing;
        self.input = self.input.clone().with_value(self.search.clone());
      },
      Action::EnterFilterInsert => {
        *self.mode_mut() = Mode::PickerFilterEditing;
        self.input = self.input.clone().with_value(self.filter.clone());
      },
      Action::EnterNormal => {
        *self.mode_mut() = Mode::Picker;
        if self.filtered_crates.len() > 0 && self.state.selected().is_none() {
          self.state.select(Some(0))
        }
      },
      Action::SubmitSearchQuery => {
        *self.mode_mut() = Mode::Picker;
        self.filter.clear();
        return Ok(Some(Action::ReloadData));
      },
      Action::ReloadData => {
        self.reload_data();
      },
      Action::GetInfo => {
        self.get_info();
      },
      _ => {},
    }
    Ok(None)
  }

  pub fn handle_events(&mut self, evt: Event) -> Result<Option<Action>> {
    if let Event::Key(key) = evt {
      return self.handle_key_events(key);
    }
    Ok(None)
  }

  pub fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
    let cmd = match *self.mode.borrow() {
      Mode::Picker => {
        match key.code {
          KeyCode::Char('q') => Action::Quit,
          KeyCode::Char('?') => Action::EnterSearchQueryInsert,
          KeyCode::Char('/') => Action::EnterFilterInsert,
          KeyCode::Char('j') | KeyCode::Down => Action::MoveSelectionNext,
          KeyCode::Char('k') | KeyCode::Up => Action::MoveSelectionPrevious,
          KeyCode::Char('g') => {
            if let Some(KeyEvent { code: KeyCode::Char('g'), .. }) = self.last_events.last() {
              Action::MoveSelectionTop
            } else {
              self.last_events.push(key.clone());
              return Ok(None);
            }
          },
          KeyCode::PageUp => Action::MoveSelectionTop,
          KeyCode::Char('G') | KeyCode::PageDown => Action::MoveSelectionBottom,
          KeyCode::Char('r') => Action::ReloadData,
          KeyCode::Home => Action::MoveSelectionTop,
          KeyCode::End => Action::MoveSelectionBottom,
          KeyCode::Esc => Action::Quit,
          _ => return Ok(None),
        }
      },
      Mode::PickerSearchQueryEditing => {
        match key.code {
          KeyCode::Esc => Action::EnterNormal,
          KeyCode::Enter => Action::SubmitSearchQuery,
          _ => {
            self.input.handle_event(&crossterm::event::Event::Key(key));
            self.search = self.input.value().into();
            return Ok(None);
          },
        }
      },
      Mode::PickerFilterEditing => {
        match key.code {
          KeyCode::Esc => Action::EnterNormal,
          KeyCode::Enter => Action::EnterNormal,
          _ => {
            self.input.handle_event(&crossterm::event::Event::Key(key));
            self.filter = self.input.value().into();
            self.state.select(None);
            Action::GetInfo
          },
        }
      },
      _ => return Ok(None),
    };
    Ok(Some(cmd))
  }
}
