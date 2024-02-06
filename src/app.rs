use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use itertools::Itertools;
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use strum::Display;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tracing::{debug, error, info};
use tui_input::backend::crossterm::EventHandler;

use crate::{
    action::Action,
    config, crates_io_api_helper,
    serde_helper::keybindings::key_event_to_string,
    tui::{self, Tui},
    widgets::{
        crate_info_table::CrateInfoTableWidget,
        popup_message::PopupMessageWidget,
        search_filter_prompt::{SearchFilterPrompt, SearchFilterPromptWidget},
        search_results_table::{SearchResultsTable, SearchResultsTableWidget},
    },
};

#[derive(Default, Debug, Display, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Search,
    Filter,
    Picker,
    Popup,
    Quit,
}

struct AppWidget;

#[derive(Debug)]
pub struct App {
    /// Receiver end of an asynchronous channel for actions that the app needs
    /// to process.
    rx: UnboundedReceiver<Action>,

    /// Sender end of an asynchronous channel for dispatching actions from
    /// various parts of the app to be handled by the event loop.
    tx: UnboundedSender<Action>,

    /// The current page number being displayed or interacted with in the UI.
    page: u64,

    /// The number of crates displayed per page in the UI.
    page_size: u64,

    /// Sort preference for search results
    sort: crates_io_api::Sort,

    /// A thread-safe indicator of whether data is currently being loaded,
    /// allowing different parts of the app to know if it's in a loading state.
    loading_status: Arc<AtomicBool>,

    /// A thread-safe, shared vector holding the list of crates fetched from
    /// crates.io, wrapped in a mutex to control concurrent access.
    crates: Arc<Mutex<Vec<crates_io_api::Crate>>>,

    /// A thread-safe, shared vector holding the list of version fetched from
    /// crates.io, wrapped in a mutex to control concurrent access.
    versions: Arc<Mutex<Vec<crates_io_api::Version>>>,

    /// A thread-safe shared container holding the detailed information about
    /// the currently selected crate; this can be `None` if no crate is
    /// selected.
    crate_info: Arc<Mutex<Option<crates_io_api::FullCrate>>>,

    last_task_details_handle: HashMap<uuid::Uuid, JoinHandle<()>>,

    /// The total number of crates fetchable from crates.io, which may not be
    /// known initially and can be used for UI elements like pagination.
    total_num_crates: Option<u64>,

    /// A string for the current search input by the user, submitted to
    /// crates.io as a query
    search: String,

    /// A string for the current filter input by the user, used only locally
    /// for filtering for the list of crates in the current view.
    filter: String,

    /// An input handler component for managing raw user input into textual
    /// form.
    input: tui_input::Input,

    /// A table component designed to handle the listing and selection of crates
    /// within the terminal UI.
    search_results: SearchResultsTable,

    /// A boolean flag that determines whether detailed crate information should
    /// be displayed within the UI.
    show_crate_info: bool,

    /// An optional error message that, when set, should be shown to the user,
    /// in the form of a popup.
    error_message: Option<String>,

    /// An optional info message that, when set, should be shown to the user, in
    /// the form of a popup.
    info_message: Option<String>,

    /// Current scroll index used for navigating through scrollable content in a
    /// popup.
    popup_scroll_index: usize,

    /// The active mode of the application, which could change how user inputs
    /// and commands are interpreted.
    mode: Mode,

    /// A prompt displaying the current search or filter query, if any, that the
    /// user can interact with.
    prompt: SearchFilterPrompt,

    /// A list of key events that have been held since the last tick, useful for
    /// interpreting sequences of key presses.
    last_tick_key_events: Vec<KeyEvent>,

    /// frame counter
    frame_count: usize,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            rx,
            tx,
            page: 1,
            page_size: 25,
            sort: crates_io_api::Sort::Relevance,
            mode: Mode::default(),
            loading_status: Default::default(),
            search: Default::default(),
            filter: Default::default(),
            crates: Default::default(),
            versions: Default::default(),
            crate_info: Default::default(),
            last_task_details_handle: Default::default(),
            total_num_crates: Default::default(),
            input: Default::default(),
            search_results: Default::default(),
            show_crate_info: Default::default(),
            error_message: Default::default(),
            info_message: Default::default(),
            popup_scroll_index: Default::default(),
            prompt: Default::default(),
            last_tick_key_events: Default::default(),
            frame_count: Default::default(),
        }
    }

    /// The main 'run' function delegates to the two functions below,
    /// to handle TUI events and App actions respectively.
    pub async fn run(&mut self, mut tui: Tui) -> Result<()> {
        tui.enter()?;
        loop {
            if let Some(e) = tui.next().await {
                self.handle_tui_event(e)?.map(|action| self.tx.send(action));
            }
            while let Ok(action) = self.rx.try_recv() {
                self.handle_action(action.clone(), &mut tui)?
                    .map(|action| self.tx.send(action));
            }
            if self.should_quit() {
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    /// Handles an event by producing an optional `Action` that the application
    /// should perform in response.
    ///
    /// This method maps incoming events from the terminal user interface to
    /// specific `Action` that represents tasks or operations the
    /// application needs to carry out.
    fn handle_tui_event(&mut self, e: tui::Event) -> Result<Option<Action>> {
        let maybe_action = match e {
            tui::Event::Quit => Some(Action::Quit),
            tui::Event::Tick => Some(Action::Tick),
            tui::Event::KeyRefresh => Some(Action::KeyRefresh),
            tui::Event::Render => Some(Action::Render),
            tui::Event::Resize(x, y) => Some(Action::Resize(x, y)),
            tui::Event::Key(key) => {
                debug!("Received key {:?}", key);
                self.forward_key_events(key)?;
                self.handle_key_events_from_config(key)
            }
            _ => None,
        };
        Ok(maybe_action)
    }

    /// Processes key events depending on the current mode
    ///
    /// This function forwards events to input prompt handler
    fn forward_key_events(&mut self, key: KeyEvent) -> Result<()> {
        match self.mode {
            Mode::Search => {
                self.input.handle_event(&crossterm::event::Event::Key(key));
            }
            Mode::Filter => {
                self.input.handle_event(&crossterm::event::Event::Key(key));
                self.tx.send(Action::HandleFilterPromptChange)?
            }
            _ => (),
        };
        Ok(())
    }

    /// Evaluates a sequence of key events against user-configured key bindings
    /// to determine if an `Action` should be triggered.
    ///
    /// This method supports user-configurable key sequences by collecting key
    /// events over time and then translating them into actions according to the
    /// current mode.
    fn handle_key_events_from_config(&mut self, key: KeyEvent) -> Option<Action> {
        self.last_tick_key_events.push(key);
        let config = config::get();
        let action = config
            .key_bindings
            .event_to_action(&self.mode, &self.last_tick_key_events);
        if action.is_some() {
            self.last_tick_key_events.drain(..);
        }
        action
    }

    /// Performs the `Action` by calling on a respective app method.
    ///
    /// `Action`'s represent a reified method call on the `App` instance.
    ///
    /// Upon receiving an action, this function updates the application state,
    /// performs necessary operations like drawing or resizing the view, or
    /// changing the mode. Actions that affect the navigation within the
    /// application, are also handled. Certain actions generate a follow-up
    /// action which will be to be processed in the next iteration of the main
    /// event loop.
    fn handle_action(&mut self, action: Action, tui: &mut Tui) -> Result<Option<Action>> {
        if action != Action::Tick && action != Action::Render {
            info!("{action:?}");
        }
        match action {
            Action::Quit => self.quit(),
            Action::Render => self.draw(tui)?,
            Action::KeyRefresh => self.key_refresh_tick(),
            Action::Resize(w, h) => self.resize(tui, (w, h))?,
            Action::Tick => self.tick(),
            Action::StoreTotalNumberOfCrates(n) => self.store_total_number_of_crates(n),
            Action::ScrollUp if self.mode == Mode::Popup => self.popup_scroll_previous(),
            Action::ScrollDown if self.mode == Mode::Popup => self.popup_scroll_next(),
            Action::ScrollUp => self.search_results.scroll_previous(1),
            Action::ScrollDown => self.search_results.scroll_next(1),
            Action::ScrollTop => self.search_results.scroll_to_top(),
            Action::ScrollBottom => self.search_results.scroll_to_bottom(),
            Action::ReloadData => self.reload_data(),
            Action::IncrementPage => self.increment_page(),
            Action::DecrementPage => self.decrement_page(),
            Action::EnterSearchInsertMode => self.enter_search_insert_mode(),
            Action::EnterFilterInsertMode => self.enter_filter_insert_mode(),
            Action::HandleFilterPromptChange => self.handle_filter_prompt_change(),
            Action::EnterNormal => self.enter_normal_mode(),
            Action::SubmitSearch => self.submit_search(),
            Action::ToggleShowCrateInfo => self.toggle_show_crate_info(),
            Action::UpdateCurrentSelectionCrateInfo => self.update_current_selection_crate_info(),
            Action::ShowErrorPopup(ref err) => self.set_error_flag(err.clone()),
            Action::ShowInfoPopup(ref info) => self.set_info_flag(info.clone()),
            Action::ClosePopup => self.clear_error_and_info_flags(),
            Action::ToggleSortByAndReload => self.toggle_sort_by_and_reload()?,
            Action::ToggleSortBy => self.toggle_sort_by()?,
            Action::ClearTaskDetailsHandle(ref id) => {
                self.clear_task_details_handle(uuid::Uuid::parse_str(&id)?)?
            }
            _ => {}
        }
        let maybe_action = match action {
            Action::ScrollUp | Action::ScrollDown | Action::ScrollTop | Action::ScrollBottom => {
                Some(Action::UpdateCurrentSelectionCrateInfo)
            }
            Action::SubmitSearch => Some(Action::ReloadData),
            _ => None,
        };
        Ok(maybe_action)
    }

    fn clear_task_details_handle(&mut self, id: uuid::Uuid) -> Result<()> {
        if let Some((_, handle)) = self.last_task_details_handle.remove_entry(&id) {
            handle.abort()
        }
        Ok(())
    }

    fn clear_all_previous_task_details_handles(&mut self) {
        *self.crate_info.lock().unwrap() = None;
        for (_, v) in self.last_task_details_handle.iter() {
            v.abort()
        }
        self.last_task_details_handle.clear()
    }
}

impl App {
    fn tick(&mut self) {
        self.update_crate_table();
    }

    fn update_crate_table(&mut self) {
        self.search_results
            .content_length(self.search_results.crates.len());

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
        self.search_results.crates = crates;
    }

    fn key_refresh_tick(&mut self) {
        self.last_tick_key_events.drain(..);
    }

    fn resize(&mut self, tui: &mut Tui, (w, h): (u16, u16)) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.tx.send(Action::Render)?;
        Ok(())
    }

    fn should_quit(&self) -> bool {
        self.mode == Mode::Quit
    }

    fn quit(&mut self) {
        self.mode = Mode::Quit
    }

    fn increment_page(&mut self) {
        if let Some(n) = self.total_num_crates {
            let max_page_size = (n / self.page_size) + 1;
            if self.page < max_page_size {
                self.page = self.page.saturating_add(1).min(max_page_size);
                self.reload_data();
            }
        }
    }

    fn decrement_page(&mut self) {
        let min_page_size = 1;
        if self.page > min_page_size {
            self.page = self.page.saturating_sub(1).max(min_page_size);
            self.reload_data();
        }
    }

    fn popup_scroll_previous(&mut self) {
        self.popup_scroll_index = self.popup_scroll_index.saturating_sub(1)
    }

    fn popup_scroll_next(&mut self) {
        self.popup_scroll_index = self.popup_scroll_index.saturating_add(1)
    }

    fn enter_search_insert_mode(&mut self) {
        self.mode = Mode::Search;
        self.input = self.input.clone().with_value(self.search.clone());
    }

    fn enter_filter_insert_mode(&mut self) {
        self.show_crate_info = false;
        self.mode = Mode::Filter;
        self.input = self.input.clone().with_value(self.filter.clone());
    }

    fn enter_normal_mode(&mut self) {
        self.mode = Mode::Picker;
        if !self.search_results.crates.is_empty() && self.search_results.selected().is_none() {
            self.search_results.select(Some(0))
        }
    }

    fn handle_filter_prompt_change(&mut self) {
        self.filter = self.input.value().into();
        self.search_results.select(None);
    }

    fn submit_search(&mut self) {
        self.clear_all_previous_task_details_handles();
        self.show_crate_info = false;
        self.mode = Mode::Picker;
        self.filter.clear();
        self.search = self.input.value().into();
    }

    fn toggle_show_crate_info(&mut self) {
        self.show_crate_info = !self.show_crate_info;
        if self.show_crate_info {
            self.request_crate_details()
        } else {
            self.clear_all_previous_task_details_handles();
        }
    }

    fn toggle_sort_by(&mut self) -> Result<()> {
        use crates_io_api::Sort as S;
        self.sort = match self.sort {
            S::Alphabetical => S::Relevance,
            S::Relevance => S::Downloads,
            S::Downloads => S::RecentDownloads,
            S::RecentDownloads => S::RecentUpdates,
            S::RecentUpdates => S::NewlyAdded,
            S::NewlyAdded => S::Alphabetical,
        };
        Ok(())
    }

    fn toggle_sort_by_and_reload(&mut self) -> Result<()> {
        self.toggle_sort_by()?;
        self.tx.send(Action::ReloadData)?;
        Ok(())
    }

    fn set_error_flag(&mut self, err: String) {
        error!("Error: {err}");
        self.error_message = Some(err);
        self.mode = Mode::Popup;
    }

    fn set_info_flag(&mut self, info: String) {
        info!("Info: {info}");
        self.info_message = Some(info);
        self.mode = Mode::Popup;
    }

    fn clear_error_and_info_flags(&mut self) {
        self.error_message = None;
        self.info_message = None;
        self.mode = Mode::Search;
    }

    fn update_current_selection_crate_info(&mut self) {
        if self.show_crate_info {
            self.clear_all_previous_task_details_handles();
            self.request_crate_details();
        } else {
            self.clear_all_previous_task_details_handles();
        }
    }

    fn store_total_number_of_crates(&mut self, n: u64) {
        self.total_num_crates = Some(n)
    }
}

impl App {
    /// Reloads the list of crates based on the current search parameters,
    /// updating the application state accordingly. This involves fetching
    /// data asynchronously from the crates.io API and updating various parts of
    /// the application state, such as the crates listing, current crate
    /// info, and loading status.
    fn reload_data(&mut self) {
        self.prepare_reload();
        let search_params = self.create_search_parameters();
        self.request_crates(search_params);
    }

    /// Clears current search results and resets the UI to prepare for new data.
    fn prepare_reload(&mut self) {
        self.search_results.select(None);
        *self.crate_info.lock().unwrap() = None;
    }

    /// Creates the parameters required for the search task.
    fn create_search_parameters(&self) -> crates_io_api_helper::SearchParameters {
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
    fn request_crates(&self, params: crates_io_api_helper::SearchParameters) {
        tokio::spawn(async move {
            params.loading_status.store(true, Ordering::SeqCst);
            if let Err(error_message) = crates_io_api_helper::request_crates(&params).await {
                let _ = params.tx.send(Action::ShowErrorPopup(error_message));
            }
            params.loading_status.store(false, Ordering::SeqCst);
        });
    }

    /// Spawns an asynchronous task to fetch crate details from crates.io based
    /// on currently selected crate
    fn request_crate_details(&mut self) {
        if self.search_results.crates.is_empty() {
            return;
        }
        if let Some(crate_name) = self.selected_crate_name() {
            let tx = self.tx.clone();
            let crate_info = self.crate_info.clone();
            let loading_status = self.loading_status.clone();

            // Spawn the async work to fetch crate details.
            let uuid = uuid::Uuid::new_v4();
            let last_task_details_handle = tokio::spawn(async move {
                info!("Requesting details for {crate_name}: {uuid}");
                loading_status.store(true, Ordering::SeqCst);
                if let Err(error_message) =
                    crates_io_api_helper::request_crate_details(&crate_name, crate_info).await
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

    // Extracts the selected crate name, if possible.
    fn selected_crate_name(&self) -> Option<String> {
        self.search_results
            .selected()
            .and_then(|index| self.search_results.crates.get(index))
            .filter(|crate_| !crate_.name.is_empty())
            .map(|crate_| crate_.name.clone())
    }

    // Render the `AppWidget` as a stateful widget using `self` as the `State`
    fn draw(&mut self, tui: &mut Tui) -> Result<()> {
        tui.draw(|frame| {
            frame.render_stateful_widget(AppWidget, frame.size(), self);
            self.update_frame_count(frame);
            self.update_cursor(frame);
        })?;
        Ok(())
    }

    // Sets the frame count
    fn update_frame_count(&mut self, frame: &mut Frame<'_>) {
        self.frame_count = frame.count();
    }

    // Sets cursor for the prompt
    fn update_cursor(&mut self, frame: &mut Frame<'_>) {
        if let Some(cursor_position) = self.prompt.cursor_position() {
            frame.set_cursor(cursor_position.x, cursor_position.y)
        }
    }

    fn render_crate_info(&self, area: Rect, buf: &mut Buffer) -> Rect {
        match self.crate_info.lock().unwrap().clone() {
            Some(ci) => {
                // split available area
                let [table, info] =
                    Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(area);
                CrateInfoTableWidget::new(ci).render(info, buf);
                table
            }
            _ => area,
        }
    }

    fn events_widget(&self) -> Option<Block> {
        if self.last_tick_key_events.is_empty() {
            return None;
        }

        let title = format!(
            "{:?}",
            self.last_tick_key_events
                .iter()
                .map(key_event_to_string)
                .collect::<Vec<_>>()
        );
        Some(
            Block::default()
                .title(title)
                .title_position(ratatui::widgets::block::Position::Bottom)
                .title_alignment(ratatui::layout::Alignment::Right),
        )
    }

    fn selected_with_page_context(&self) -> u64 {
        self.search_results.selected().map_or(0, |n| {
            (self.page.saturating_sub(1) * self.page_size) + n as u64 + 1
        })
    }

    fn loading(&self) -> bool {
        self.loading_status.load(Ordering::SeqCst)
    }

    fn search_results_status(&self) -> String {
        let selected = self.selected_with_page_context();
        let ncrates = self.total_num_crates.unwrap_or_default();
        if self.loading() {
            self.spinner()
        } else {
            format!("{}/{}", selected, ncrates)
        }
    }

    fn focused(&self) -> bool {
        if self.mode == Mode::Search || self.mode == Mode::Filter {
            true
        } else {
            false
        }
    }

    fn spinner(&self) -> String {
        let spinner = ["◑", "◒", "◐", "◓"];
        let index = self.frame_count % spinner.len();
        let symbol = spinner[index];
        symbol.into()
    }
}

impl StatefulWidget for AppWidget {
    type State = App;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        use Constraint::*;
        Block::default()
            .bg(config::get().style.background_color)
            .render(area, buf);

        let [table, prompt] = if state.focused() {
            Layout::vertical([Constraint::Fill(0), Constraint::Length(5)]).areas(area)
        } else {
            Layout::vertical([Constraint::Fill(0), Constraint::Length(1)]).areas(area)
        };

        let p = SearchFilterPromptWidget::new(state.focused(), state.mode, &state.input);
        p.render(prompt, buf, &mut state.prompt);

        let [_, meta] = Layout::horizontal([Constraint::Percentage(75), Fill(0)]).areas(prompt);

        Paragraph::new(Line::from(vec![
            "Sort By: ".into(),
            format!("{:?}", state.sort.clone()).red(),
        ]))
        .centered()
        .render(
            meta.inner(&Margin {
                horizontal: 0,
                vertical: 2,
            }),
            buf,
        );

        let remaining_table = if state.show_crate_info {
            state.render_crate_info(table, buf)
        } else {
            table
        };

        SearchResultsTableWidget::new(state.mode == Mode::Picker).render(
            remaining_table,
            buf,
            &mut state.search_results,
        );

        Line::from(state.search_results_status())
            .right_aligned()
            .render(
                remaining_table.inner(&Margin {
                    horizontal: 0,
                    vertical: 1,
                }),
                buf,
            );

        if let Some(err) = &state.error_message {
            PopupMessageWidget::new("Error", err, state.popup_scroll_index).render(area, buf);
        }
        if let Some(info) = &state.info_message {
            PopupMessageWidget::new("Info", info, state.popup_scroll_index).render(area, buf);
        }

        state.events_widget().render(area, buf);
    }
}
