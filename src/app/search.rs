use std::sync::{Arc, Mutex};

use crossterm::event::{Event as CrosstermEvent, KeyEvent};
use itertools::Itertools;
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{action::Action, widgets::search_results_table::SearchResultsTable};

#[derive(Debug)]
pub struct Search {
    /// A string for the current search input by the user, submitted to
    /// crates.io as a query
    pub search: String,

    /// A string for the current filter input by the user, used only locally
    /// for filtering for the list of crates in the current view.
    pub filter: String,

    /// A table component designed to handle the listing and selection of crates
    /// within the terminal UI.
    pub search_results: SearchResultsTable,

    /// An input handler component for managing raw user input into textual
    /// form.
    pub input: tui_input::Input,
}

impl Search {
    pub fn new() -> Self {
        Self {
            search: String::new(),
            filter: String::new(),
            search_results: SearchResultsTable::default(),
            input: Input::default(),
        }
    }

    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::ScrollTop => self.search_results.scroll_to_top(),
            Action::ScrollBottom => self.search_results.scroll_to_bottom(),
            Action::ScrollSearchResultsUp => self.scroll_up(),
            Action::ScrollSearchResultsDown => self.scroll_down(),
            _ => {}
        }
    }

    pub fn update_search_table_results(&mut self, crates: Arc<Mutex<Vec<crates_io_api::Crate>>>) {
        self.search_results
            .content_length(self.search_results.crates.len());

        let filter = self.filter.clone();
        let filter_words = filter.split_whitespace().collect::<Vec<_>>();

        let crates: Vec<_> = crates
            .lock()
            .unwrap()
            .iter()
            .filter(|c| {
                filter_words.iter().all(|word| {
                    c.name.to_lowercase().contains(word)
                        || c.description
                            .clone()
                            .unwrap_or_default()
                            .to_lowercase()
                            .contains(word)
                })
            })
            .cloned()
            .collect_vec();
        self.search_results.crates = crates;
    }

    pub fn scroll_up(&mut self) {
        self.search_results.scroll_previous(1);
    }

    pub fn scroll_down(&mut self) {
        self.search_results.scroll_next(1);
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        self.input.handle_event(&CrosstermEvent::Key(key));
    }

    pub fn handle_filter_prompt_change(&mut self) {
        self.filter = self.input.value().into();
        self.search_results.select(None);
    }
}
