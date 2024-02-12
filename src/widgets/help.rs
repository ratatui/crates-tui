use itertools::Itertools;
use ratatui::{prelude::*, widgets::*};

use crate::{
    app::Mode,
    command::{Command, ALL_COMMANDS},
    config,
};

#[derive(Default, Debug, Clone)]
pub struct Help {
    pub state: TableState,
    pub mode: Option<Mode>,
}

impl Help {
    pub fn new(state: TableState, mode: Option<Mode>) -> Self {
        Self { state, mode }
    }

    pub fn scroll_up(&mut self) {
        let i = self.state.selected().map_or(0, |i| i.saturating_sub(1));
        self.state.select(Some(i));
    }

    pub fn scroll_down(&mut self) {
        let i = self.state.selected().map_or(0, |i| i.saturating_add(1));
        self.state.select(Some(i));
    }
}

pub struct HelpWidget;

const HIGHLIGHT_SYMBOL: &str = "█ ";

impl StatefulWidget for &HelpWidget {
    type State = Help;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        use Constraint::*;
        let [_, area] = Layout::vertical([Min(0), Percentage(90)]).areas(area);
        let [_, area, _] = Layout::horizontal([Min(0), Percentage(85), Min(0)]).areas(area);

        let all_key_bindings = all_key_bindings();
        select_by_mode(state, &all_key_bindings);

        let widths = [Max(10), Max(10), Min(0)];
        let header = Row::new(["Mode", "Keys", "Command"].map(|h| Line::from(h.bold())))
            .fg(config::get().color.base05)
            .bg(config::get().color.base00);
        let table = Table::new(into_rows(&all_key_bindings), widths)
            .header(header)
            .column_spacing(5)
            .highlight_symbol(HIGHLIGHT_SYMBOL)
            .highlight_style(config::get().color.base05)
            .highlight_spacing(HighlightSpacing::Always);
        StatefulWidget::render(table, area, buf, &mut state.state);
    }
}

/// Returns all key bindings for all commands and modes
///
/// The result is a vector of tuples containing the mode, command and key bindings joined by a comma
fn all_key_bindings() -> Vec<(Mode, Command, String)> {
    ALL_COMMANDS
        .iter()
        .flat_map(|(mode, commands)| {
            commands.iter().map(|command| {
                let key_bindings = key_bindings_for_command(*mode, *command);
                let key_bindings = key_bindings.join(", ");
                (*mode, *command, key_bindings)
            })
        })
        .collect_vec()
}

/// Returns the key bindings for a specific command and mode
fn key_bindings_for_command(mode: Mode, command: Command) -> Vec<String> {
    config::get()
        .key_bindings
        .get_config_for_command(mode, command)
}

/// updates the selected index based on the current mode
///
/// Only changes the selected index for the first render
fn select_by_mode(state: &mut Help, rows: &Vec<(Mode, Command, String)>) {
    if let Some(mode) = state.mode {
        tracing::debug!("{:?}", mode);
        let selected = rows
            .iter()
            .find_position(|(m, _, _)| *m == mode)
            .map(|(index, _)| index)
            .unwrap_or_default();
        *state.state.selected_mut() = Some(selected);
        *state.state.offset_mut() = selected.saturating_sub(2);
        // Reset the mode after the first render - let the user scroll
        state.mode = None;
    };
    // ensure the selected index is within the bounds
    *state.state.selected_mut() = Some(
        state
            .state
            .selected()
            .unwrap_or_default()
            .min(rows.len().saturating_sub(1)),
    );
}

fn into_rows<'a>(rows: &'a [(Mode, Command, String)]) -> impl Iterator<Item = Row<'a>> {
    rows.iter().map(|(mode, command, keys)| {
        Row::new([
            Line::styled(format!("{} ", mode), Color::DarkGray),
            Line::raw(format!("{}", keys)),
            Line::raw(format!("{:?} ", command)),
        ])
        .fg(config::get().color.base05)
        .bg(config::get().color.base00)
    })
}
