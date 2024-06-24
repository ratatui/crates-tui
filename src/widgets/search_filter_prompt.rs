use ratatui::{layout::Constraint::*, layout::Position, prelude::*, widgets::*};

use crate::{app::Mode, config};

use super::search_page::SearchMode;

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SearchFilterPrompt {
    cursor_position: Option<Position>,
}

impl SearchFilterPrompt {
    pub fn cursor_position(&self) -> Option<Position> {
        self.cursor_position
    }
}

pub struct SearchFilterPromptWidget<'a> {
    mode: Mode,
    sort: crates_io_api::Sort,
    input: &'a tui_input::Input,
    vertical_margin: u16,
    horizontal_margin: u16,
    search_mode: SearchMode,
}

impl<'a> SearchFilterPromptWidget<'a> {
    pub fn new(
        mode: Mode,
        sort: crates_io_api::Sort,
        input: &'a tui_input::Input,
        search_mode: SearchMode,
    ) -> Self {
        Self {
            mode,
            sort,
            input,
            vertical_margin: 2,
            horizontal_margin: 2,
            search_mode,
        }
    }
}

impl StatefulWidget for SearchFilterPromptWidget<'_> {
    type State = SearchFilterPrompt;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let [input, meta] = Layout::horizontal([Percentage(75), Fill(0)]).areas(area);

        self.input_block().render(area, buf);

        if self.search_mode.is_focused() {
            self.sort_by_info().render(meta.inner(self.margin()), buf);
        }
        self.input_text(input.width as usize)
            .render(input.inner(self.margin()), buf);

        self.update_cursor_state(area, state);
    }
}

impl SearchFilterPromptWidget<'_> {
    fn input_block(&self) -> Block {
        let borders = if self.search_mode.is_focused() {
            Borders::ALL
        } else {
            Borders::NONE
        };
        let border_color = match self.mode {
            Mode::Search => config::get().color.base0a,
            Mode::Filter => config::get().color.base0b,
            _ => config::get().color.base06,
        };
        let input_block = Block::default()
            .borders(borders)
            .fg(config::get().color.base05)
            .border_style(border_color);
        input_block
    }

    fn sort_by_info(&self) -> impl Widget {
        Paragraph::new(Line::from(vec![
            "Sort By: ".into(),
            format!("{:?}", self.sort.clone()).fg(config::get().color.base0d),
        ]))
        .right_aligned()
    }

    fn input_text(&self, width: usize) -> impl Widget + '_ {
        let scroll = self.input.cursor().saturating_sub(width.saturating_sub(4));
        let text = if self.search_mode.is_focused() {
            Line::from(vec![self.input.value().into()])
        } else if self.mode.is_summary() || self.mode.is_help() {
            Line::from(vec![])
        } else {
            Line::from(vec![
                self.input.value().into(),
                " (".into(),
                format!("{:?}", self.sort.clone()).fg(config::get().color.base0d),
                ")".into(),
            ])
        };
        Paragraph::new(text).scroll((0, scroll as u16))
    }

    fn update_cursor_state(&self, area: Rect, state: &mut SearchFilterPrompt) {
        let width = ((area.width as f64 * 0.75) as u16).saturating_sub(2);
        if self.search_mode.is_focused() {
            let margin = self.margin();
            state.cursor_position = Some(Position::new(
                (area.x + margin.horizontal + self.input.cursor() as u16).min(width),
                area.y + margin.vertical,
            ));
        } else {
            state.cursor_position = None
        }
    }

    fn margin(&self) -> Margin {
        if self.search_mode.is_focused() {
            Margin::new(self.horizontal_margin, self.vertical_margin)
        } else {
            Margin::default()
        }
    }
}
