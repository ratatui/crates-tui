use itertools::Itertools;
use ratatui::{prelude::*, widgets::*};

use crate::{app::Mode, command::Command, config};

#[derive(Default, Debug, Clone)]
pub struct Help {
    pub state: TableState,
    pub mode: Option<Mode>,
    pub skip: Vec<usize>,
}

impl Help {
    pub fn new(state: TableState, mode: Option<Mode>) -> Self {
        Self {
            state,
            mode,
            skip: Default::default(),
        }
    }

    pub fn scroll_previous(&mut self) {
        let i = self.state.selected().map_or(0, |i| i.saturating_sub(1));
        self.state.select(Some(i));
        if self.skip.contains(&i) {
            self.scroll_previous();
        }
    }

    pub fn scroll_next(&mut self) {
        let i = self.state.selected().map_or(0, |i| i.saturating_add(1));
        self.state.select(Some(i));
        if self.skip.contains(&i) {
            self.scroll_next();
        }
    }
}

pub struct HelpWidget;

const HIGHLIGHT_SYMBOL: &str = "█ ";

fn get_commands(mode: Mode, command: Command) -> impl Iterator<Item = (Mode, String, Command)> {
    config::get()
        .key_bindings
        .get_config_for_command(mode, command.clone())
        .into_iter()
        .map(move |s| (mode, s, command.clone()))
}

impl StatefulWidget for &HelpWidget {
    type State = Help;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        use Constraint::*;

        let [_, area] = Layout::vertical([Min(0), Percentage(90)]).areas(area);

        let [_, area, _] = Layout::horizontal([Min(0), Percentage(85), Min(0)]).areas(area);

        let skip = &mut state.skip;
        let rows = std::iter::once((Mode::Help, "ESC".into(), Command::SwitchToLastMode))
            .chain(vec![(Mode::Help, "".into(), Command::SwitchToLastMode)])
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::SwitchMode(Mode::Help),
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::SwitchMode(Mode::Summary),
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::SwitchMode(Mode::Search),
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::SwitchMode(Mode::Filter),
            ))
            .chain(get_commands(Mode::PickerShowCrateInfo, Command::ScrollDown))
            .chain(get_commands(Mode::PickerShowCrateInfo, Command::ScrollUp))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::ScrollCrateInfoUp,
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::ScrollCrateInfoDown,
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::ToggleSortBy {
                    reload: true,
                    forward: true,
                },
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::ToggleSortBy {
                    reload: true,
                    forward: false,
                },
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::ToggleSortBy {
                    reload: false,
                    forward: true,
                },
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::ToggleSortBy {
                    reload: false,
                    forward: false,
                },
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::IncrementPage,
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::DecrementPage,
            ))
            .chain(get_commands(Mode::PickerShowCrateInfo, Command::ReloadData))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::ToggleShowCrateInfo,
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::OpenDocsUrlInBrowser,
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::OpenCratesIOUrlInBrowser,
            ))
            .chain(get_commands(
                Mode::PickerShowCrateInfo,
                Command::CopyCargoAddCommandToClipboard,
            ))
            .chain(vec![(Mode::Help, "".into(), Command::SwitchToLastMode)])
            .chain(get_commands(Mode::Summary, Command::Quit))
            .chain(get_commands(Mode::Summary, Command::ScrollDown))
            .chain(get_commands(Mode::Summary, Command::ScrollUp))
            .chain(get_commands(Mode::Summary, Command::PreviousSummaryMode))
            .chain(get_commands(Mode::Summary, Command::NextSummaryMode))
            .chain(get_commands(Mode::Summary, Command::SwitchMode(Mode::Help)))
            .chain(get_commands(
                Mode::Summary,
                Command::SwitchMode(Mode::Search),
            ))
            .chain(get_commands(
                Mode::Summary,
                Command::SwitchMode(Mode::Filter),
            ))
            .chain(vec![(Mode::Help, "".into(), Command::SwitchToLastMode)])
            .chain(get_commands(
                Mode::Search,
                Command::SwitchMode(Mode::PickerHideCrateInfo),
            ))
            .chain(get_commands(Mode::Search, Command::SubmitSearch))
            .chain(get_commands(
                Mode::Search,
                Command::ToggleSortBy {
                    reload: false,
                    forward: true,
                },
            ))
            .chain(get_commands(
                Mode::Search,
                Command::ToggleSortBy {
                    reload: false,
                    forward: false,
                },
            ))
            .chain(get_commands(
                Mode::Search,
                Command::ToggleSortBy {
                    reload: true,
                    forward: true,
                },
            ))
            .chain(get_commands(
                Mode::Search,
                Command::ToggleSortBy {
                    reload: true,
                    forward: false,
                },
            ))
            .chain(get_commands(Mode::Search, Command::ScrollSearchResultsUp))
            .chain(get_commands(Mode::Search, Command::ScrollSearchResultsDown))
            .chain(get_commands(
                Mode::Filter,
                Command::SwitchMode(Mode::PickerHideCrateInfo),
            ))
            .chain(get_commands(Mode::Filter, Command::ScrollSearchResultsUp))
            .chain(get_commands(Mode::Filter, Command::ScrollSearchResultsDown))
            .collect_vec();

        if let Some(mode) = state.mode {
            tracing::debug!("{:?}", mode);
            let select = rows
                .iter()
                .find_position(|(m, _, _)| mode == *m)
                .map(|(i, _)| i)
                .unwrap_or_default();
            *state.state.selected_mut() = Some(select);
            *state.state.offset_mut() = select.saturating_sub(2);
            state.mode = None;
        };

        let rows = rows
            .iter()
            .enumerate()
            .map(|(i, (m, s, a))| {
                if s.is_empty() {
                    skip.push(i);
                    Row::new([
                        Text::from(vec!["".into()]),
                        Text::from(vec!["".into()]),
                        Text::from(vec!["".into()]),
                    ])
                } else {
                    Row::new([
                        Text::from(vec![Line::from(format!("{} ", m).fg(Color::DarkGray))]),
                        Text::from(vec![Line::from(format!("{} ", s))]),
                        Text::from(vec![Line::from(format!("{:?}", a))]),
                    ])
                    .fg(config::get().color.base05)
                    .bg(config::get().color.base00)
                }
            })
            .collect_vec();

        *state.state.selected_mut() = Some(
            state
                .state
                .selected()
                .unwrap_or_default()
                .min(rows.len().saturating_sub(1)),
        );

        let widths = [Constraint::Max(10), Constraint::Max(20), Constraint::Min(0)];
        let table = Table::new(rows, widths)
            .header(
                Row::new(
                    ["Mode", "Key Chords", "Command"]
                        .iter()
                        .map(|h| Text::from(vec![Line::from(h.bold()), "".into()])),
                )
                .fg(config::get().color.base05)
                .bg(config::get().color.base00),
            )
            .column_spacing(5)
            .highlight_symbol(HIGHLIGHT_SYMBOL)
            .highlight_style(config::get().color.base05)
            .highlight_spacing(HighlightSpacing::Always);
        StatefulWidget::render(table, area, buf, &mut state.state);
    }
}
