use itertools::Itertools;
use ratatui::{
    layout::Flex,
    prelude::*,
    widgets::{block::*, *},
};

#[derive(Debug, Default, Clone, Copy)]
pub struct PopupMessageState {
    scroll: usize,
}

impl PopupMessageState {
    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1)
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1)
    }

    pub fn scroll_top(&mut self) {
        self.scroll = 0;
    }
}

#[derive(Debug, Clone)]
pub struct PopupMessageWidget {
    title: String,
    message: String,
}

impl PopupMessageWidget {
    pub fn new(title: String, message: String) -> Self {
        Self { title, message }
    }
}

impl StatefulWidget for &PopupMessageWidget {
    type State = PopupMessageState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let [center] = Layout::horizontal([Constraint::Percentage(50)])
            .flex(Flex::Center)
            .areas(area);

        let message = textwrap::wrap(&self.message, center.width as usize)
            .iter()
            .map(|s| Line::from(s.to_string()))
            .collect_vec();
        let line_count = message.len();
        let [center] = Layout::vertical([Constraint::Length(line_count as u16 + 3)])
            .flex(Flex::Center)
            .areas(center);

        state.scroll = state.scroll.min(line_count.saturating_sub(1));
        let instruction = Title::from(vec!["Esc".bold(), " to close".into()])
            .position(Position::Bottom)
            .alignment(Alignment::Right);
        let block = Block::bordered()
            .border_style(Color::DarkGray)
            .title(self.title.clone())
            .title(instruction);
        Clear.render(center, buf);
        Paragraph::new(self.message.clone())
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((state.scroll as u16, 0))
            .render(center, buf);
    }
}
