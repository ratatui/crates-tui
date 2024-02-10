use ratatui::{layout::Constraint::*, layout::Position, prelude::*, widgets::*};

use crate::{app::Mode, command::Command, config};

#[derive(Debug, Clone)]
pub struct SearchPrompt {
    input: tui_input::Input,
    sort: crates_io_api::Sort,
}

#[derive(Debug, Clone)]
pub struct SearchPromptState {
    pub cursor_position: Option<Position>,
}

const MARGIN: Margin = Margin::new(2, 2);

impl StatefulWidget for &SearchPrompt {
    type State = SearchPromptState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let [input_area, meta_area] = Layout::horizontal([Percentage(75), Fill(0)]).areas(area);

        self.input_block().render(area, buf);

        let sort_area = meta_area.inner(&MARGIN);
        self.sort_by_info().render(sort_area, buf);

        self.input_text(input_area.width as usize)
            .render(input_area.inner(&MARGIN), buf);

        state.cursor_position = Some(self.cursor_position(area));
    }
}

impl SearchPrompt {
    fn cursor_position(&self, area: Rect) -> Position {
        // TODO base this on the actual area used rather than the area of the whole so we don't have to
        // calculate it twice
        let width = ((area.width as f64 * 0.75) as u16).saturating_sub(2);
        let x = (area.x + MARGIN.horizontal + self.input.cursor() as u16).min(width);
        let y = area.y + MARGIN.vertical;
        Position::new(x, y)
    }

    fn input_block(&self) -> Block {
        let border_color = config::get().color.base0a;
        let help_key = self.help_command_key();
        let toggle_sort_key = self.toggle_sort_key();
        let search_title = Line::from(vec!["Search: ".into(), "Enter".bold(), " to submit".into()]);
        let toggle_sort_title = Line::from(vec![toggle_sort_key.bold(), " to toggle sort".into()]);
        let help_title = Line::from(vec![help_key.bold(), " for help".into()]);
        Block::bordered()
            .fg(config::get().color.base05)
            .border_style(border_color)
            .title_top(search_title)
            .title_top(toggle_sort_title.right_aligned())
            .title_bottom(help_title.right_aligned())
    }

    // TODO make this a method on KeyBindings
    fn help_command_key(&self) -> String {
        config::get()
            .key_bindings
            .get_config_for_command(Mode::Search, Command::SwitchMode(Mode::Help))
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    fn toggle_sort_key(&self) -> String {
        config::get()
            .key_bindings
            .get_config_for_command(
                Mode::Search,
                Command::ToggleSortBy {
                    reload: false,
                    forward: true,
                },
            )
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    fn sort_by_info(&self) -> impl Widget {
        Line::from(vec![
            "Sort By: ".into(),
            format!("{:?}", self.sort.clone()).fg(config::get().color.base0d),
        ])
        .right_aligned()
    }

    fn input_text(&self, width: usize) -> impl Widget + '_ {
        let scroll = self.input.cursor().saturating_sub(width.saturating_sub(4));
        let text = Line::from(vec![self.input.value().into()]);
        Paragraph::new(text).scroll((0, scroll as u16))
    }
}
