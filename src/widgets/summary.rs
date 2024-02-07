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
    pub just_updated: ListState,
    pub most_downloaded: ListState,
    pub new_crates: ListState,
    pub most_recently_downloaded: ListState,
    pub popular_categories: ListState,
    pub popular_keywords: ListState,
    pub mode: SummaryMode,
}

impl Summary {
    pub fn mode(&self) -> SummaryMode {
        self.mode.clone()
    }

    pub fn get_state_mut(&mut self) -> &mut ListState {
        use SummaryMode as M;
        match self.mode {
            M::JustUpdated => &mut self.just_updated,
            M::MostDownloaded => &mut self.most_downloaded,
            M::NewCrates => &mut self.new_crates,
            M::MostRecentlyDownloaded => &mut self.most_recently_downloaded,
            M::PopularCategories => &mut self.popular_categories,
            M::PopularKeywords => &mut self.popular_keywords,
        }
    }

    pub fn scroll_previous(&mut self) {
        let state = self.get_state_mut();
        let i = state.selected().map_or(0, |i| i.saturating_sub(1));
        state.select(Some(i));
    }

    pub fn scroll_next(&mut self) {
        let state = self.get_state_mut();
        let i = state.selected().map_or(0, |i| i.saturating_add(1));
        state.select(Some(i));
    }

    pub fn next_mode(&mut self) {
        let old_state = self.get_state_mut();
        *old_state.selected_mut() = None;
        self.mode.next();
        let new_state = self.get_state_mut();
        *new_state.selected_mut() = Some(0);
    }

    pub fn previous_mode(&mut self) {
        let old_state = self.get_state_mut();
        *old_state.selected_mut() = None;
        self.mode.previous();
        let new_state = self.get_state_mut();
        *new_state.selected_mut() = Some(0);
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
        List::new(items)
            .block(
                Block::default()
                    .borders(borders)
                    .title("New Crates".bold())
                    .title_alignment(Alignment::Left),
            )
            .highlight_symbol(HIGHLIGHT_SYMBOL)
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
        List::new(items)
            .block(
                Block::default()
                    .borders(borders)
                    .title("Most Downloaded".bold())
                    .title_alignment(Alignment::Left),
            )
            .highlight_symbol(HIGHLIGHT_SYMBOL)
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
        List::new(items)
            .block(
                Block::default()
                    .borders(borders)
                    .title("Just Updated".bold())
                    .title_alignment(Alignment::Left),
            )
            .highlight_symbol(HIGHLIGHT_SYMBOL)
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
        List::new(items)
            .block(
                Block::default()
                    .borders(borders)
                    .title("Most Recent Downloads".bold())
                    .title_alignment(Alignment::Left),
            )
            .highlight_symbol(HIGHLIGHT_SYMBOL)
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
        List::new(items)
            .block(
                Block::default()
                    .borders(borders)
                    .title("Popular Keywords".bold())
                    .title_alignment(Alignment::Left),
            )
            .highlight_symbol(HIGHLIGHT_SYMBOL)
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
        List::new(items)
            .block(
                Block::default()
                    .borders(borders)
                    .title("Popular Categories".bold())
                    .title_alignment(Alignment::Left),
            )
            .highlight_symbol(HIGHLIGHT_SYMBOL)
    }
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
        *(state.new_crates.selected_mut()) = state
            .new_crates
            .selected()
            .map(|i| i.min(list.len().saturating_sub(1)).max(1));
        StatefulWidget::render(list, new_crates, buf, &mut state.new_crates);

        let list = self.most_downloaded(state.mode.is_most_downloaded());
        *(state.most_downloaded.selected_mut()) = state
            .most_downloaded
            .selected()
            .map(|i| i.min(list.len().saturating_sub(1)).max(1));
        StatefulWidget::render(list, most_downloaded, buf, &mut state.most_downloaded);

        let list = self.just_updated(state.mode.is_just_updated());
        *(state.just_updated.selected_mut()) = state
            .just_updated
            .selected()
            .map(|i| i.min(list.len().saturating_sub(1)).max(1));
        StatefulWidget::render(list, just_updated, buf, &mut state.just_updated);

        let [most_recently_downloaded, popular_keywords, popular_categories] =
            Layout::horizontal([Percentage(30), Percentage(30), Percentage(30)])
                .flex(Flex::Center)
                .spacing(2)
                .areas(bottom);

        let list = self.most_recently_downloaded(state.mode.is_most_recently_downloaded());
        *(state.most_recently_downloaded.selected_mut()) = state
            .most_recently_downloaded
            .selected()
            .map(|i| i.min(list.len().saturating_sub(1)).max(1));
        StatefulWidget::render(
            list,
            most_recently_downloaded,
            buf,
            &mut state.most_recently_downloaded,
        );

        let list = self.popular_categories(state.mode.is_popular_categories());
        *(state.popular_categories.selected_mut()) = state
            .popular_categories
            .selected()
            .map(|i| i.min(list.len().saturating_sub(1)).max(1));
        StatefulWidget::render(list, popular_categories, buf, &mut state.popular_categories);

        let list = self.popular_keywords(state.mode.is_popular_keywords());
        *(state.popular_keywords.selected_mut()) = state
            .popular_keywords
            .selected()
            .map(|i| i.min(list.len().saturating_sub(1)).max(1));
        StatefulWidget::render(list, popular_keywords, buf, &mut state.popular_keywords);
    }
}
