use ratatui::{layout::Flex, prelude::*, widgets::*};
pub struct Popup<'a> {
  title: &'a str,
  message: &'a str,
  scroll: usize,
}

impl<'a> Popup<'a> {
  pub fn new(title: &'a str, message: &'a str, scroll: usize) -> Self {
    Self { title, message, scroll }
  }
}

impl Widget for Popup<'_> {
  fn render(self, area: Rect, buf: &mut Buffer) {
    let [center] = Layout::vertical([Constraint::Percentage(50)]).flex(Flex::Center).areas(area);
    let [center] =
      Layout::horizontal([Constraint::Percentage(50)]).flex(Flex::Center).areas(center);
    Clear.render(center, buf);
    Paragraph::new(self.message)
      .block(
        Block::bordered().title(block::Title::from(self.title)).title(
          block::Title::from("Press `ESC` to exit")
            .position(block::Position::Bottom)
            .alignment(Alignment::Right),
        ),
      )
      .wrap(Wrap { trim: false })
      .scroll((self.scroll as u16, 0))
      .render(center, buf);
  }
}
