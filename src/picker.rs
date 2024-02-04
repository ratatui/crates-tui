use std::{
  cell::RefCell,
  rc::Rc,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
  },
};

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use itertools::Itertools;
use num_format::{Locale, ToFormattedString};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tui_input::backend::crossterm::EventHandler;

use crate::{action::Action, config, mode::Mode};

#[derive(Debug, Default)]
pub struct Picker {
  action_tx: Option<UnboundedSender<Action>>,
  mode: Rc<RefCell<Mode>>,
  last_events: Vec<KeyEvent>,
  loading_status: Arc<AtomicBool>,
  search: String,
  search_horizontal_scroll: usize,
  filter: String,
  filter_horizontal_scroll: usize,
  filtered_crates: Vec<crates_io_api::Crate>,
  crates: Arc<Mutex<Vec<crates_io_api::Crate>>>,
  total_num_crates: Option<u64>,
  crate_info: Arc<Mutex<Option<crates_io_api::Crate>>>,
  state: TableState,
  scrollbar_state: ScrollbarState,
  input: tui_input::Input,
  page: u64,
  page_size: u64,
  show_crate_info: bool,
  row_height: usize,
}

impl Picker {
  pub fn new(mode: Rc<RefCell<Mode>>) -> Self {
    Self { page: 1, row_height: 1, page_size: 25, mode, ..Self::default() }
  }

  pub fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
    self.action_tx = Some(tx);
    Ok(())
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
      self.scrollbar_state = self.scrollbar_state.position(i);
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
      self.scrollbar_state = self.scrollbar_state.position(i);
    }
  }

  pub fn top(&mut self) {
    if self.filtered_crates.len() == 0 {
      self.state.select(None)
    } else {
      self.state.select(Some(0));
      self.scrollbar_state = self.scrollbar_state.position(0);
    }
  }

  pub fn bottom(&mut self) {
    if self.filtered_crates.len() == 0 {
      self.state.select(None)
    } else {
      self.state.select(Some(self.filtered_crates.len() - 1));
      self.scrollbar_state = self.scrollbar_state.position(self.filtered_crates.len() - 1);
    }
  }

  fn mode_mut(&mut self) -> std::cell::RefMut<'_, Mode> {
    std::cell::RefCell::<_>::borrow_mut(&self.mode)
  }

  fn increment_page(&mut self) {
    if let Some(n) = self.total_num_crates {
      let max_page_size = (n / self.page_size) + 1;
      if self.page < max_page_size {
        self.page = self.page.saturating_add(1).min(max_page_size);
        self.reload_data();
      }
    }
  }

  fn decrement_page(&mut self) {
    let min_page_size = 1;
    if self.page > min_page_size {
      self.page = self.page.saturating_sub(1).max(min_page_size);
      self.reload_data();
    }
  }

  fn reload_data(&mut self) {
    self.state.select(None);
    *self.crate_info.lock().unwrap() = None;
    let crates = self.crates.clone();
    let search = self.search.clone();
    let loading_status = self.loading_status.clone();
    let action_tx = self.action_tx.clone();
    let page = self.page.clamp(1, u64::MAX);
    let page_size = self.page_size;
    tokio::spawn(async move {
      loading_status.store(true, Ordering::SeqCst);
      let client =
        crates_io_api::AsyncClient::new("crates-tui (crates-tui@kdheepak.com)", std::time::Duration::from_millis(1000))
          .unwrap();
      let mut query = crates_io_api::CratesQueryBuilder::default();
      query = query.search(search);
      query = query.page(page);
      query = query.page_size(page_size);
      query = query.sort(crates_io_api::Sort::Relevance);
      let query = query.build();
      if let Ok(page) = client.crates(query).await {
        let mut all_crates = vec![];
        for _crate in page.crates.iter() {
          all_crates.push(_crate.clone())
        }
        all_crates.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        crates.lock().unwrap().drain(0..);
        *crates.lock().unwrap() = all_crates;
        if let Some(tx) = action_tx {
          if crates.lock().unwrap().len() > 0 {
            tx.send(Action::StoreTotalNumberOfCrates(page.meta.total)).unwrap_or_default();
            tx.send(Action::Tick).unwrap_or_default();
            tx.send(Action::MoveSelectionNext).unwrap_or_default();
          } else {
            tx.send(Action::Error("Something went wrong".into())).unwrap_or_default();
          }
        }
        loading_status.store(false, Ordering::SeqCst);
      } else {
        if let Some(tx) = action_tx {
          tx.send(Action::Error("Something went wrong".into())).unwrap_or_default();
        }
        loading_status.store(false, Ordering::SeqCst);
      }
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
    let action_tx = self.action_tx.clone();
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
          Err(_err) => {
            if let Some(tx) = action_tx {
              tx.send(Action::Error("Unable to get crate information".into())).unwrap_or_default()
            }
          },
        }
      });
    }
  }

  fn tick(&mut self) {
    self.last_events.drain(..);
    self.update_filtered_crates();
    self.update_scrollbar_state();
  }

  fn update_scrollbar_state(&mut self) {
    self.scrollbar_state = self.scrollbar_state.content_length(self.filtered_crates.len());
  }

  fn update_filtered_crates(&mut self) {
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
  pub fn update(&mut self, action: Action) -> Result<Option<Action>> {
    match action {
      Action::Tick => {
        self.tick();
      },
      Action::StoreTotalNumberOfCrates(n) => {
        self.total_num_crates = Some(n);
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
      Action::IncrementPage => {
        self.increment_page();
      },
      Action::DecrementPage => {
        self.decrement_page();
      },
      Action::ToggleShowCrateInfo => {
        self.show_crate_info = !self.show_crate_info;
      },
      Action::GetInfo => {
        self.get_info();
      },
      _ => {},
    }
    Ok(None)
  }

  pub fn handle_key_events(&mut self, key: KeyEvent, last_key_events: &[KeyEvent]) -> Result<Option<Action>> {
    let cmd = match *self.mode.borrow() {
      Mode::Picker => {
        match key.code {
          KeyCode::Char('q') => Action::Quit,
          KeyCode::Char('?') => Action::EnterSearchQueryInsert,
          KeyCode::Char('/') => Action::EnterFilterInsert,
          KeyCode::Char('j') | KeyCode::Down => Action::MoveSelectionNext,
          KeyCode::Char('k') | KeyCode::Up => Action::MoveSelectionPrevious,
          KeyCode::Char('l') | KeyCode::Right => Action::IncrementPage,
          KeyCode::Char('h') | KeyCode::Left => Action::DecrementPage,
          KeyCode::Char('g') => {
            if let Some(KeyEvent { code: KeyCode::Char('g'), .. }) = last_key_events.last() {
              Action::MoveSelectionTop
            } else {
              return Ok(None);
            }
          },
          KeyCode::PageUp => Action::MoveSelectionTop,
          KeyCode::Char('G') | KeyCode::PageDown => Action::MoveSelectionBottom,
          KeyCode::Char('r') => Action::ReloadData,
          KeyCode::Home => Action::MoveSelectionTop,
          KeyCode::End => Action::MoveSelectionBottom,
          KeyCode::Esc => Action::Quit,
          KeyCode::Enter => Action::ToggleShowCrateInfo,
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

impl Picker {
  pub fn render_crate_info(&mut self, f: &mut Frame, area: Rect) {
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
    rows.push(Row::new(vec![
      Cell::from("Created At"),
      Cell::from(crate_info.created_at.format("%Y-%m-%d %H:%M:%S").to_string()),
    ]));
    rows.push(Row::new(vec![
      Cell::from("Updated At"),
      Cell::from(crate_info.created_at.format("%Y-%m-%d %H:%M:%S").to_string()),
    ]));
    rows.push(Row::new(vec![Cell::from("Max Version"), Cell::from(crate_info.max_version)]));
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
    if let Some(max_stable_version) = crate_info.max_stable_version {
      rows.push(Row::new(vec![Cell::from("Max Stable Version"), Cell::from(max_stable_version)]));
    }

    let widths = [Constraint::Fill(1), Constraint::Fill(4)];
    let table_widget = Table::new(rows, widths).block(Block::default().borders(Borders::ALL));
    f.render_widget(table_widget, area);
  }

  pub fn render_table(&mut self, f: &mut Frame, area: Rect) {
    let selected_style = Style::default();
    let header = Row::new(
      ["Name", "Description", "Downloads", "Last Updated"]
        .iter()
        .map(|h| Text::from(vec![Line::from(""), Line::from(h.bold()), Line::from("")])),
    )
    .bg(config::get().background_color)
    .height(3);
    let highlight_symbol = if *self.mode.borrow() == Mode::Picker { " \u{2022} " } else { "   " };

    let widths =
      [Constraint::Length(1), Constraint::Max(20), Constraint::Min(0), Constraint::Max(10), Constraint::Max(20)];

    let (areas, spacers) =
      Layout::horizontal(&widths).spacing(1).split_with_spacers(area.inner(&Margin { horizontal: 1, vertical: 0 }));
    let size = areas[2];

    let crates = self.filtered_crates.clone();
    let rows = crates.iter().enumerate().map(|(i, item)| {
      let desc = item.description.clone().unwrap_or_default();
      let mut desc = textwrap::wrap(&desc, size.width as usize).iter().map(|s| Line::from(s.to_string())).collect_vec();
      desc.insert(0, Line::from(""));
      let height = desc.len();
      Row::new([
        Text::from(vec![Line::from(""), Line::from(item.name.clone()), Line::from("")]),
        Text::from(desc),
        Text::from(vec![Line::from(""), Line::from(item.downloads.to_formatted_string(&Locale::en)), Line::from("")]),
        Text::from(vec![
          Line::from(""),
          Line::from(item.updated_at.format("%Y-%m-%d %H:%M:%S").to_string()),
          Line::from(""),
        ]),
      ])
      .bg(match i % 2 {
        0 => config::get().row_background_color_1,
        1 => config::get().row_background_color_2,
        _ => unreachable!("Cannot reach this line"),
      })
      .height(height.saturating_add(1) as u16)
    });

    let widths = [Constraint::Max(20), Constraint::Min(0), Constraint::Max(10), Constraint::Max(20)];
    let table_widget = Table::new(rows, widths)
      .header(header)
      .column_spacing(1)
      .highlight_style(selected_style)
      .highlight_symbol(Text::from(vec!["".into(), highlight_symbol.into(), "".into()]))
      .highlight_spacing(HighlightSpacing::Always);
    f.render_stateful_widget(table_widget, area, &mut self.state);

    if !self.filtered_crates.is_empty() {
      for space in spacers.iter().skip(2).cloned() {
        f.render_widget(
          Text::from(
            std::iter::once(" ")
              .chain(std::iter::once(" "))
              .chain(std::iter::once(" "))
              .chain(std::iter::repeat("│").take(space.height.into()))
              .map(Line::from)
              .collect_vec(),
          )
          .style(Style::default().fg(Color::DarkGray)),
          space,
        );
      }
    }
  }

  fn background(&self) -> impl Widget {
    Block::default().bg(config::get().background_color)
  }

  fn render_scrollbar(&mut self, f: &mut Frame<'_>, area: Rect) {
    let mut state = self.scrollbar_state;
    f.render_stateful_widget(
      Scrollbar::default().track_symbol(Some(" ")).begin_symbol(None).end_symbol(None),
      area,
      &mut state,
    );
  }

  fn input_block(&self) -> impl Widget {
    let ncrates = self.total_num_crates.unwrap_or_default();
    let loading_status = if self.loading_status.load(Ordering::SeqCst) {
      format!("Loaded {}...", ncrates)
    } else {
      format!(
        "{}/{}",
        self.state.selected().map_or(0, |n| (self.page.saturating_sub(1) * self.page_size) + n as u64 + 1),
        ncrates
      )
    };
    Block::default()
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
      .title(loading_status)
      .title_alignment(Alignment::Right)
      .border_style(match *self.mode.borrow() {
        Mode::PickerSearchQueryEditing => Style::default().fg(config::get().search_query_outline_color),
        Mode::PickerFilterEditing => Style::default().fg(config::get().filter_query_outline_color),
        _ => Style::default().add_modifier(Modifier::DIM),
      })
  }

  fn input_text(&self) -> impl Widget + '_ {
    let scroll = if *self.mode.borrow() == Mode::PickerSearchQueryEditing {
      self.search_horizontal_scroll
    } else if *self.mode.borrow() == Mode::PickerFilterEditing {
      self.filter_horizontal_scroll
    } else {
      0
    };
    Paragraph::new(self.input.value()).scroll((0, scroll as u16))
  }

  fn render_input(&mut self, f: &mut Frame, area: Rect) {
    f.render_widget(self.input_block(), area);
    f.render_widget(self.input_text(), area.inner(&Margin { horizontal: 2, vertical: 2 }));
  }

  fn render_cursor(&mut self, f: &mut Frame<'_>, area: Rect) {
    if *self.mode.borrow() == Mode::PickerSearchQueryEditing || *self.mode.borrow() == Mode::PickerFilterEditing {
      f.set_cursor((area.x + 2 + self.input.cursor() as u16).min(area.x + area.width.saturating_sub(2)), area.y + 2)
    }
  }

  pub fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
    f.render_widget(self.background(), area);

    let [table, input] = Layout::vertical([Constraint::Fill(0), Constraint::Length(5)]).areas(area);

    let [table, scrollbar] = Layout::horizontal([Constraint::Fill(0), Constraint::Length(1)]).areas(table);
    self.render_scrollbar(f, scrollbar);

    let table = if self.show_crate_info {
      let [table, info] = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(table);
      self.render_crate_info(f, info);
      table
    } else {
      table
    };

    self.render_table(f, table);

    self.render_input(f, input);
    self.render_cursor(f, input);

    Ok(())
  }
}
