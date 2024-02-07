use itertools::Itertools;
use ratatui::{prelude::*, widgets::*};

use crate::{action::Action, app::Mode, config};

#[derive(Default, Debug, Clone)]
pub struct Help {
    pub state: TableState,
    pub mode: Mode,
}

impl Help {
    pub fn new(state: TableState, mode: Mode) -> Self {
        Self { state, mode }
    }

    pub fn scroll_previous(&mut self) {
        let i = self.state.selected().map_or(0, |i| i.saturating_sub(1));
        self.state.select(Some(i));
    }

    pub fn scroll_next(&mut self) {
        let i = self.state.selected().map_or(0, |i| i.saturating_add(1));
        self.state.select(Some(i));
    }
}

pub struct HelpWidget;

const HIGHLIGHT_SYMBOL: &str = "█ ";

fn get_actions(mode: Mode, action: Action) -> impl Iterator<Item = (Mode, String, Action)> {
    config::get()
        .key_bindings
        .get_config_for_action(mode, action.clone())
        .into_iter()
        .map(move |s| (mode, s, action.clone()))
}

impl StatefulWidget for &HelpWidget {
    type State = Help;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        use Constraint::*;

        let [_, area] = Layout::vertical([Min(0), Percentage(90)]).areas(area);

        let [_, area, _] = Layout::horizontal([Min(0), Percentage(85), Min(0)]).areas(area);

        let rows = std::iter::once((Mode::Help, "ESC".into(), Action::SwitchToLastMode))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::SwitchMode(Mode::Help),
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::SwitchMode(Mode::Summary),
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::SwitchMode(Mode::Search),
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::SwitchMode(Mode::Filter),
            ))
            .chain(get_actions(Mode::PickerShowCrateInfo, Action::ScrollDown))
            .chain(get_actions(Mode::PickerShowCrateInfo, Action::ScrollUp))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::ScrollCrateInfoUp,
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::ScrollCrateInfoDown,
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::ToggleSortBy {
                    reload: true,
                    forward: true,
                },
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::ToggleSortBy {
                    reload: true,
                    forward: false,
                },
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::ToggleSortBy {
                    reload: false,
                    forward: true,
                },
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::ToggleSortBy {
                    reload: false,
                    forward: false,
                },
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::IncrementPage,
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::DecrementPage,
            ))
            .chain(get_actions(Mode::PickerShowCrateInfo, Action::ReloadData))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::ToggleShowCrateInfo,
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::OpenDocsUrlInBrowser,
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::OpenCratesIOUrlInBrowser,
            ))
            .chain(get_actions(
                Mode::PickerShowCrateInfo,
                Action::CopyCargoAddCommandToClipboard,
            ))
            .chain(get_actions(Mode::Summary, Action::Quit))
            .chain(get_actions(Mode::Summary, Action::ScrollDown))
            .chain(get_actions(Mode::Summary, Action::ScrollUp))
            .chain(get_actions(Mode::Summary, Action::PreviousSummaryMode))
            .chain(get_actions(Mode::Summary, Action::NextSummaryMode))
            .chain(get_actions(Mode::Summary, Action::SwitchMode(Mode::Help)))
            .chain(get_actions(Mode::Summary, Action::SwitchMode(Mode::Search)))
            .chain(get_actions(Mode::Summary, Action::SwitchMode(Mode::Filter)))
            .chain(get_actions(
                Mode::Search,
                Action::SwitchMode(Mode::PickerHideCrateInfo),
            ))
            .chain(get_actions(Mode::Search, Action::SubmitSearch))
            .chain(get_actions(
                Mode::Search,
                Action::ToggleSortBy {
                    reload: false,
                    forward: true,
                },
            ))
            .chain(get_actions(
                Mode::Search,
                Action::ToggleSortBy {
                    reload: false,
                    forward: false,
                },
            ))
            .chain(get_actions(
                Mode::Search,
                Action::ToggleSortBy {
                    reload: true,
                    forward: true,
                },
            ))
            .chain(get_actions(
                Mode::Search,
                Action::ToggleSortBy {
                    reload: true,
                    forward: false,
                },
            ))
            .chain(get_actions(Mode::Summary, Action::SwitchMode(Mode::Filter)))
            .map(|(m, s, a)| {
                Row::new([
                    Text::from(vec![Line::from(format!("{} ", m).fg(Color::DarkGray))]),
                    Text::from(vec![Line::from(format!("{} ", s))]),
                    Text::from(vec![Line::from(format!("{:?}", a))]),
                ])
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
            .header(Row::new(
                ["Mode", "Key Chords", "Action"]
                    .iter()
                    .map(|h| Text::from(vec![Line::from(h.bold()), "".into()])),
            ))
            .column_spacing(5)
            .block(
                Block::default()
                    .title("Help".bold())
                    .title_alignment(Alignment::Left),
            )
            .highlight_symbol(HIGHLIGHT_SYMBOL)
            .highlight_spacing(HighlightSpacing::Always);
        StatefulWidget::render(table, area, buf, &mut state.state);
    }
}
