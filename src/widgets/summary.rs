use derive_deref::Deref;
use itertools::Itertools;
use ratatui::{layout::Flex, prelude::*, widgets::*};

// pub just_updated: Vec<Crate>,
// pub most_downloaded: Vec<Crate>,
// pub new_crates: Vec<Crate>,
// pub most_recently_downloaded: Vec<Crate>,
// pub num_crates: u64,
// pub num_downloads: u64,
// pub popular_categories: Vec<Category>,
// pub popular_keywords: Vec<Keyword>,
#[derive(Deref)]
pub struct SummaryWidget<'a>(pub &'a crates_io_api::Summary);

impl<'a> SummaryWidget<'a> {
    fn new_crates(&self) -> impl Widget {
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
        List::new(items).block(Block::default().title("New Crates".bold()))
    }

    fn most_downloaded(&self) -> impl Widget {
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.most_downloaded
                    .iter()
                    .map(|item| Text::from(vec![Line::raw(item.name.clone()), Line::raw("")])),
            )
            .collect_vec();
        List::new(items).block(Block::default().title("Most Downloaded".bold()))
    }

    fn just_updated(&self) -> impl Widget {
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
        List::new(items).block(Block::default().title("Just Updated".bold()))
    }

    fn most_recently_downloaded(&self) -> impl Widget {
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.most_recently_downloaded
                    .iter()
                    .map(|item| Text::from(vec![Line::raw(item.name.clone()), Line::raw("")])),
            )
            .collect_vec();
        List::new(items).block(Block::default().title("Most Recent Downloads".bold()))
    }

    fn popular_keywords(&self) -> impl Widget {
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.popular_keywords
                    .iter()
                    .map(|item| Text::from(vec![Line::raw(item.keyword.clone()), Line::raw("")])),
            )
            .collect_vec();
        List::new(items).block(Block::default().title("Popular Keywords".bold()))
    }

    fn popular_categories(&self) -> impl Widget {
        let items = std::iter::once(Text::from(Line::raw("")))
            .chain(
                self.popular_categories
                    .iter()
                    .map(|item| Text::from(vec![Line::raw(item.category.clone()), Line::raw("")])),
            )
            .collect_vec();
        List::new(items).block(Block::default().title("Popular Categories".bold()))
    }
}

impl Widget for &SummaryWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use Constraint::*;

        let [_, area] = Layout::vertical([Min(0), Percentage(90)]).areas(area);

        let [_, area, _] = Layout::horizontal([Min(0), Percentage(75), Min(0)]).areas(area);

        let [top, bottom] = Layout::vertical([Percentage(50), Percentage(50)]).areas(area);

        let [new_crates, most_downloaded, just_updated] =
            Layout::horizontal([Percentage(30), Percentage(30), Percentage(30)])
                .flex(Flex::Center)
                .spacing(2)
                .areas(top);

        Widget::render(self.new_crates(), new_crates, buf);
        Widget::render(self.most_downloaded(), most_downloaded, buf);
        Widget::render(self.just_updated(), just_updated, buf);

        let [most_recent_downloads, popular_keywords, popular_categories] =
            Layout::horizontal([Percentage(30), Percentage(30), Percentage(30)])
                .flex(Flex::Center)
                .spacing(2)
                .areas(bottom);

        Widget::render(self.most_recently_downloaded(), most_recent_downloads, buf);
        Widget::render(self.popular_keywords(), popular_keywords, buf);
        Widget::render(self.popular_categories(), popular_categories, buf);
    }
}