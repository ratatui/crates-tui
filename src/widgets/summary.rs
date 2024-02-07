use derive_deref::Deref;
use itertools::Itertools;
use ratatui::{layout::Flex, prelude::*, widgets::*};
use strum::{Display, EnumIs, EnumIter, FromRepr};

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
}

#[derive(Default, Debug, Clone)]
pub struct Summary {
    pub state: [ListState; 6],
    pub last_selection: [usize; 6],
    pub mode: SummaryMode,
}

impl Summary {
    pub fn mode(&self) -> SummaryMode {
        self.mode.clone()
    }

    pub fn get_state_mut(&mut self, mode: SummaryMode) -> &mut ListState {
        &mut self.state[mode as usize]
    }

    pub fn get_state(&self, mode: SummaryMode) -> &ListState {
        &self.state[mode as usize]
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

#[derive(Deref)]
pub struct SummaryWidget<'a>(pub &'a crates_io_api::Summary);

const HIGHLIGHT_SYMBOL: &str = "█";

impl<'a> SummaryWidget<'a> {
    fn new_crates(&self, selected: bool) -> List {
        let borders = if selected {
            Borders::NONE
        } else {
            Borders::NONE
        };
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(self.new_crates.iter().map(|item| {
                Text::from(vec![
                    Line::raw(item.name.clone()),
                    Line::styled(
                        item.max_version.clone(),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Line::raw(""),
                ])
            }))
            .collect_vec();
        list_builder(items, "New Crates".into(), borders)
    }

    fn most_downloaded(&self, selected: bool) -> List {
        let borders = if selected {
            Borders::NONE
        } else {
            Borders::NONE
        };
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.most_downloaded
                    .iter()
                    .map(|item| Text::from(vec![Line::raw(item.name.clone()), Line::raw("")])),
            )
            .collect_vec();
        list_builder(items, "Most Downloaded".into(), borders)
    }

    fn just_updated(&self, selected: bool) -> List {
        let borders = if selected {
            Borders::NONE
        } else {
            Borders::NONE
        };
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(self.just_updated.iter().map(|item| {
                Text::from(vec![
                    Line::raw(item.name.clone()),
                    Line::styled(
                        item.max_version.clone(),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Line::raw(""),
                ])
            }))
            .collect_vec();
        list_builder(items, "Just Updated".into(), borders)
    }

    fn most_recently_downloaded(&self, selected: bool) -> List {
        let borders = if selected {
            Borders::NONE
        } else {
            Borders::NONE
        };
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.most_recently_downloaded
                    .iter()
                    .map(|item| Text::from(vec![Line::raw(item.name.clone()), Line::raw("")])),
            )
            .collect_vec();
        list_builder(items, "Most Recently Downloaded".into(), borders)
    }

    fn popular_keywords(&self, selected: bool) -> List {
        let borders = if selected {
            Borders::NONE
        } else {
            Borders::NONE
        };
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.popular_keywords
                    .iter()
                    .map(|item| Text::from(vec![Line::raw(item.keyword.clone()), Line::raw("")])),
            )
            .collect_vec();
        list_builder(items, "Popular Keywords".into(), borders)
    }

    fn popular_categories(&self, selected: bool) -> List {
        let borders = if selected {
            Borders::NONE
        } else {
            Borders::NONE
        };
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.popular_categories
                    .iter()
                    .map(|item| Text::from(vec![Line::raw(item.category.clone()), Line::raw("")])),
            )
            .collect_vec();
        list_builder(items, "Popular Categories".into(), borders)
    }

    fn render_list(
        &self,
        area: Rect,
        buf: &mut Buffer,
        list: List,
        mode: SummaryMode,
        state: &mut Summary,
    ) {
        *(state.get_state_mut(mode).selected_mut()) = state
            .get_state(mode)
            .selected()
            .map(|i| i.min(list.len().saturating_sub(1)).max(1));
        StatefulWidget::render(list, area, buf, state.get_state_mut(mode));
    }
}

fn list_builder(items: Vec<Text>, title: String, borders: Borders) -> List {
    List::new(items)
        .block(
            Block::default()
                .borders(borders)
                .title(format!("   {title}").bold())
                .title_alignment(Alignment::Left),
        )
        .highlight_symbol(HIGHLIGHT_SYMBOL)
        .highlight_spacing(HighlightSpacing::Always)
}

impl StatefulWidget for &SummaryWidget<'_> {
    type State = Summary;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        use Constraint::*;

        let [_, area] = Layout::vertical([Min(0), Percentage(90)]).areas(area);

        let [_, area, _] = Layout::horizontal([Min(0), Percentage(75), Min(0)]).areas(area);

        let [top, bottom] = Layout::vertical([Percentage(50), Percentage(50)]).areas(area);

        let [new_crates, most_downloaded, just_updated] =
            Layout::horizontal([Percentage(30), Percentage(30), Percentage(30)])
                .flex(Flex::Center)
                .spacing(2)
                .areas(top);

        let list = self.new_crates(state.mode.is_new_crates());
        self.render_list(new_crates, buf, list, SummaryMode::NewCrates, state);

        let list = self.most_downloaded(state.mode.is_most_downloaded());
        self.render_list(
            most_downloaded,
            buf,
            list,
            SummaryMode::MostDownloaded,
            state,
        );

        let list = self.just_updated(state.mode.is_just_updated());
        self.render_list(just_updated, buf, list, SummaryMode::JustUpdated, state);

        let [most_recently_downloaded, popular_keywords, popular_categories] =
            Layout::horizontal([Percentage(30), Percentage(30), Percentage(30)])
                .flex(Flex::Center)
                .spacing(2)
                .areas(bottom);

        let list = self.most_recently_downloaded(state.mode.is_most_recently_downloaded());
        self.render_list(
            most_recently_downloaded,
            buf,
            list,
            SummaryMode::MostRecentlyDownloaded,
            state,
        );

        let list = self.popular_categories(state.mode.is_popular_categories());
        self.render_list(
            popular_categories,
            buf,
            list,
            SummaryMode::PopularCategories,
            state,
        );

        let list = self.popular_keywords(state.mode.is_popular_keywords());
        self.render_list(
            popular_keywords,
            buf,
            list,
            SummaryMode::PopularKeywords,
            state,
        );
    }
}
