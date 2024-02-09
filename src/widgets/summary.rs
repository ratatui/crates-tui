use itertools::Itertools;
use ratatui::{layout::Flex, prelude::*, widgets::*};
use strum::{Display, EnumIs, EnumIter, FromRepr};

use crate::config;

#[derive(Default, Debug, Clone, Copy, EnumIs, FromRepr, Display, EnumIter)]
pub enum SummaryMode {
    #[default]
    NewCrates,
    MostDownloaded,
    JustUpdated,
    MostRecentlyDownloaded,
    PopularKeywords,
    PopularCategories,
}

const HIGHLIGHT_SYMBOL: &str = "█";

impl SummaryMode {
    /// Get the previous tab, if there is no previous tab return the current tab.
    fn previous(&mut self) {
        let current_index: usize = *self as usize;
        let previous_index = current_index.saturating_sub(1);
        *self = Self::from_repr(previous_index).unwrap_or(*self)
    }

    /// Get the next tab, if there is no next tab return the current tab.
    fn next(&mut self) {
        let current_index = *self as usize;
        let next_index = current_index.saturating_add(1);
        *self = Self::from_repr(next_index).unwrap_or(*self)
    }

    fn url_prefix(&self) -> String {
        match self {
            SummaryMode::NewCrates => "https://crates.io/crates/",
            SummaryMode::MostDownloaded => "https://crates.io/crates/",
            SummaryMode::JustUpdated => "https://crates.io/crates/",
            SummaryMode::MostRecentlyDownloaded => "https://crates.io/crates/",
            SummaryMode::PopularKeywords => "https://crates.io/keywords/",
            SummaryMode::PopularCategories => "https://crates.io/categories/",
        }
        .into()
    }
}

#[derive(Default, Debug, Clone)]
pub struct Summary {
    pub state: [ListState; 6],
    pub last_selection: [usize; 6],
    pub mode: SummaryMode,
    pub summary_data: Option<crates_io_api::Summary>,
}

impl Summary {
    pub fn mode(&self) -> SummaryMode {
        self.mode
    }

    pub fn url(&self) -> Option<String> {
        let prefix = self.mode.url_prefix();
        if let Some(ref summary) = self.summary_data {
            let state = self.get_state(self.mode);
            let i = state.selected().unwrap_or_default().saturating_sub(1); // starting index for list is 1 because we render empty line as the 0th element
            tracing::debug!("i = {i}");
            let suffix = match self.mode {
                SummaryMode::NewCrates => summary.new_crates[i].name.clone(),
                SummaryMode::MostDownloaded => summary.most_downloaded[i].name.clone(),
                SummaryMode::JustUpdated => summary.most_downloaded[i].name.clone(),
                SummaryMode::MostRecentlyDownloaded => {
                    summary.most_recently_downloaded[i].name.clone()
                }
                SummaryMode::PopularKeywords => summary.popular_keywords[i].id.clone(),
                SummaryMode::PopularCategories => summary.popular_categories[i].slug.clone(),
            };
            Some(format!("{prefix}{suffix}"))
        } else {
            None
        }
    }

    pub fn get_state_mut(&mut self, mode: SummaryMode) -> &mut ListState {
        &mut self.state[mode as usize]
    }

    pub fn get_state(&self, mode: SummaryMode) -> &ListState {
        &self.state[mode as usize]
    }

    pub fn selected(&self, mode: SummaryMode) -> Option<usize> {
        self.get_state(mode).selected().map(|i| i.max(1)) // never let index go to 0 because we render an empty line as a the first element
    }

    pub fn scroll_previous(&mut self) {
        let state = self.get_state_mut(self.mode);
        let i = state.selected().map_or(0, |i| i.saturating_sub(1));
        state.select(Some(i));
    }

    pub fn scroll_next(&mut self) {
        let state = self.get_state_mut(self.mode);
        let i = state.selected().map_or(0, |i| i.saturating_add(1));
        state.select(Some(i));
    }

    pub fn save_state(&mut self) {
        if let Some(i) = self.get_state(self.mode).selected() {
            self.last_selection[self.mode as usize] = i
        }
    }

    pub fn next_mode(&mut self) {
        self.save_state();
        let old_state = self.get_state_mut(self.mode);
        *old_state.selected_mut() = None;
        self.mode.next();
        let i = self.last_selection[self.mode as usize];
        let new_state = self.get_state_mut(self.mode);
        *new_state.selected_mut() = Some(i);
    }

    pub fn previous_mode(&mut self) {
        self.save_state();
        let old_state = self.get_state_mut(self.mode);
        *old_state.selected_mut() = None;
        self.mode.previous();
        let i = self.last_selection[self.mode as usize];
        let new_state = self.get_state_mut(self.mode);
        *new_state.selected_mut() = Some(i);
    }
}

impl Summary {
    fn borders(&self, _selected: bool) -> Borders {
        Borders::NONE
    }

    fn new_crates(&self) -> List<'static> {
        let selected = self.mode.is_new_crates();
        let borders = self.borders(selected);
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.summary_data
                    .as_ref()
                    .unwrap()
                    .new_crates
                    .iter()
                    .map(|item| {
                        Text::from(vec![
                            Line::styled(item.name.clone(), config::get().color.base05),
                            Line::raw(""),
                        ])
                    }),
            )
            .collect_vec();
        list_builder(items, "New Crates", selected, borders)
    }

    fn most_downloaded(&self) -> List<'static> {
        let selected = self.mode.is_most_downloaded();
        let borders = self.borders(selected);
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.summary_data
                    .as_ref()
                    .unwrap()
                    .most_downloaded
                    .iter()
                    .map(|item| {
                        Text::from(vec![
                            Line::styled(item.name.clone(), config::get().color.base05),
                            Line::raw(""),
                        ])
                    }),
            )
            .collect_vec();
        list_builder(items, "Most Downloaded", selected, borders)
    }

    fn just_updated(&self) -> List<'static> {
        let selected = self.mode.is_just_updated();
        let borders = self.borders(selected);
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.summary_data
                    .as_ref()
                    .unwrap()
                    .just_updated
                    .iter()
                    .map(|item| {
                        Text::from(vec![
                            Line::from(vec![
                                item.name.clone().fg(config::get().color.base05),
                                " ".into(),
                                Span::styled(
                                    format!("v{}", item.max_version),
                                    Style::default().fg(config::get().color.base05),
                                ),
                            ]),
                            Line::raw(""),
                        ])
                    }),
            )
            .collect_vec();
        list_builder(items, "Just Updated", selected, borders)
    }

    fn most_recently_downloaded(&self) -> List<'static> {
        let selected = self.mode.is_most_recently_downloaded();
        let borders = self.borders(selected);
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.summary_data
                    .as_ref()
                    .unwrap()
                    .most_recently_downloaded
                    .iter()
                    .map(|item| {
                        Text::from(vec![
                            Line::styled(item.name.clone(), config::get().color.base05),
                            Line::raw(""),
                        ])
                    }),
            )
            .collect_vec();
        list_builder(items, "Most Recently Downloaded", selected, borders)
    }

    fn popular_keywords(&self) -> List<'static> {
        let selected = self.mode.is_popular_keywords();
        let borders = self.borders(selected);
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.summary_data
                    .as_ref()
                    .unwrap()
                    .popular_keywords
                    .iter()
                    .map(|item| {
                        Text::from(vec![
                            Line::styled(item.keyword.clone(), config::get().color.base05),
                            Line::raw(""),
                        ])
                    }),
            )
            .collect_vec();
        list_builder(items, "Popular Keywords", selected, borders)
    }

    fn popular_categories(&self) -> List<'static> {
        let selected = self.mode.is_popular_categories();
        let borders = self.borders(selected);
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.summary_data
                    .as_ref()
                    .unwrap()
                    .popular_categories
                    .iter()
                    .map(|item| {
                        Text::from(vec![
                            Line::styled(item.category.clone(), config::get().color.base05),
                            Line::raw(""),
                        ])
                    }),
            )
            .collect_vec();
        list_builder(items, "Popular Categories", selected, borders)
    }
}

fn list_builder<'a>(
    items: Vec<Text<'a>>,
    title: &'a str,
    selected: bool,
    borders: Borders,
) -> List<'a> {
    let title_style = if selected {
        Style::default()
            .fg(config::get().color.base00)
            .bg(config::get().color.base0a)
            .bold()
    } else {
        Style::default().fg(config::get().color.base0d).bold()
    };
    List::new(items)
        .block(
            Block::default()
                .borders(borders)
                .title(Line::from(vec![" ".into(), title.into(), " ".into()]))
                .title_style(title_style)
                .title_alignment(Alignment::Left),
        )
        .highlight_symbol(HIGHLIGHT_SYMBOL)
        .highlight_style(config::get().color.base05)
        .highlight_spacing(HighlightSpacing::Always)
}

pub struct SummaryWidget;

impl SummaryWidget {
    fn render_list(
        &self,
        area: Rect,
        buf: &mut Buffer,
        list: List,
        mode: SummaryMode,
        state: &mut Summary,
    ) {
        *(state.get_state_mut(mode).selected_mut()) = state
            .selected(mode)
            .map(|i| i.min(list.len().saturating_sub(1)));
        StatefulWidget::render(list, area, buf, state.get_state_mut(mode));
    }
}

impl StatefulWidget for &SummaryWidget {
    type State = Summary;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if state.summary_data.is_none() {
            return;
        }
        use Constraint::*;

        let [_, area] = Layout::vertical([Min(0), Percentage(90)]).areas(area);

        let [_, area, _] = Layout::horizontal([Min(0), Percentage(85), Min(0)]).areas(area);

        let [top, bottom] = Layout::vertical([Percentage(50), Percentage(50)]).areas(area);

        let [new_crates, most_downloaded, just_updated] =
            Layout::horizontal([Percentage(30), Percentage(30), Percentage(30)])
                .flex(Flex::Center)
                .spacing(2)
                .areas(top);

        let list = state.new_crates();
        self.render_list(new_crates, buf, list, SummaryMode::NewCrates, state);

        let list = state.most_downloaded();
        self.render_list(
            most_downloaded,
            buf,
            list,
            SummaryMode::MostDownloaded,
            state,
        );

        let list = state.just_updated();
        self.render_list(just_updated, buf, list, SummaryMode::JustUpdated, state);

        let [most_recently_downloaded, popular_keywords, popular_categories] =
            Layout::horizontal([Percentage(30), Percentage(30), Percentage(30)])
                .flex(Flex::Center)
                .spacing(2)
                .areas(bottom);

        let list = state.most_recently_downloaded();
        self.render_list(
            most_recently_downloaded,
            buf,
            list,
            SummaryMode::MostRecentlyDownloaded,
            state,
        );

        let list = state.popular_categories();
        self.render_list(
            popular_categories,
            buf,
            list,
            SummaryMode::PopularCategories,
            state,
        );

        let list = state.popular_keywords();
        self.render_list(
            popular_keywords,
            buf,
            list,
            SummaryMode::PopularKeywords,
            state,
        );
    }
}
