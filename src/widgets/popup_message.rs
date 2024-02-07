use itertools::Itertools;
use ratatui::{layout::Flex, prelude::*, widgets::*};
pub struct PopupMessageWidget<'a> {
    title: &'a str,
    message: &'a str,
    scroll: usize,
}

impl<'a> PopupMessageWidget<'a> {
    pub fn new(title: &'a str, message: &'a str, scroll: usize) -> Self {
        Self {
            title,
            message,
            scroll,
        }
    }
}

impl Widget for PopupMessageWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [center] = Layout::horizontal([Constraint::Percentage(50)])
            .flex(Flex::Center)
            .areas(area);

        let message = textwrap::wrap(self.message, center.width as usize)
            .iter()
            .map(|s| Line::from(s.to_string()))
            .collect_vec();
        let height = message.len() as u16;

        let [center] = Layout::vertical([Constraint::Length(height + 3)])
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
            .scroll((self.scroll as u16, 0))
            .render(center, buf);
    }
}
