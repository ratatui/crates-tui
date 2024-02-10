use ratatui::{prelude::*, widgets::*};

use crate::{app::Mode, command::Command, config};

pub struct StatusBarWidget {
    mode: Mode,
}

impl StatusBarWidget {
    pub fn new(mode: Mode) -> Self {
        Self { mode }
    }
}

impl Widget for StatusBarWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.input_block().render(area, buf);
    }
}

impl StatusBarWidget {
    fn input_block(&self) -> Block {
        let line = if self.mode.is_filter() {
            vec!["Filter: ".into(), "Enter".bold(), " to submit".into()]
        } else if self.mode.is_search() {
            vec!["Search: ".into(), "Enter".bold(), " to submit".into()]
        } else if self.mode.is_summary() {
            let help = config::get()
                .key_bindings
                .get_config_for_command(self.mode, Command::SwitchMode(Mode::Help))
                .into_iter()
                .next()
                .unwrap_or_default();
            let open_in_browser = config::get()
                .key_bindings
                .get_config_for_command(self.mode, Command::OpenCratesIOUrlInBrowser)
                .into_iter()
                .next()
                .unwrap_or_default();
            let search = config::get()
                .key_bindings
                .get_config_for_command(Mode::Common, Command::NextTab)
                .into_iter()
                .next()
                .unwrap_or_default();
            vec![
                open_in_browser.bold(),
                " to open in browser, ".into(),
                search.bold(),
                " to enter search, ".into(),
                help.bold(),
                " for help".into(),
            ]
        } else if self.mode.is_help() {
            vec!["ESC".bold(), " to return".into()]
        } else {
            let search = config::get()
                .key_bindings
                .get_config_for_command(self.mode, Command::SwitchMode(Mode::Search))
                .into_iter()
                .next()
                .unwrap_or_default();
            let filter = config::get()
                .key_bindings
                .get_config_for_command(self.mode, Command::SwitchMode(Mode::Filter))
                .into_iter()
                .next()
                .unwrap_or_default();
            let help = config::get()
                .key_bindings
                .get_config_for_command(self.mode, Command::SwitchMode(Mode::Help))
                .into_iter()
                .next()
                .unwrap_or_default();
            vec![
                search.bold(),
                " to search, ".into(),
                filter.bold(),
                " to filter, ".into(),
                help.bold(),
                " for help".into(),
            ]
        };
        let borders = Borders::NONE;
        let alignment = Alignment::Right;
        let border_color = match self.mode {
            Mode::Search => config::get().color.base0a,
            Mode::Filter => config::get().color.base0b,
            _ => config::get().color.base06,
        };
        let input_block = Block::default()
            .borders(borders)
            .title(block::Title::from(Line::from(line)).alignment(alignment))
            .fg(config::get().color.base05)
            .border_style(border_color);
        if self.mode.is_search() {
            let help = config::get()
                .key_bindings
                .get_config_for_command(self.mode, Command::SwitchMode(Mode::Help))
                .into_iter()
                .next()
                .unwrap_or_default();
            let toggle_sort = config::get()
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
                .unwrap_or_default();
            input_block
                .title(Line::from(vec![
                    toggle_sort.bold(),
                    " to toggle sort".into(),
                ]))
                .title_alignment(Alignment::Right)
                .title(
                    block::Title::from(Line::from(vec![help.bold(), " for help".into()]))
                        .position(block::Position::Bottom)
                        .alignment(Alignment::Right),
                )
        } else {
            input_block
        }
    }
}
