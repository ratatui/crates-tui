use ratatui::{layout::Position, prelude::*, widgets::*};

use crate::{app::Mode, config};

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
    focused: bool,
    mode: Mode,
    input: &'a tui_input::Input,
    vertical_margin: u16,
    horizontal_margin: u16,
}

impl<'a> SearchFilterPromptWidget<'a> {
    pub fn new(focused: bool, mode: Mode, input: &'a tui_input::Input) -> Self {
        Self {
            focused,
            mode,
            input,
            vertical_margin: 2,
            horizontal_margin: 2,
        }
    }

    fn focused(&self) -> bool {
        self.focused
    }

    fn horizontal_margin(&self) -> u16 {
        if self.focused() {
            self.horizontal_margin
        } else {
            0
        }
    }

    fn vertical_margin(&self) -> u16 {
        if self.focused() {
            self.vertical_margin
        } else {
            0
        }
    }

    fn input_block(&self) -> impl Widget {
        Block::default()
            .borders(if self.focused() {
                Borders::ALL
            } else {
                Borders::NONE
            })
            .title(
                block::Title::from(Line::from(vec![
                    "Press ".into(),
                    "?".bold(),
                    " to search, ".into(),
                    "/".bold(),
                    " to filter, ".into(),
                    "Enter".bold(),
                    " to submit".into(),
                ]))
                .alignment(Alignment::Right),
            )
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

    fn update_cursor_state(&self, area: Rect, state: &mut SearchFilterPrompt) {
        let width = ((area.width as f64 * 0.75) as u16).saturating_sub(2);
        if self.focused() {
            state.cursor_position = Some(Position::new(
                (area.x + self.horizontal_margin() + self.input.cursor() as u16).min(width),
                area.y + self.vertical_margin(),
            ));
        } else {
            state.cursor_position = None
        }
    }
}

impl StatefulWidget for SearchFilterPromptWidget<'_> {
    type State = SearchFilterPrompt;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.input_block().render(area, buf);
        self.input_text((area.width as f64 * 0.75) as usize).render(
            area.inner(&Margin {
                horizontal: self.horizontal_margin(),
                vertical: self.vertical_margin(),
            }),
            buf,
        );
        self.update_cursor_state(area, state);
    }
}
