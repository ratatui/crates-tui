use color_eyre::Result;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use strum::EnumIs;
use tracing::info;

use crossterm::event::{Event as CrosstermEvent, KeyEvent};
use itertools::Itertools;
use ratatui::prelude::*;
use ratatui::{layout::Position, widgets::StatefulWidget};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{
    action::Action,
    app::Mode,
    crates_io_api_helper,
    widgets::{search_filter_prompt::SearchFilterPrompt, search_results_table::SearchResultsTable},
};

use super::search_results_table::SearchResultsTableWidget;

#[derive(Debug)]
pub struct SearchPage {
    pub mode: SearchMode,

    /// A string for the current search input by the user, submitted to
    /// crates.io as a query
    pub search: String,

    /// A string for the current filter input by the user, used only locally
    /// for filtering for the list of crates in the current view.
    pub filter: String,

    /// A table component designed to handle the listing and selection of crates
    /// within the terminal UI.
    pub results: SearchResultsTable,

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

    /// Sender end of an asynchronous channel for dispatching actions from
    /// various parts of the app to be handled by the event loop.
    tx: UnboundedSender<Action>,

    /// A thread-safe indicator of whether data is currently being loaded,
    /// allowing different parts of the app to know if it's in a loading state.
    loading_status: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, EnumIs)]
pub enum SearchMode {
    #[default]
    Search,
    Filter,
    ResultsHideCrate,
    ResultsShowCrate,
}

impl SearchMode {
    pub fn is_focused(&self) -> bool {
        matches!(self, SearchMode::Search | SearchMode::Filter)
    }

    pub fn toggle_show_crate_info(&mut self) {
        *self = match self {
            SearchMode::ResultsShowCrate => SearchMode::ResultsHideCrate,
            SearchMode::ResultsHideCrate => SearchMode::ResultsShowCrate,
            _ => *self,
        };
    }

    pub fn should_show_crate_info(&self) -> bool {
        matches!(self, SearchMode::ResultsShowCrate)
    }
}

impl SearchPage {
    pub fn new(tx: UnboundedSender<Action>, loading_status: Arc<AtomicBool>) -> Self {
        Self {
            mode: Default::default(),
            search: String::new(),
            filter: String::new(),
            results: SearchResultsTable::default(),
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
            tx,
            loading_status,
        }
    }

    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::ScrollTop => self.results.scroll_to_top(),
            Action::ScrollBottom => self.results.scroll_to_bottom(),
            Action::ScrollSearchResultsUp => self.scroll_up(),
            Action::ScrollSearchResultsDown => self.scroll_down(),
            _ => {}
        }
    }

    pub fn update_search_table_results(&mut self) {
        self.results.content_length(self.results.crates.len());

        let filter = self.filter.clone();
        let filter_words = filter.split_whitespace().collect::<Vec<_>>();

        let crates: Vec<_> = self
            .crates
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
        self.results.crates = crates;
    }

    pub fn scroll_up(&mut self) {
        self.results.scroll_previous(1);
    }

    pub fn scroll_down(&mut self) {
        self.results.scroll_next(1);
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        self.input.handle_event(&CrosstermEvent::Key(key));
    }

    pub fn handle_filter_prompt_change(&mut self) {
        self.filter = self.input.value().into();
        self.results.select(None);
    }

    pub fn cursor_position(&self) -> Option<Position> {
        self.prompt.cursor_position()
    }

    pub fn increment_page(&mut self) {
        if let Some(n) = self.total_num_crates {
            let max_page_size = (n / self.page_size) + 1;
            if self.page < max_page_size {
                self.page = self.page.saturating_add(1).min(max_page_size);
                self.reload_data();
            }
        }
    }

    pub fn decrement_page(&mut self) {
        let min_page_size = 1;
        if self.page > min_page_size {
            self.page = self.page.saturating_sub(1).max(min_page_size);
            self.reload_data();
        }
    }

    pub fn clear_task_details_handle(&mut self, id: uuid::Uuid) -> Result<()> {
        if let Some((_, handle)) = self.last_task_details_handle.remove_entry(&id) {
            handle.abort()
        }
        Ok(())
    }

    pub fn is_focused(&self) -> bool {
        self.mode.is_focused()
    }

    pub fn clear_all_previous_task_details_handles(&mut self) {
        *self.full_crate_info.lock().unwrap() = None;
        for (_, v) in self.last_task_details_handle.iter() {
            v.abort()
        }
        self.last_task_details_handle.clear()
    }

    pub fn submit_query(&mut self) {
        self.clear_all_previous_task_details_handles();
        self.filter.clear();
        self.search = self.input.value().into();
    }

    /// Reloads the list of crates based on the current search parameters,
    /// updating the application state accordingly. This involves fetching
    /// data asynchronously from the crates.io API and updating various parts of
    /// the application state, such as the crates listing, current crate
    /// info, and loading status.
    pub fn reload_data(&mut self) {
        self.prepare_reload();
        let search_params = self.create_search_parameters();
        self.request_search_results(search_params);
    }

    /// Clears current search results and resets the UI to prepare for new data.
    pub fn prepare_reload(&mut self) {
        self.results.select(None);
        *self.full_crate_info.lock().unwrap() = None;
        *self.crate_response.lock().unwrap() = None;
    }

    /// Creates the parameters required for the search task.
    pub fn create_search_parameters(&self) -> crates_io_api_helper::SearchParameters {
        crates_io_api_helper::SearchParameters {
            search: self.search.clone(),
            page: self.page.clamp(1, u64::MAX),
            page_size: self.page_size,
            crates: self.crates.clone(),
            versions: self.versions.clone(),
            loading_status: self.loading_status.clone(),
            sort: self.sort.clone(),
            tx: self.tx.clone(),
        }
    }

    /// Spawns an asynchronous task to fetch crate data from crates.io.
    pub fn request_search_results(&self, params: crates_io_api_helper::SearchParameters) {
        tokio::spawn(async move {
            params.loading_status.store(true, Ordering::SeqCst);
            if let Err(error_message) = crates_io_api_helper::request_search_results(&params).await
            {
                let _ = params.tx.send(Action::ShowErrorPopup(error_message));
            }
            let _ = params.tx.send(Action::UpdateSearchTableResults);
            params.loading_status.store(false, Ordering::SeqCst);
        });
    }

    /// Spawns an asynchronous task to fetch crate details from crates.io based
    /// on currently selected crate
    pub fn request_crate_details(&mut self) {
        if self.results.crates.is_empty() {
            return;
        }
        if let Some(crate_name) = self.results.selected_crate_name() {
            let tx = self.tx.clone();
            let crate_response = self.crate_response.clone();
            let loading_status = self.loading_status.clone();

            // Spawn the async work to fetch crate details.
            let uuid = uuid::Uuid::new_v4();
            let last_task_details_handle = tokio::spawn(async move {
                info!("Requesting details for {crate_name}: {uuid}");
                loading_status.store(true, Ordering::SeqCst);
                if let Err(error_message) =
                    crates_io_api_helper::request_crate_details(&crate_name, crate_response).await
                {
                    let _ = tx.send(Action::ShowErrorPopup(error_message));
                };
                loading_status.store(false, Ordering::SeqCst);
                info!("Retrieved details for {crate_name}: {uuid}");
                let _ = tx.send(Action::ClearTaskDetailsHandle(uuid.to_string()));
            });
            self.last_task_details_handle
                .insert(uuid, last_task_details_handle);
        }
    }

    /// Spawns an asynchronous task to fetch crate details from crates.io based
    /// on currently selected crate
    pub fn request_full_crate_details(&mut self) {
        if self.results.crates.is_empty() {
            return;
        }
        if let Some(crate_name) = self.results.selected_crate_name() {
            let tx = self.tx.clone();
            let full_crate_info = self.full_crate_info.clone();
            let loading_status = self.loading_status.clone();

            // Spawn the async work to fetch crate details.
            let uuid = uuid::Uuid::new_v4();
            let last_task_details_handle = tokio::spawn(async move {
                info!("Requesting details for {crate_name}: {uuid}");
                loading_status.store(true, Ordering::SeqCst);
                if let Err(error_message) =
                    crates_io_api_helper::request_full_crate_details(&crate_name, full_crate_info)
                        .await
                {
                    let _ = tx.send(Action::ShowErrorPopup(error_message));
                };
                loading_status.store(false, Ordering::SeqCst);
                info!("Retrieved details for {crate_name}: {uuid}");
                let _ = tx.send(Action::ClearTaskDetailsHandle(uuid.to_string()));
            });
            self.last_task_details_handle
                .insert(uuid, last_task_details_handle);
        }
    }

    pub fn results_status(&self) -> String {
        let selected = self.selected_with_page_context();
        let ncrates = self.total_num_crates.unwrap_or_default();
        format!("{}/{} Results", selected, ncrates)
    }

    pub fn selected_with_page_context(&self) -> u64 {
        self.results.selected().map_or(0, |n| {
            (self.page.saturating_sub(1) * self.page_size) + n as u64 + 1
        })
    }

    pub fn page_number_status(&self) -> String {
        let max_page_size = (self.total_num_crates.unwrap_or_default() / self.page_size) + 1;
        format!("Page: {}/{}", self.page, max_page_size)
    }

    pub fn enter_normal_mode(&mut self) {
        if !self.results.crates.is_empty() && self.results.selected().is_none() {
            self.results.select(Some(0))
        }
    }

    pub fn enter_insert_mode(&mut self) {
        self.input = self.input.clone().with_value(if self.mode.is_search() {
            self.search.clone()
        } else if self.mode.is_filter() {
            self.filter.clone()
        } else {
            unreachable!("Cannot enter insert mode when mode is {:?}", self.mode)
        });
    }

    pub fn toggle_show_crate_info(&mut self) {
        self.mode.toggle_show_crate_info();
        if self.mode.should_show_crate_info() {
            self.request_crate_details()
        } else {
            self.clear_all_previous_task_details_handles();
        }
    }

    fn toggle_sort_by_forward(&mut self) {
        use crates_io_api::Sort as S;
        self.sort = match self.sort {
            S::Alphabetical => S::Relevance,
            S::Relevance => S::Downloads,
            S::Downloads => S::RecentDownloads,
            S::RecentDownloads => S::RecentUpdates,
            S::RecentUpdates => S::NewlyAdded,
            S::NewlyAdded => S::Alphabetical,
        };
    }

    fn toggle_sort_by_backward(&mut self) {
        use crates_io_api::Sort as S;
        self.sort = match self.sort {
            S::Relevance => S::Alphabetical,
            S::Downloads => S::Relevance,
            S::RecentDownloads => S::Downloads,
            S::RecentUpdates => S::RecentDownloads,
            S::NewlyAdded => S::RecentUpdates,
            S::Alphabetical => S::NewlyAdded,
        };
    }

    pub fn toggle_sort_by(&mut self, reload: bool, forward: bool) -> Result<()> {
        if forward {
            self.toggle_sort_by_forward()
        } else {
            self.toggle_sort_by_backward()
        };
        if reload {
            self.tx.send(Action::ReloadData)?;
        }
        Ok(())
    }
}

pub struct SearchPageWidget {
    pub mode: Mode,
}

impl SearchPageWidget {
    pub fn new(mode: Mode) -> Self {
        Self { mode }
    }
}

impl StatefulWidget for SearchPageWidget {
    type State = SearchPage;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        SearchResultsTableWidget::new(state.is_focused()).render(area, buf, &mut state.results);

        Line::from(state.page_number_status())
            .left_aligned()
            .render(
                area.inner(&Margin {
                    horizontal: 1,
                    vertical: 2,
                }),
                buf,
            );

        Line::from(state.results_status()).right_aligned().render(
            area.inner(&Margin {
                horizontal: 1,
                vertical: 2,
            }),
            buf,
        );
    }
}
