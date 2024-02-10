use itertools::Itertools;
use num_format::{Locale, ToFormattedString};
use ratatui::{prelude::*, widgets::*};

use crate::config;

#[derive(Debug, Default)]
pub struct SearchResults {
    pub crates: Vec<crates_io_api::Crate>,
    pub versions: Vec<crates_io_api::Version>,
    pub table_state: TableState,
    pub scrollbar_state: ScrollbarState,
}

impl SearchResults {
    pub fn selected_crate_name(&self) -> Option<String> {
        self.selected()
            .and_then(|index| self.crates.get(index))
            .filter(|krate| !krate.name.is_empty())
            .map(|krate| krate.name.clone())
    }

    pub fn selected(&self) -> Option<usize> {
        self.table_state.selected()
    }

    pub fn content_length(&mut self, content_length: usize) {
        self.scrollbar_state = self.scrollbar_state.content_length(content_length)
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.table_state.select(index)
    }

    pub fn scroll_next(&mut self) {
        let wrap_index = self.crates.len().max(1);
        let next = self
            .table_state
            .selected()
            .map_or(0, |i| (i + 1) % wrap_index);
        self.scroll_to(next);
    }

    pub fn scroll_previous(&mut self) {
        let last = self.crates.len().saturating_sub(1);
        let wrap_index = self.crates.len().max(1);
        let previous = self
            .table_state
            .selected()
            .map_or(last, |i| (i + last) % wrap_index);
        self.scroll_to(previous);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_to(0);
    }

    pub fn scroll_to_bottom(&mut self) {
        let bottom = self.crates.len().saturating_sub(1);
        self.scroll_to(bottom);
    }

    fn scroll_to(&mut self, index: usize) {
        if self.crates.is_empty() {
            self.table_state.select(None)
        } else {
            self.table_state.select(Some(index));
            self.scrollbar_state = self.scrollbar_state.position(index);
        }
    }
}

pub struct SearchResultsWidget {
    highlight: bool,
}

impl SearchResultsWidget {
    pub fn new(highlight: bool) -> Self {
        Self { highlight }
    }
}

impl StatefulWidget for SearchResultsWidget {
    type State = SearchResults;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let [area, scrollbar_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(area);

        let [_, scrollbar_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(scrollbar_area);

        Scrollbar::default()
            .track_symbol(Some(" "))
            .thumb_symbol("▐")
            .begin_symbol(None)
            .end_symbol(None)
            .track_style(config::get().color.base06)
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
            let header = Row::new(
                ["Name", "Description", "Downloads"]
                    .iter()
                    .map(|h| Text::from(vec!["".into(), Line::from(h.bold()), "".into()])),
            )
            .fg(config::get().color.base05)
            .bg(config::get().color.base00)
            .height(3);
            let highlight_symbol = if self.highlight {
                " █ "
            } else {
                " \u{2022} "
            };

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
                .style({
                    let s = Style::default()
                        .fg(config::get().color.base05)
                        .bg(match i % 2 {
                            0 => config::get().color.base00,
                            1 => config::get().color.base01,
                            _ => unreachable!("Cannot reach this line"),
                        });
                    if i == selected {
                        s.bg(config::get().color.base02)
                    } else {
                        s
                    }
                })
                .height(height.saturating_add(1) as u16)
            });

            let widths = [Constraint::Max(20), Constraint::Min(0), Constraint::Max(10)];
            Table::new(rows, widths)
                .header(header)
                .column_spacing(1)
                .highlight_symbol(Text::from(vec![
                    "".into(),
                    highlight_symbol.into(),
                    "".into(),
                ]))
                .highlight_style(config::get().color.base05)
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
                            std::iter::repeat("│".fg(config::get().color.base0f))
                                .take(space.height as usize),
                        )
                        .map(Line::from)
                        .collect_vec(),
                )
                .render(space, buf);
            }
        }
    }
}
