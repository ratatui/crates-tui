use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIs};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tracing::{debug, error, info};

use crate::{
    action::Action,
    config, crates_io_api_helper,
    events::{Event, Events},
    serde_helper::keybindings::key_event_to_string,
    tui::Tui,
    widgets::{
        crate_info_table::{CrateInfo, CrateInfoTableWidget},
        help::{Help, HelpWidget},
        popup_message::{PopupMessageState, PopupMessageWidget},
        search_filter_prompt::SearchFilterPromptWidget,
        search_results_table::SearchResultsTableWidget,
        summary::{Summary, SummaryWidget},
        tabs::SelectedTab,
    },
};

mod search_page;
use search_page::SearchPage;
mod search_prompt;

use self::search_page::SearchMode;

#[derive(
    Default, Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIs,
)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Common,
    #[default]
    Summary,
    Search,
    Filter,
    // Picker(CrateInfo), unable to make configuration file work with this
    PickerShowCrateInfo,
    PickerHideCrateInfo,
    Popup,
    Help,
    Quit,
}

impl Mode {
    pub fn focused(&self) -> bool {
        matches!(self, Mode::Search | Mode::Filter)
    }

    pub fn is_picker(&self) -> bool {
        self.is_picker_hide_crate_info() || self.is_picker_show_crate_info()
    }

    pub fn toggle_crate_info(&mut self) {
        *self = match self {
            Mode::PickerShowCrateInfo => Mode::PickerHideCrateInfo,
            Mode::PickerHideCrateInfo => Mode::PickerShowCrateInfo,
            _ => *self,
        };
    }

    pub fn should_show_crate_info(&self) -> bool {
        matches!(self, Mode::PickerShowCrateInfo)
    }
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
    full_crate_info: Arc<Mutex<Option<crates_io_api::FullCrate>>>,

    /// A thread-safe shared container holding the detailed information about
    /// the currently selected crate; this can be `None` if no crate is
    /// selected.
    crate_response: Arc<Mutex<Option<crates_io_api::CrateResponse>>>,

    /// A thread-safe shared container holding the detailed information about
    /// the currently selected crate; this can be `None` if no crate is
    /// selected.
    summary_data: Arc<Mutex<Option<crates_io_api::Summary>>>,

    /// contains list state for summary
    summary: Summary,

    /// contains table state for info popup
    crate_info: CrateInfo,

    last_task_details_handle: HashMap<uuid::Uuid, JoinHandle<()>>,

    search: SearchPage,

    /// A popupt to show info / error messages
    popup: Option<(PopupMessageWidget, PopupMessageState)>,

    /// The active mode of the application, which could change how user inputs
    /// and commands are interpreted.
    mode: Mode,

    /// The active mode of the application, which could change how user inputs
    /// and commands are interpreted.
    last_mode: Mode,

    /// A list of key events that have been held since the last tick, useful for
    /// interpreting sequences of key presses.
    last_tick_key_events: Vec<KeyEvent>,

    /// frame counter
    frame_count: usize,

    help: Help,

    selected_tab: SelectedTab,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let search = SearchPage::new();
        Self {
            rx,
            tx,
            mode: Mode::default(),
            last_mode: Mode::default(),
            loading_status: Default::default(),
            search,
            crates: Default::default(),
            versions: Default::default(),
            full_crate_info: Default::default(),
            crate_response: Default::default(),
            crate_info: Default::default(),
            summary_data: Default::default(),
            summary: Default::default(),
            last_task_details_handle: Default::default(),
            popup: Default::default(),
            last_tick_key_events: Default::default(),
            frame_count: Default::default(),
            help: Default::default(),
            selected_tab: Default::default(),
        }
    }

    /// Runs the main loop of the application, handling events and actions
    pub async fn run(&mut self, mut tui: Tui, mut events: Events) -> Result<()> {
        // uncomment to test error handling
        // panic!("test panic");
        // Err(color_eyre::eyre::eyre!("Error"))?;
        self.tx.send(Action::Init)?;

        loop {
            if let Some(e) = events.next().await {
                self.handle_event(e)?.map(|action| self.tx.send(action));
            }
            while let Ok(action) = self.rx.try_recv() {
                self.handle_action(action.clone(), &mut tui)?
                    .map(|action| self.tx.send(action));
            }
            if self.should_quit() {
                break;
            }
        }
        Ok(())
    }

    /// Handles an event by producing an optional `Action` that the application
    /// should perform in response.
    ///
    /// This method maps incoming events from the terminal user interface to
    /// specific `Action` that represents tasks or operations the
    /// application needs to carry out.
    fn handle_event(&mut self, e: Event) -> Result<Option<Action>> {
        let maybe_action = match e {
            Event::Quit => Some(Action::Quit),
            Event::Tick => Some(Action::Tick),
            Event::KeyRefresh => Some(Action::KeyRefresh),
            Event::Render => Some(Action::Render),
            Event::Resize(x, y) => Some(Action::Resize(x, y)),
            Event::Key(key) => self.handle_key_event(key)?,
            _ => None,
        };
        Ok(maybe_action)
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        debug!("Received key {:?}", key);
        match self.mode {
            Mode::Search => {
                self.search.handle_key(key);
            }
            Mode::Filter => {
                self.search.handle_key(key);
                self.search.handle_filter_prompt_change();
            }
            _ => (),
        };
        Ok(self.handle_key_events_from_config(key))
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
        config
            .key_bindings
            .event_to_command(self.mode, &self.last_tick_key_events)
            .or_else(|| {
                config
                    .key_bindings
                    .event_to_command(Mode::Common, &self.last_tick_key_events)
            })
            .map(|command| config.key_bindings.command_to_action(command))
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
        if action != Action::Tick && action != Action::Render && action != Action::KeyRefresh {
            info!("{action:?}");
        }
        match action {
            Action::Quit => self.quit(),
            Action::Render => self.draw(tui)?,
            Action::KeyRefresh => self.key_refresh_tick(),
            Action::Init => self.init()?,
            Action::Resize(w, h) => self.resize(tui, (w, h))?,
            Action::Tick => self.tick(),
            Action::StoreTotalNumberOfCrates(n) => self.store_total_number_of_crates(n),
            Action::ScrollUp => self.scroll_up(),
            Action::ScrollDown => self.scroll_down(),

            Action::ScrollTop
            | Action::ScrollBottom
            | Action::ScrollSearchResultsUp
            | Action::ScrollSearchResultsDown => self.search.handle_action(action.clone()),

            Action::ScrollCrateInfoUp => self.crate_info.scroll_previous(),
            Action::ScrollCrateInfoDown => self.crate_info.scroll_next(),
            Action::ReloadData => self.reload_data(),
            Action::IncrementPage => self.increment_page(),
            Action::DecrementPage => self.decrement_page(),
            Action::NextSummaryMode => self.summary.next_mode(),
            Action::PreviousSummaryMode => self.summary.previous_mode(),
            Action::NextTab => self.goto_next_tab(),
            Action::PreviousTab => self.goto_previous_tab(),
            Action::SwitchMode(mode) if mode.is_search() || mode.is_filter() => {
                self.enter_insert_mode(mode)
            }
            Action::SwitchMode(Mode::PickerHideCrateInfo) => self.enter_normal_mode(),
            Action::SwitchMode(Mode::PickerShowCrateInfo) => self.enter_normal_mode(),
            Action::SwitchMode(mode) => self.switch_mode(mode),
            Action::SwitchToLastMode => self.switch_to_last_mode(),
            Action::SubmitSearch => self.submit_search(),
            Action::ToggleShowCrateInfo => self.toggle_show_crate_info(),
            Action::UpdateCurrentSelectionCrateInfo => self.update_current_selection_crate_info(),
            Action::UpdateSearchTableResults => {
                self.search.update_search_table_results(self.crates.clone())
            }
            Action::UpdateSummary => self.update_summary(),
            Action::ShowFullCrateInfo => self.show_full_crate_details(),
            Action::ShowErrorPopup(ref err) => self.show_error_popup(err.clone()),
            Action::ShowInfoPopup(ref info) => self.show_info_popup(info.clone()),
            Action::ClosePopup => self.close_popup(),
            Action::ToggleSortBy { reload, forward } => self.toggle_sort_by(reload, forward)?,
            Action::ClearTaskDetailsHandle(ref id) => {
                self.clear_task_details_handle(uuid::Uuid::parse_str(id)?)?
            }
            Action::OpenDocsUrlInBrowser => self.open_docs_url_in_browser()?,
            Action::OpenCratesIOUrlInBrowser if self.mode.is_summary() => {
                self.open_summary_url_in_browser()?
            }
            Action::OpenCratesIOUrlInBrowser => self.open_crates_io_url_in_browser()?,
            Action::CopyCargoAddCommandToClipboard => self.copy_cargo_add_command_to_clipboard()?,
            _ => {}
        }
        let maybe_action = match action {
            Action::ScrollUp | Action::ScrollDown | Action::ScrollTop | Action::ScrollBottom
                if self.mode.is_summary() || self.mode.is_popup() || self.mode.is_help() =>
            {
                None
            }
            Action::ScrollUp | Action::ScrollDown | Action::ScrollTop | Action::ScrollBottom => {
                Some(Action::UpdateCurrentSelectionCrateInfo)
            }
            Action::SubmitSearch => Some(Action::ReloadData),
            _ => None,
        };
        Ok(maybe_action)
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
}

impl App {
    fn tick(&mut self) {
        self.search.update_search_table_results(self.crates.clone());
    }

    fn init(&mut self) -> Result<()> {
        self.request_summary()?;
        Ok(())
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

    fn scroll_up(&mut self) {
        match self.mode {
            Mode::Popup => {
                if let Some((_, popup_state)) = &mut self.popup {
                    popup_state.scroll_up();
                }
            }
            Mode::Summary => self.summary.scroll_previous(),
            Mode::Help => self.help.scroll_previous(),
            _ => self.search.scroll_up(),
        }
    }

    fn scroll_down(&mut self) {
        match self.mode {
            Mode::Popup => {
                if let Some((_, popup_state)) = &mut self.popup {
                    popup_state.scroll_down();
                }
            }
            Mode::Summary => self.summary.scroll_next(),
            Mode::Help => self.help.scroll_next(),
            _ => self.search.scroll_down(),
        }
    }

    fn increment_page(&mut self) {
        if let Some(n) = self.search.total_num_crates {
            let max_page_size = (n / self.search.page_size) + 1;
            if self.search.page < max_page_size {
                self.search.page = self.search.page.saturating_add(1).min(max_page_size);
                self.reload_data();
            }
        }
    }

    fn decrement_page(&mut self) {
        let min_page_size = 1;
        if self.search.page > min_page_size {
            self.search.page = self.search.page.saturating_sub(1).max(min_page_size);
            self.reload_data();
        }
    }

    fn enter_insert_mode(&mut self, mode: Mode) {
        self.switch_mode(mode);
        self.search.input = self
            .search
            .input
            .clone()
            .with_value(if self.mode.is_search() {
                self.search.search.clone()
            } else if self.mode.is_filter() {
                self.search.filter.clone()
            } else {
                unreachable!("Cannot enter insert mode when mode is {:?}", self.mode)
            });
    }

    fn enter_normal_mode(&mut self) {
        self.switch_mode(Mode::PickerHideCrateInfo);
        if !self.search.search_results.crates.is_empty()
            && self.search.search_results.selected().is_none()
        {
            self.search.search_results.select(Some(0))
        }
    }

    fn switch_mode(&mut self, mode: Mode) {
        self.last_mode = self.mode;
        self.mode = mode;
        match self.mode {
            Mode::Search => {
                self.selected_tab.select(SelectedTab::Search);
                self.search.search_mode = SearchMode::Search;
            }
            Mode::Filter => {
                self.selected_tab.select(SelectedTab::Search);
                self.search.search_mode = SearchMode::Filter;
            }
            Mode::PickerHideCrateInfo => {
                self.selected_tab.select(SelectedTab::Search);
                self.search.search_mode = SearchMode::ResultsHideCrate;
            }
            Mode::PickerShowCrateInfo => {
                self.selected_tab.select(SelectedTab::Search);
                self.search.search_mode = SearchMode::ResultsShowCrate;
            }
            Mode::Summary => self.selected_tab.select(SelectedTab::Summary),
            Mode::Help => {
                self.help.mode = Some(self.last_mode);
                self.selected_tab.select(SelectedTab::None)
            }
            _ => self.selected_tab.select(SelectedTab::None),
        }
    }

    fn switch_to_last_mode(&mut self) {
        self.switch_mode(self.last_mode);
    }

    fn goto_next_tab(&mut self) {
        match self.mode {
            Mode::Summary => self.switch_mode(Mode::Search),
            Mode::Search => self.switch_mode(Mode::Summary),
            _ => self.switch_mode(Mode::Summary),
        }
    }

    fn goto_previous_tab(&mut self) {
        match self.mode {
            Mode::Summary => self.switch_mode(Mode::Search),
            Mode::Search => self.switch_mode(Mode::Summary),
            _ => self.switch_mode(Mode::Summary),
        }
    }

    fn submit_search(&mut self) {
        self.clear_all_previous_task_details_handles();
        self.switch_mode(Mode::PickerHideCrateInfo);
        self.search.filter.clear();
        self.search.search = self.search.input.value().into();
    }

    fn toggle_show_crate_info(&mut self) {
        self.mode.toggle_crate_info();
        if self.mode.should_show_crate_info() {
            self.request_crate_details()
        } else {
            self.clear_all_previous_task_details_handles();
        }
    }

    fn toggle_sort_by_forward(&mut self) {
        use crates_io_api::Sort as S;
        self.search.sort = match self.search.sort {
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
        self.search.sort = match self.search.sort {
            S::Relevance => S::Alphabetical,
            S::Downloads => S::Relevance,
            S::RecentDownloads => S::Downloads,
            S::RecentUpdates => S::RecentDownloads,
            S::NewlyAdded => S::RecentUpdates,
            S::Alphabetical => S::NewlyAdded,
        };
    }

    fn toggle_sort_by(&mut self, reload: bool, forward: bool) -> Result<()> {
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

    fn show_error_popup(&mut self, message: String) {
        error!("Error: {message}");
        self.popup = Some((
            PopupMessageWidget::new("Error".into(), message),
            PopupMessageState::default(),
        ));
        self.last_mode = self.mode;
        self.mode = Mode::Popup;
    }

    fn show_info_popup(&mut self, info: String) {
        info!("Info: {info}");
        self.popup = Some((
            PopupMessageWidget::new("Info".into(), info),
            PopupMessageState::default(),
        ));
        self.last_mode = self.mode;
        self.mode = Mode::Popup;
    }

    fn close_popup(&mut self) {
        self.popup = None;
        if self.last_mode.is_popup() {
            self.switch_mode(Mode::Search);
        } else {
            self.switch_mode(self.last_mode);
        }
    }

    fn update_current_selection_crate_info(&mut self) {
        self.clear_all_previous_task_details_handles();
        self.request_crate_details();
    }

    fn show_full_crate_details(&mut self) {
        self.clear_all_previous_task_details_handles();
        self.request_full_crate_details();
    }

    fn store_total_number_of_crates(&mut self, n: u64) {
        self.search.total_num_crates = Some(n)
    }

    fn open_docs_url_in_browser(&self) -> Result<()> {
        if let Some(crate_response) = self.crate_response.lock().unwrap().clone() {
            let name = crate_response.crate_data.name;
            webbrowser::open(&format!("https://docs.rs/{name}/latest"))?;
        }
        Ok(())
    }

    fn open_summary_url_in_browser(&self) -> Result<()> {
        if let Some(url) = self.summary.url() {
            webbrowser::open(&url)?;
        } else {
            let _ = self.tx.send(Action::ShowErrorPopup(
                "Unable to open URL in browser: No summary data loaded".into(),
            ));
        }
        Ok(())
    }

    fn open_crates_io_url_in_browser(&self) -> Result<()> {
        if let Some(crate_response) = self.crate_response.lock().unwrap().clone() {
            let name = crate_response.crate_data.name;
            webbrowser::open(&format!("https://crates.io/crates/{name}"))?;
        }
        Ok(())
    }

    fn copy_cargo_add_command_to_clipboard(&self) -> Result<()> {
        use copypasta::ClipboardProvider;
        match copypasta::ClipboardContext::new() {
            Ok(mut ctx) => {
                if let Some(crate_response) = self.crate_response.lock().unwrap().clone() {
                    let msg = format!("cargo add {}", crate_response.crate_data.name);
                    let _ = match ctx.set_contents(msg.clone()).ok() {
                        Some(_) => self.tx.send(Action::ShowInfoPopup(format!(
                            "Copied to clipboard: `{msg}`"
                        ))),
                        None => self.tx.send(Action::ShowErrorPopup(format!(
                            "Unable to copied to clipboard: `{msg}`"
                        ))),
                    };
                } else {
                    let _ = self
                        .tx
                        .send(Action::ShowErrorPopup("No selection made to copy".into()));
                }
            }
            Err(err) => {
                let _ = self.tx.send(Action::ShowErrorPopup(format!(
                    "Unable to create ClipboardContext: {}",
                    err
                )));
            }
        }
        Ok(())
    }

    fn clear_task_details_handle(&mut self, id: uuid::Uuid) -> Result<()> {
        if let Some((_, handle)) = self.last_task_details_handle.remove_entry(&id) {
            handle.abort()
        }
        Ok(())
    }

    fn clear_all_previous_task_details_handles(&mut self) {
        *self.full_crate_info.lock().unwrap() = None;
        for (_, v) in self.last_task_details_handle.iter() {
            v.abort()
        }
        self.last_task_details_handle.clear()
    }

    /// Reloads the list of crates based on the current search parameters,
    /// updating the application state accordingly. This involves fetching
    /// data asynchronously from the crates.io API and updating various parts of
    /// the application state, such as the crates listing, current crate
    /// info, and loading status.
    fn reload_data(&mut self) {
        self.prepare_reload();
        let search_params = self.create_search_parameters();
        self.request_search_results(search_params);
    }

    /// Clears current search results and resets the UI to prepare for new data.
    fn prepare_reload(&mut self) {
        self.search.search_results.select(None);
        *self.full_crate_info.lock().unwrap() = None;
        *self.crate_response.lock().unwrap() = None;
    }

    /// Creates the parameters required for the search task.
    fn create_search_parameters(&self) -> crates_io_api_helper::SearchParameters {
        crates_io_api_helper::SearchParameters {
            search: self.search.search.clone(),
            page: self.search.page.clamp(1, u64::MAX),
            page_size: self.search.page_size,
            crates: self.crates.clone(),
            versions: self.versions.clone(),
            loading_status: self.loading_status.clone(),
            sort: self.search.sort.clone(),
            tx: self.tx.clone(),
        }
    }

    /// Spawns an asynchronous task to fetch crate data from crates.io.
    fn request_search_results(&self, params: crates_io_api_helper::SearchParameters) {
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
    fn request_crate_details(&mut self) {
        if self.search.search_results.crates.is_empty() {
            return;
        }
        if let Some(crate_name) = self.search.search_results.selected_crate_name() {
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
    fn request_full_crate_details(&mut self) {
        if self.search.search_results.crates.is_empty() {
            return;
        }
        if let Some(crate_name) = self.search.search_results.selected_crate_name() {
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

    fn request_summary(&self) -> Result<()> {
        let tx = self.tx.clone();
        let loading_status = self.loading_status.clone();
        let summary = self.summary_data.clone();
        tokio::spawn(async move {
            loading_status.store(true, Ordering::SeqCst);
            if let Err(error_message) = crates_io_api_helper::request_summary(summary).await {
                let _ = tx.send(Action::ShowErrorPopup(error_message));
            }
            loading_status.store(false, Ordering::SeqCst);
            let _ = tx.send(Action::UpdateSummary);
            let _ = tx.send(Action::ScrollDown);
        });
        Ok(())
    }

    // Sets the frame count
    fn update_frame_count(&mut self, frame: &mut Frame<'_>) {
        self.frame_count = frame.count();
    }

    // Sets cursor for the prompt
    fn update_cursor(&mut self, frame: &mut Frame<'_>) {
        if let Some(cursor_position) = self.search.cursor_position() {
            frame.set_cursor(cursor_position.x, cursor_position.y)
        }
    }

    fn update_summary(&mut self) {
        if let Some(summary) = self.summary_data.lock().unwrap().clone() {
            self.summary.summary_data = Some(summary);
        } else {
            self.summary.summary_data = None;
        }
    }

    fn render_crate_info(&mut self, area: Rect, buf: &mut Buffer) {
        if let Some(ci) = self.crate_response.lock().unwrap().clone() {
            Clear.render(area, buf);
            CrateInfoTableWidget::new(ci).render(area, buf, &mut self.crate_info);
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
                .title_position(ratatui::widgets::block::Position::Top)
                .title_alignment(ratatui::layout::Alignment::Right),
        )
    }

    fn selected_with_page_context(&self) -> u64 {
        self.search.search_results.selected().map_or(0, |n| {
            (self.search.page.saturating_sub(1) * self.search.page_size) + n as u64 + 1
        })
    }

    fn loading(&self) -> bool {
        self.loading_status.load(Ordering::SeqCst)
    }

    fn page_number_status(&self) -> String {
        let max_page_size =
            (self.search.total_num_crates.unwrap_or_default() / self.search.page_size) + 1;
        format!("Page: {}/{}", self.search.page, max_page_size)
    }

    fn search_results_status(&self) -> String {
        let selected = self.selected_with_page_context();
        let ncrates = self.search.total_num_crates.unwrap_or_default();
        format!("{}/{} Results", selected, ncrates)
    }
}

impl StatefulWidget for AppWidget {
    type State = App;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Background color
        Block::default()
            .bg(config::get().color.base00)
            .render(area, buf);

        use Constraint::*;
        let [header, main] = Layout::vertical([Length(1), Fill(1)]).areas(area);
        let [tabs, events] = Layout::horizontal([Min(15), Fill(1)]).areas(header);

        state.render_tabs(tabs, buf);
        state.events_widget().render(events, buf);

        let mode = if matches!(state.mode, Mode::Popup | Mode::Quit) {
            state.last_mode
        } else {
            state.mode
        };
        match mode {
            Mode::Summary => state.render_summary(main, buf),
            Mode::Help => state.render_help(main, buf),

            Mode::Search => state.render_search(main, buf),
            Mode::Filter => state.render_search(main, buf),
            Mode::PickerShowCrateInfo => state.render_search_with_crate(main, buf),
            Mode::PickerHideCrateInfo => state.render_search(main, buf),

            Mode::Common => {}
            Mode::Popup => {}
            Mode::Quit => {}
        };

        if state.loading() {
            Line::from(state.spinner())
                .right_aligned()
                .render(main, buf);
        }

        if let Some((popup, popup_state)) = &mut state.popup {
            popup.render(area, buf, popup_state);
        }
    }
}

impl App {
    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        use strum::IntoEnumIterator;
        let titles = SelectedTab::iter().map(|tab| tab.title());
        let highlight_style = SelectedTab::highlight_style();

        let selected_tab_index = self.selected_tab as usize;
        Tabs::new(titles)
            .highlight_style(highlight_style)
            .select(selected_tab_index)
            .padding("", "")
            .divider(" ")
            .render(area, buf);
    }

    fn render_search_with_crate(&mut self, area: Rect, buf: &mut Buffer) {
        let [area, info] = Layout::vertical([Constraint::Min(0), Constraint::Max(15)]).areas(area);
        self.render_search(area, buf);
        self.render_crate_info(info, buf);
    }

    fn render_summary(&mut self, area: Rect, buf: &mut Buffer) {
        let [main, prompt] =
            Layout::vertical([Constraint::Fill(0), Constraint::Length(1)]).areas(area);
        SummaryWidget.render(main, buf, &mut self.summary);
        self.render_prompt(prompt, buf);
    }

    fn render_help(&mut self, area: Rect, buf: &mut Buffer) {
        let [main, prompt] =
            Layout::vertical([Constraint::Fill(0), Constraint::Length(1)]).areas(area);
        HelpWidget.render(main, buf, &mut self.help);
        self.render_prompt(prompt, buf);
    }

    fn render_search(&mut self, area: Rect, buf: &mut Buffer) {
        let prompt_height = if self.mode.is_picker() { 1 } else { 5 };
        let [main, prompt] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(prompt_height)]).areas(area);

        SearchResultsTableWidget::new(self.mode.is_picker()).render(
            main,
            buf,
            &mut self.search.search_results,
        );

        Line::from(self.page_number_status()).left_aligned().render(
            main.inner(&Margin {
                horizontal: 1,
                vertical: 2,
            }),
            buf,
        );

        Line::from(self.search_results_status())
            .right_aligned()
            .render(
                main.inner(&Margin {
                    horizontal: 1,
                    vertical: 2,
                }),
                buf,
            );

        self.render_prompt(prompt, buf);
    }

    fn render_prompt(&mut self, area: Rect, buf: &mut Buffer) {
        let p =
            SearchFilterPromptWidget::new(self.mode, self.search.sort.clone(), &self.search.input);
        p.render(area, buf, &mut self.search.prompt);
    }

    fn spinner(&self) -> String {
        let spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let index = self.frame_count % spinner.len();
        let symbol = spinner[index];
        symbol.into()
    }
}
