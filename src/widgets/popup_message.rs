use itertools::Itertools;
use ratatui::{layout::Flex, prelude::*, widgets::*};

#[derive(Debug, Default, Clone, Copy)]
pub struct Popup {
    scroll: usize,
}

impl Popup {
    pub fn scroll_previous(&mut self) {
        self.scroll = self.scroll.saturating_sub(1)
    }

    pub fn scroll_next(&mut self) {
        self.scroll = self.scroll.saturating_add(1)
    }

    pub fn reset(&mut self) {
        self.scroll = 0;
    }
}

pub struct PopupMessageWidget<'a> {
    title: &'a str,
    message: &'a str,
}

impl<'a> PopupMessageWidget<'a> {
    pub fn new(title: &'a str, message: &'a str) -> Self {
        Self { title, message }
    }
}

impl StatefulWidget for PopupMessageWidget<'_> {
    type State = Popup;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let [center] = Layout::horizontal([Constraint::Percentage(50)])
            .flex(Flex::Center)
            .areas(area);

        let message = textwrap::wrap(self.message, center.width as usize)
            .iter()
            .map(|s| Line::from(s.to_string()))
            .collect_vec();
        let height = message.len();
        state.scroll = state.scroll.min(height.saturating_sub(1));

        let [center] = Layout::vertical([Constraint::Length(height as u16 + 3)])
            .flex(Flex::Center)
            .areas(center);
        Clear.render(center, buf);

        Paragraph::new(self.message)
            .block(
                Block::bordered()
                    .border_style(Color::DarkGray)
                    .title(block::Title::from(self.title))
                    .title(
                        block::Title::from(Line::from(vec!["ESC".bold(), " to close".into()]))
                            .position(block::Position::Bottom)
                            .alignment(Alignment::Right),
                    ),
            )
            .wrap(Wrap { trim: false })
            .scroll((state.scroll as u16, 0))
            .render(center, buf);
    }
}
