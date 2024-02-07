use itertools::Itertools;
use num_format::{Locale, ToFormattedString};
use ratatui::{prelude::*, widgets::*};

use crate::config;

#[derive(Debug, Default)]
pub struct SearchResultsTable {
    pub crates: Vec<crates_io_api::Crate>,
    pub versions: Vec<crates_io_api::Version>,
    pub table_state: TableState,
    pub scrollbar_state: ScrollbarState,
}

impl SearchResultsTable {
    pub fn selected_crate_name(&self) -> Option<String> {
        self.selected()
            .and_then(|index| self.crates.get(index))
            .filter(|crate_| !crate_.name.is_empty())
            .map(|crate_| crate_.name.clone())
    }

    pub fn content_length(&mut self, content_length: usize) {
        self.scrollbar_state = self.scrollbar_state.content_length(content_length)
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.table_state.select(index)
    }

    pub fn selected(&self) -> Option<usize> {
        self.table_state.selected()
    }

    pub fn scroll_next(&mut self, count: usize) {
        if self.crates.is_empty() {
            self.table_state.select(None)
        } else {
            // wrapping behavior
            let i = self
                .table_state
                .selected()
                .map_or(0, |i| (i + count) % self.crates.len());
            self.table_state.select(Some(i));
            self.scrollbar_state = self.scrollbar_state.position(i);
        }
    }

    pub fn scroll_previous(&mut self, count: usize) {
        if self.crates.is_empty() {
            self.table_state.select(None)
        } else {
            // wrapping behavior
            let i = self
                .table_state
                .selected()
                .map_or(self.crates.len().saturating_sub(1), |i| {
                    if i == 0 {
                        self.crates.len().saturating_sub(1)
                    } else {
                        i.saturating_sub(count)
                    }
                });
            self.table_state.select(Some(i));
            self.scrollbar_state = self.scrollbar_state.position(i);
        }
    }

    pub fn scroll_to_top(&mut self) {
        if self.crates.is_empty() {
            self.table_state.select(None)
        } else {
            self.table_state.select(Some(0));
            self.scrollbar_state = self.scrollbar_state.position(0);
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        if self.crates.is_empty() {
            self.table_state.select(None)
        } else {
            self.table_state.select(Some(self.crates.len() - 1));
            self.scrollbar_state = self.scrollbar_state.position(self.crates.len() - 1);
        }
    }
}

pub struct SearchResultsTableWidget {
    highlight: bool,
}

impl SearchResultsTableWidget {
    pub fn new(highlight: bool) -> Self {
        Self { highlight }
    }
}

impl StatefulWidget for SearchResultsTableWidget {
    type State = SearchResultsTable;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let [_, scrollbar_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);
        Scrollbar::default()
            .track_symbol(Some(" "))
            .begin_symbol(None)
            .end_symbol(None)
            .render(scrollbar_area, buf, &mut state.scrollbar_state);

        let widths = [
            Constraint::Length(1),
            Constraint::Max(20),
            Constraint::Min(0),
            Constraint::Max(10),
        ];
        let (areas, spacers) =
            Layout::horizontal(widths)
                .spacing(1)
                .split_with_spacers(area.inner(&Margin {
                    horizontal: 1,
                    vertical: 0,
                }));
        let description_area = areas[2];
        let text_wrap_width = description_area.width as usize;

        let selected = state.selected().unwrap_or_default();
        let table_widget = {
            let selected_style = Style::default();
            let header = Row::new(
                ["Name", "Description", "Downloads"]
                    .iter()
                    .map(|h| Text::from(vec!["".into(), Line::from(h.bold()), "".into()])),
            )
            .bg(config::get().style.background_color)
            .height(3);
            let highlight_symbol = if self.highlight { " \u{2022} " } else { "   " };

            let rows = state.crates.iter().enumerate().map(|(i, item)| {
                let mut desc = textwrap::wrap(
                    &item.description.clone().unwrap_or_default(),
                    text_wrap_width,
                )
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
                ])
                .bg(match i % 2 {
                    0 => config::get().style.row_background_color_1,
                    1 => config::get().style.row_background_color_2,
                    _ => unreachable!("Cannot reach this line"),
                })
                .height(if i == selected {
                    height.saturating_add(1) as u16
                } else {
                    // TODO: make this `3` when partial rendering is implemented
                    height.saturating_add(1) as u16
                })
            });

            let widths = [Constraint::Max(20), Constraint::Min(0), Constraint::Max(10)];
            Table::new(rows, widths)
                .header(header)
                .column_spacing(1)
                .highlight_style(selected_style)
                .highlight_symbol(Text::from(vec![
                    "".into(),
                    highlight_symbol.into(),
                    "".into(),
                ]))
                .highlight_spacing(HighlightSpacing::Always)
        };

        StatefulWidget::render(table_widget, area, buf, &mut state.table_state);

        // only render margins when there's items in the table
        if !state.crates.is_empty() {
            // don't render margin for the first column
            for space in spacers.iter().skip(2).copied() {
                Text::from(
                    std::iter::once(" ".into())
                        .chain(std::iter::once(" ".into()))
                        .chain(std::iter::once(" ".into()))
                        .chain(
                            std::iter::repeat("│".fg(Color::DarkGray)).take(space.height as usize),
                        )
                        .map(Line::from)
                        .collect_vec(),
                )
                .render(space, buf);
            }
        }
    }
}
