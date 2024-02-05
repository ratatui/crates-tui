use ratatui::{prelude::*, widgets::*};

use crate::{app::Mode, config};

pub struct Prompt<'a> {
  total_num_crates: u64,
  loading: bool,
  selected: u64,
  mode: Mode,
  input: &'a tui_input::Input,
  vertical_margin: u16,
  horizontal_margin: u16,
}

impl<'a> Prompt<'a> {
  pub fn new(total_num_crates: u64, selected: u64, loading: bool, mode: Mode, input: &'a tui_input::Input) -> Self {
    let vertical_margin = 1 + config::get().prompt_padding;
    let horizontal_margin = 1 + config::get().prompt_padding;
    Self { total_num_crates, loading, selected, mode, input, vertical_margin, horizontal_margin }
  }

  pub fn render_spinner(&self, f: &mut Frame, area: Rect) {
    let spinner = ["◑", "◒", "◐", "◓"];
    let index = f.count() % spinner.len();
    let symbol = spinner[index];

    f.buffer_mut().set_string(area.x + area.width.saturating_sub(1), area.y, symbol, Style::default());
  }

  pub fn render_cursor(&self, f: &mut Frame, area: Rect) {
    if self.mode == Mode::Search || self.mode == Mode::Filter {
      f.set_cursor(
        (area.x + self.horizontal_margin + self.input.cursor() as u16).min(area.x + area.width.saturating_sub(2)),
        area.y + self.vertical_margin,
      );
    }
  }

  fn input_block(&self) -> impl Widget {
    let ncrates = self.total_num_crates;
    let loading_status =
      if self.loading { format!("Loaded {ncrates} ...") } else { format!("{}/{}", self.selected, ncrates) };
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
      .border_style(match self.mode {
        Mode::Search => Style::default().fg(config::get().style.search_query_outline_color),
        Mode::Filter => Style::default().fg(config::get().style.filter_query_outline_color),
        _ => Style::default().add_modifier(Modifier::DIM),
      })
  }

  fn input_text(&self, width: usize) -> impl Widget + '_ {
    let scroll = self.input.cursor().saturating_sub(width.saturating_sub(4));
    Paragraph::new(self.input.value()).scroll((0, scroll as u16))
  }
}

impl Widget for &Prompt<'_> {
  fn render(self, area: Rect, buf: &mut Buffer) {
    self.input_block().render(area, buf);
    self
      .input_text(area.width as usize)
      .render(area.inner(&Margin { horizontal: self.horizontal_margin, vertical: self.vertical_margin }), buf);
  }
}
