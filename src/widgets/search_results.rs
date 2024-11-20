use crates_io_api::Crate;
use itertools::Itertools;
use num_format::{Locale, ToFormattedString};
use ratatui::{prelude::*, widgets::*};
use unicode_width::UnicodeWidthStr;

use crate::config;

#[derive(Debug, Default)]
pub struct SearchResults {
    pub crates: Vec<crates_io_api::Crate>,
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
        use Constraint::*;
        const TABLE_HEADER_HEIGHT: u16 = 3;
        const COLUMN_SPACING: u16 = 3;

        let [table_area, scrollbar_area] = Layout::horizontal([Fill(1), Length(1)]).areas(area);
        let [_, scrollbar_area] =
            Layout::vertical([Length(TABLE_HEADER_HEIGHT), Fill(1)]).areas(scrollbar_area);

        Scrollbar::default()
            .track_symbol(Some(" "))
            .thumb_symbol("▐")
            .begin_symbol(None)
            .end_symbol(None)
            .track_style(config::get().color.base06)
            .render(scrollbar_area, buf, &mut state.scrollbar_state);

        let highlight_symbol = if self.highlight {
            " █ "
        } else {
            " \u{2022} "
        };

        let column_widths = [Max(20), Fill(1), Max(11)];

        // Emulate the table layout calculations using Layout so we can render the vertical borders
        // in the space between the columns and can wrap the description field based on the actual
        // width of the description column
        let highlight_symbol_width = highlight_symbol.width() as u16;
        let [_highlight_column, table_columns] =
            Layout::horizontal([Length(highlight_symbol_width), Fill(1)]).areas(table_area);
        let column_layout = Layout::horizontal(column_widths).spacing(COLUMN_SPACING);
        let [_name_column, description_column, _downloads_column] =
            column_layout.areas(table_columns);
        let spacers: [Rect; 4] = column_layout.spacers(table_columns);

        let vertical_pad = |line| Text::from(vec!["".into(), line, "".into()]);

        let header_cells = ["Name", "Description", "Downloads"]
            .map(|h| h.bold().into())
            .map(vertical_pad);
        let header = Row::new(header_cells)
            .fg(config::get().color.base05)
            .bg(config::get().color.base00)
            .height(TABLE_HEADER_HEIGHT);

        let description_column_width = description_column.width as usize;
        let selected_index = state.selected().unwrap_or_default();
        let rows = state
            .crates
            .iter()
            .enumerate()
            .map(|(index, krate)| {
                row_from_crate(krate, description_column_width, index, selected_index)
            })
            .collect_vec();

        let table = Table::new(rows, column_widths)
            .header(header)
            .column_spacing(COLUMN_SPACING)
            .highlight_symbol(vertical_pad(highlight_symbol.into()))
            .row_highlight_style(config::get().color.base05)
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(table, table_area, buf, &mut state.table_state);

        render_table_borders(state, spacers, buf);
    }
}

fn row_from_crate(
    krate: &Crate,
    description_column_width: usize,
    index: usize,
    selected_index: usize,
) -> Row {
    let mut description = textwrap::wrap(
        &krate.description.clone().unwrap_or_default(),
        description_column_width,
    )
    .iter()
    .map(|s| Line::from(s.to_string()))
    .collect_vec();
    description.insert(0, "".into());
    description.push("".into());
    let vertical_padded = |line| Text::from(vec!["".into(), line, "".into()]);
    let crate_name = Line::from(krate.name.clone());
    let downloads = Line::from(krate.downloads.to_formatted_string(&Locale::en)).right_aligned();
    let description_height = description.len() as u16;
    Row::new([
        vertical_padded(crate_name),
        Text::from(description),
        vertical_padded(downloads),
    ])
    .height(description_height)
    .fg(config::get().color.base05)
    .bg(bg_color(index, selected_index))
}

fn bg_color(index: usize, selected_index: usize) -> Color {
    if index == selected_index {
        config::get().color.base02
    } else {
        match index % 2 {
            0 => config::get().color.base00,
            1 => config::get().color.base01,
            _ => unreachable!("mod 2 is always 0 or 1"),
        }
    }
}

fn render_table_borders(state: &mut SearchResults, spacers: [Rect; 4], buf: &mut Buffer) {
    // only render margins when there's items in the table
    if !state.crates.is_empty() {
        // don't render margin for the first column
        for space in spacers.iter().skip(1).copied() {
            Text::from(
                std::iter::once(" ".into())
                    .chain(std::iter::once(" ".into()))
                    .chain(std::iter::once(" ".into()))
                    .chain(
                        std::iter::repeat(" │".fg(config::get().color.base0f))
                            .take(space.height as usize),
                    )
                    .map(Line::from)
                    .collect_vec(),
            )
            .render(space, buf);
        }
    }
}
