use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crossterm::event::{Event as CrosstermEvent, KeyEvent};
use itertools::Itertools;
use ratatui::layout::Position;
use tokio::task::JoinHandle;
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{
    action::Action,
    widgets::{search_filter_prompt::SearchFilterPrompt, search_results_table::SearchResultsTable},
};

#[derive(Debug)]
pub struct SearchPage {
    pub search_mode: SearchMode,

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

    /// A prompt displaying the current search or filter query, if any, that the
    /// user can interact with.
    pub prompt: SearchFilterPrompt,

    /// The current page number being displayed or interacted with in the UI.
    pub page: u64,

    /// The number of crates displayed per page in the UI.
    pub page_size: u64,

    /// Sort preference for search results
    pub sort: crates_io_api::Sort,

    /// The total number of crates fetchable from crates.io, which may not be
    /// known initially and can be used for UI elements like pagination.
    pub total_num_crates: Option<u64>,

    /// A thread-safe, shared vector holding the list of crates fetched from
    /// crates.io, wrapped in a mutex to control concurrent access.
    pub crates: Arc<Mutex<Vec<crates_io_api::Crate>>>,

    /// A thread-safe, shared vector holding the list of version fetched from
    /// crates.io, wrapped in a mutex to control concurrent access.
    pub versions: Arc<Mutex<Vec<crates_io_api::Version>>>,

    /// A thread-safe shared container holding the detailed information about
    /// the currently selected crate; this can be `None` if no crate is
    /// selected.
    pub full_crate_info: Arc<Mutex<Option<crates_io_api::FullCrate>>>,

    /// A thread-safe shared container holding the detailed information about
    /// the currently selected crate; this can be `None` if no crate is
    /// selected.
    pub crate_response: Arc<Mutex<Option<crates_io_api::CrateResponse>>>,

    pub last_task_details_handle: HashMap<uuid::Uuid, JoinHandle<()>>,
}

#[derive(Debug, Default)]
pub enum SearchMode {
    #[default]
    Search,
    Filter,
    ResultsHideCrate,
    ResultsShowCrate,
}

impl SearchPage {
    pub fn new() -> Self {
        Self {
            search_mode: Default::default(),
            search: String::new(),
            filter: String::new(),
            search_results: SearchResultsTable::default(),
            input: Input::default(),
            prompt: SearchFilterPrompt::default(),
            page: 1,
            page_size: 25,
            sort: crates_io_api::Sort::Relevance,
            total_num_crates: None,
            crates: Default::default(),
            versions: Default::default(),
            full_crate_info: Default::default(),
            crate_response: Default::default(),
            last_task_details_handle: Default::default(),
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

    pub fn cursor_position(&self) -> Option<Position> {
        self.prompt.cursor_position()
    }
}
