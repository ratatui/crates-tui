use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use strum::Display;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, info};
use tui_input::backend::crossterm::EventHandler;

use crate::{
    action::Action,
    config,
    serde_helper::keybindings::key_event_to_string,
    tui::{self, Tui},
    widgets::{
        crate_info::CrateInfo,
        crates_table::{CrateTableState, CratesTable},
        popup::Popup,
        prompt::{Prompt, PromptState},
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
}

// FIXME comments on the fields
#[derive(Debug)]
pub struct App {
    tx: UnboundedSender<Action>,
    page: u64,
    page_size: u64,
    mode: Mode,
    last_key_events: Vec<KeyEvent>,
    loading_status: Arc<AtomicBool>,
    search: String,
    filter: String,
    filtered_crates: Vec<crates_io_api::Crate>,
    crates: Arc<Mutex<Vec<crates_io_api::Crate>>>,
    crate_info: Arc<Mutex<Option<crates_io_api::Crate>>>,
    total_num_crates: Option<u64>,
    input: tui_input::Input,
    crate_table_state: CrateTableState,
    show_crate_info: bool,
    error: Option<String>,
    info: Option<String>,
    popup_scroll: usize,
    last_tick_key_events: Vec<KeyEvent>,
    prompt_state: PromptState,
}

impl App {
    pub fn new(tx: UnboundedSender<Action>) -> Self {
        Self {
            tx,
            page: 1,
            page_size: 25,
            mode: Mode::default(),
            last_key_events: Default::default(),
            loading_status: Default::default(),
            search: Default::default(),
            filter: Default::default(),
            filtered_crates: Default::default(),
            crates: Default::default(),
            crate_info: Default::default(),
            total_num_crates: Default::default(),
            input: Default::default(),
            crate_table_state: Default::default(),
            show_crate_info: Default::default(),
            error: Default::default(),
            info: Default::default(),
            popup_scroll: Default::default(),
            last_tick_key_events: Default::default(),
            prompt_state: Default::default(),
        }
    }

    // FIXME: can we make this infinitely scrollable instead of manually handling the page size?
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
        self.popup_scroll = self.popup_scroll.saturating_sub(1)
    }

    fn popup_scroll_next(&mut self) {
        self.popup_scroll = self.popup_scroll.saturating_add(1)
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
        if !self.filtered_crates.is_empty() && self.crate_table_state.selected().is_none() {
            self.crate_table_state.select(Some(0))
        }
    }

    fn submit_search(&mut self) {
        self.mode = Mode::Picker;
        self.filter.clear();
        self.search = self.input.value().into();
    }

    fn toggle_show_crate_info(&mut self) {
        self.show_crate_info = !self.show_crate_info;
        if self.show_crate_info {
            self.fetch_crate_details()
        } else {
            *self.crate_info.lock().unwrap() = None;
        }
    }

    fn set_error_flag(&mut self, err: String) {
        error!("Error: {err}");
        self.error = Some(err);
        self.mode = Mode::Popup;
    }

    fn set_info_flag(&mut self, info: String) {
        info!("Info: {info}");
        self.info = Some(info);
        self.mode = Mode::Popup;
    }

    fn clear_error_and_info_flags(&mut self) {
        self.error = None;
        self.info = None;
        self.mode = Mode::Search;
    }

    fn update_current_selection_crate_info(&mut self) {
        if self.show_crate_info {
            self.fetch_crate_details();
        } else {
            *self.crate_info.lock().unwrap() = None;
        }
    }

    fn store_total_number_of_crates(&mut self, n: u64) {
        self.total_num_crates = Some(n)
    }

    // FIXME overly long and complex method
    fn reload_data(&mut self) {
        self.crate_table_state.select(None);
        *self.crate_info.lock().unwrap() = None;
        let crates = self.crates.clone();
        let search = self.search.clone();
        let loading_status = self.loading_status.clone();
        let tx = self.tx.clone();
        let page = self.page.clamp(1, u64::MAX);
        let page_size = self.page_size;
        tokio::spawn(async move {
            loading_status.store(true, Ordering::SeqCst);
            match crates_io_api::AsyncClient::new(
                "crates-tui (crates-tui@kdheepak.com)",
                std::time::Duration::from_millis(1000),
            ) {
                Ok(client) => {
                    let query = crates_io_api::CratesQueryBuilder::default()
                        .search(&search)
                        .page(page)
                        .page_size(page_size)
                        .sort(crates_io_api::Sort::Relevance)
                        .build();
                    match client.crates(query).await {
                        Ok(page) => {
                            let mut all_crates = vec![];
                            for crate_ in page.crates.iter() {
                                all_crates.push(crate_.clone())
                            }
                            all_crates.sort_by(|a, b| b.downloads.cmp(&a.downloads));
                            crates.lock().unwrap().drain(0..);
                            *crates.lock().unwrap() = all_crates;
                            if crates.lock().unwrap().len() > 0 {
                                tx.send(Action::StoreTotalNumberOfCrates(page.meta.total))
                                    .unwrap_or_default();
                                tx.send(Action::Tick).unwrap_or_default();
                                tx.send(Action::ScrollDown).unwrap_or_default();
                            } else {
                                tx.send(Action::Error(format!(
                                    "Could not find any crates with query `{search}`.",
                                )))
                                .unwrap_or_default();
                            }
                            loading_status.store(false, Ordering::SeqCst);
                        }
                        Err(err) => {
                            tx.send(Action::Error(format!("API Client Error: {err:#?}")))
                                .unwrap_or_default();
                            loading_status.store(false, Ordering::SeqCst);
                        }
                    }
                }
                Err(err) => tx
                    .send(Action::Error(format!("Error creating client: {err:#?}")))
                    .unwrap_or_default(),
            }
        });
    }

    // Extracts the selected crate name, if possible.
    fn selected_crate_name(&self) -> Option<String> {
        self.crate_table_state
            .selected()
            .and_then(|index| self.filtered_crates.get(index))
            .filter(|crate_| !crate_.name.is_empty())
            .map(|crate_| crate_.name.clone())
    }

    fn fetch_crate_details(&mut self) {
        if self.filtered_crates.is_empty() {
            return;
        }
        if let Some(crate_name) = self.selected_crate_name() {
            let tx = self.tx.clone();
            let crate_info = self.crate_info.clone();
            let loading_status = self.loading_status.clone();

            // Spawn the async work to fetch crate details.
            tokio::spawn(async move {
                loading_status.store(true, Ordering::SeqCst);
                App::async_fetch_crate_details(crate_name, tx, crate_info).await;
                loading_status.store(false, Ordering::SeqCst);
            });
        }
    }

    // Performs the async fetch of crate details.
    async fn async_fetch_crate_details(
        crate_name: String,
        tx: UnboundedSender<Action>,
        crate_info: Arc<Mutex<Option<crates_io_api::Crate>>>,
    ) {
        let client = match crates_io_api::AsyncClient::new(
            "crates-tui (crates-tui@kdheepak.com)",
            std::time::Duration::from_millis(1000),
        ) {
            Ok(client) => client,
            Err(error_message) => {
                return tx
                    .send(Action::Error(format!("{}", error_message)))
                    .unwrap_or_default();
            }
        };

        let result = client.get_crate(&crate_name).await;

        match result {
            Ok(crate_data) => *crate_info.lock().unwrap() = Some(crate_data.crate_data),
            Err(err) => {
                let error_message = format!("Error fetching crate details: {err}");
                tx.send(Action::Error(error_message)).unwrap_or_default();
            }
        }
    }

    fn tick(&mut self) {
        // FIXME: this is not obvious what it does what are the last_events? Why are you removing them?
        self.last_key_events.drain(..);
        self.update_filtered_crates();
        self.update_crate_table_state();
    }

    fn update_crate_table_state(&mut self) {
        self.crate_table_state
            .content_length(self.filtered_crates.len());
    }

    fn update_filtered_crates(&mut self) {
        let filter = self.filter.clone();
        let filter_words = filter.split_whitespace().collect::<Vec<_>>();
        self.filtered_crates = self
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
            .collect();
    }
}

impl App {
    async fn handle_tui_event(
        &mut self,
        e: tui::Event,
        tx: &UnboundedSender<Action>,
    ) -> Result<()> {
        match e {
            tui::Event::Quit => tx.send(Action::Quit)?,
            tui::Event::Tick => tx.send(Action::Tick)?,
            tui::Event::KeyRefresh => tx.send(Action::KeyRefresh)?,
            tui::Event::Render => tx.send(Action::Render)?,
            tui::Event::Resize(x, y) => tx.send(Action::Resize(x, y))?,
            tui::Event::Key(key) => {
                debug!("Received key {:?}", key);
                if let Some(action) = self.handle_key_events(key)? {
                    tx.send(action)?;
                }
                if let Some(action) = self.handle_key_events_from_config(key) {
                    tx.send(action)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_action(
        &mut self,
        action: Action,
        tui: &mut Tui,
        tx: &UnboundedSender<Action>,
    ) -> Result<()> {
        if action != Action::Tick && action != Action::Render {
            info!("{action:?}");
        }
        match action {
            Action::KeyRefresh => {
                self.last_tick_key_events.drain(..);
            }
            Action::Resize(w, h) => {
                tui.resize(Rect::new(0, 0, w, h))?;
                tx.send(Action::Render)?;
            }
            Action::Render => {
                tui.draw(|f| self.draw(f, f.size()))?;
            }
            _ => {}
        }
        Ok(())
    }

    // The main 'run' function now delegates to the two functions above,
    // to handle TUI events and App actions respectively.
    pub async fn run(&mut self, tui: &mut Tui, mut rx: UnboundedReceiver<Action>) -> Result<()> {
        let mut should_quit = false;
        let tx = self.tx.clone();

        tui.enter()?;

        loop {
            if let Some(e) = tui.next().await {
                self.handle_tui_event(e, &tx).await?;
            }
            while let Ok(action) = rx.try_recv() {
                if let Some(inner_action) = self.update(action.clone())? {
                    tx.send(inner_action)?
                };
                self.handle_action(action.clone(), tui, &tx).await?;
                if action == Action::Quit {
                    should_quit = true;
                    break;
                }
            }
            if should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    pub fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => self.tick(),
            Action::StoreTotalNumberOfCrates(n) => self.store_total_number_of_crates(n),
            Action::ScrollUp if self.mode == Mode::Popup => self.popup_scroll_previous(),
            Action::ScrollDown if self.mode == Mode::Popup => self.popup_scroll_next(),
            Action::ScrollUp => self.crate_table_state.previous_crate(&self.filtered_crates),
            Action::ScrollDown => self.crate_table_state.next_crate(&self.filtered_crates),
            Action::ScrollTop => self.crate_table_state.top(&self.filtered_crates),
            Action::ScrollBottom => self.crate_table_state.bottom(&self.filtered_crates),
            Action::ReloadData => self.reload_data(),
            Action::IncrementPage => self.increment_page(),
            Action::DecrementPage => self.decrement_page(),
            Action::EnterSearchInsertMode => self.enter_search_insert_mode(),
            Action::EnterFilterInsertMode => self.enter_filter_insert_mode(),
            Action::EnterNormal => self.enter_normal_mode(),
            Action::SubmitSearch => self.submit_search(),
            Action::ToggleShowCrateInfo => self.toggle_show_crate_info(),
            Action::UpdateCurrentSelectionCrateInfo => self.update_current_selection_crate_info(),
            Action::Error(ref err) => self.set_error_flag(err.clone()),
            Action::Info(ref info) => self.set_info_flag(info.clone()),
            Action::ClosePopup => self.clear_error_and_info_flags(),
            _ => {}
        };

        match action {
            Action::ScrollUp | Action::ScrollDown | Action::ScrollTop | Action::ScrollBottom => {
                Ok(Some(Action::UpdateCurrentSelectionCrateInfo))
            }
            Action::SubmitSearch => Ok(Some(Action::ReloadData)),
            _ => Ok(None),
        }
    }

    pub fn handle_key_events_from_config(&mut self, key: KeyEvent) -> Option<Action> {
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

    pub fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match self.mode {
            Mode::Search => match key.code {
                _ => {
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    return Ok(None);
                }
            },
            Mode::Filter => match key.code {
                _ => {
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    self.filter = self.input.value().into();
                    self.crate_table_state.select(None);
                    return Ok(None);
                }
            },
            _ => return Ok(None),
        };
    }

    // FIXME - render a single top level widget and then inside that widget render the other
    // widgets
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        frame.render_widget(
            Block::default().bg(config::get().style.background_color),
            area,
        );

        let [table, prompt] = Layout::vertical([
            Constraint::Fill(0),
            Constraint::Length(3 + config::get().prompt_padding * 2),
        ])
        .areas(area);

        // FIXME every part of this method has complex logic that calls or creats other methods
        // That makes it hard to understand the whole method. Split it into smaller methods
        let table = match self.crate_info.lock().unwrap().clone() {
            Some(ci) if self.show_crate_info => {
                let [table, info] =
                    Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(table);
                frame.render_widget(CrateInfo::new(ci), info);
                table
            }
            _ => table,
        };

        frame.render_stateful_widget(
            CratesTable::new(&self.filtered_crates, self.mode == Mode::Picker),
            table,
            &mut self.crate_table_state,
        );

        let loading_status = self.loading_status.load(Ordering::SeqCst);
        let selected = self.crate_table_state.selected().map_or(0, |n| {
            (self.page.saturating_sub(1) * self.page_size) + n as u64 + 1
        });
        let total_num_crates = self.total_num_crates.unwrap_or_default();

        let p = Prompt::new(
            total_num_crates,
            selected,
            loading_status,
            self.mode,
            &self.input,
        );
        frame.render_stateful_widget(&p, prompt, &mut self.prompt_state);

        self.prompt_state.frame_count(frame.count());
        if let Some(cursor_position) = self.prompt_state.cursor_position() {
            frame.set_cursor(cursor_position.x, cursor_position.y)
        }

        if let Some(err) = &self.error {
            frame.render_widget(Popup::new("Error", err, self.popup_scroll), area);
        }
        if let Some(info) = &self.info {
            frame.render_widget(Popup::new("Info", info, self.popup_scroll), area);
        }

        frame.render_widget(
            Block::default()
                .title(format!(
                    "{:?}",
                    self.last_tick_key_events
                        .iter()
                        .map(key_event_to_string)
                        .collect::<Vec<_>>()
                ))
                .title_position(ratatui::widgets::block::Position::Bottom)
                .title_alignment(ratatui::layout::Alignment::Right),
            frame.size(),
        );
    }
}
