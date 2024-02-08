use ratatui::{prelude::*, widgets::*};
use strum::{Display, EnumIter, FromRepr};

use crate::config;

#[derive(Debug, Default, Clone, Copy, Display, FromRepr, EnumIter)]
pub enum SelectedTab {
    #[default]
    Summary,
    Search,
    None,
}

impl SelectedTab {
    pub fn select(&mut self, selected_tab: SelectedTab) {
        *self = selected_tab
    }

    pub fn highlight_style() -> Style {
        Style::default()
            .fg(config::get().color.base00)
            .bg(config::get().color.base0a)
            .bold()
    }
}

impl Widget for &SelectedTab {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            SelectedTab::Summary => self.render_tab_summary(area, buf),
            SelectedTab::Search => self.render_tab_search(area, buf),
            SelectedTab::None => (),
        }
    }
}

impl SelectedTab {
    pub fn title(&self) -> Line<'static> {
        match self {
            SelectedTab::None => "".into(),
            _ => format!("  {self}  ")
                .fg(config::get().color.base0d)
                .bg(config::get().color.base00)
                .into(),
        }
    }

    fn render_tab_summary(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Summary")
            .block(self.block())
            .render(area, buf)
    }

    fn render_tab_search(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Search")
            .block(self.block())
            .render(area, buf)
    }

    fn block(&self) -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .border_set(symbols::border::PLAIN)
            .padding(Padding::horizontal(1))
            .border_style(config::get().color.base03)
    }
}
