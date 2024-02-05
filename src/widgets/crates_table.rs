use itertools::Itertools;
use num_format::{Locale, ToFormattedString};
use ratatui::{prelude::*, widgets::*};

use crate::config;

pub struct CratesTable<'a> {
  crates: &'a [crates_io_api::Crate],
  highlight: bool,
}

impl<'a> CratesTable<'a> {
  pub fn new(crates: &'a [crates_io_api::Crate], highlight: bool) -> Self {
    Self { crates, highlight }
  }
}

impl<'a> StatefulWidget for CratesTable<'a> {
  type State = (&'a mut TableState, &'a mut ScrollbarState);

  fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
    Scrollbar::default()
      .track_symbol(Some(" "))
      .begin_symbol(None)
      .end_symbol(None)
      .render(area, buf, state.1);

    let widths = [
      Constraint::Length(1),
      Constraint::Max(20),
      Constraint::Min(0),
      Constraint::Max(10),
      Constraint::Max(20),
    ];
    let (areas, spacers) = Layout::horizontal(widths)
      .spacing(1)
      .split_with_spacers(area.inner(&Margin { horizontal: 1, vertical: 0 }));
    let description_area = areas[2];
    let text_wrap_width = description_area.width as usize;

    let table_widget = {
      let selected_style = Style::default();
      let header = Row::new(
        ["Name", "Description", "Downloads", "Last Updated"]
          .iter()
          .map(|h| Text::from(vec!["".into(), Line::from(h.bold()), "".into()])),
      )
      .bg(config::get().style.background_color)
      .height(3);
      let highlight_symbol = if self.highlight { " \u{2022} " } else { "   " };

      let rows = self.crates.iter().enumerate().map(|(i, item)| {
        let mut desc =
          textwrap::wrap(&item.description.clone().unwrap_or_default(), text_wrap_width)
            .iter()
            .map(|s| Line::from(s.to_string()))
            .collect_vec();
        desc.insert(0, "".into());
        let height = desc.len();
        Row::new([
          Text::from(vec!["".into(), Line::from(item.name.clone()), "".into()]),
          Text::from(desc),
          Text::from(vec![
            "".into(),
            Line::from(item.downloads.to_formatted_string(&Locale::en)),
            "".into(),
          ]),
          Text::from(vec![
            "".into(),
            Line::from(item.updated_at.format("%Y-%m-%d %H:%M:%S").to_string()),
            "".into(),
          ]),
        ])
        .bg(match i % 2 {
          0 => config::get().style.row_background_color_1,
          1 => config::get().style.row_background_color_2,
          _ => unreachable!("Cannot reach this line"),
        })
        .height(height.saturating_add(1) as u16)
      });

      let widths =
        [Constraint::Max(20), Constraint::Min(0), Constraint::Max(10), Constraint::Max(20)];
      Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .highlight_style(selected_style)
        .highlight_symbol(Text::from(vec!["".into(), highlight_symbol.into(), "".into()]))
        .highlight_spacing(HighlightSpacing::Always)
    };

    StatefulWidget::render(table_widget, area, buf, state.0);

    // only render margins when there's items in the table
    if !self.crates.is_empty() {
      // don't render margin for the first column
      for space in spacers.iter().skip(2).copied() {
        Text::from(
          std::iter::once(" ")
            .chain(std::iter::once(" "))
            .chain(std::iter::once(" "))
            .chain(std::iter::repeat("│").take(space.height as usize))
            .map(Line::from)
            .collect_vec(),
        )
        .style(Style::default().fg(Color::DarkGray))
        .render(space, buf);
      }
    }
  }
}
