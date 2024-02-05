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
use tui_input::backend::crossterm::EventHandler;

use crate::{
    action::Action,
    config,
    serde_helper::keybindings::key_event_to_string,
    tui::{self, Tui},
    widgets::{crate_info::CrateInfo, crates_table::CratesTable, popup::Popup, prompt::Prompt},
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

#[derive(Debug)]
pub struct App {
    tx: UnboundedSender<Action>,
    page: u64,
    page_size: u64,
    mode: Mode,
    last_events: Vec<KeyEvent>,
    loading_status: Arc<AtomicBool>,
    search: String,
    filter: String,
    filtered_crates: Vec<crates_io_api::Crate>,
    crates: Arc<Mutex<Vec<crates_io_api::Crate>>>,
    crate_info: Arc<Mutex<Option<crates_io_api::Crate>>>,
    total_num_crates: Option<u64>,
    input: tui_input::Input,
    show_crate_info: bool,
    error: Option<String>,
    info: Option<String>,
    table_state: TableState,
    scrollbar_state: ScrollbarState,
    popup_scroll: usize,
    last_tick_key_events: Vec<KeyEvent>,
}

impl App {
    pub fn new(tx: UnboundedSender<Action>) -> Self {
        Self {
            tx,
            page: 1,
            page_size: 25,
            mode: Mode::default(),
            last_events: Default::default(),
            loading_status: Default::default(),
            search: Default::default(),
            filter: Default::default(),
            filtered_crates: Default::default(),
            crates: Default::default(),
            crate_info: Default::default(),
            total_num_crates: Default::default(),
            table_state: Default::default(),
            scrollbar_state: Default::default(),
            input: Default::default(),
            show_crate_info: Default::default(),
            error: Default::default(),
            info: Default::default(),
            popup_scroll: Default::default(),
            last_tick_key_events: Default::default(),
        }
    }

    pub fn next(&mut self) {
        if self.filtered_crates.is_empty() {
            self.table_state.select(None)
        } else {
            // wrapping behavior
            let i = match self.table_state.selected() {
                Some(i) => {
                    if i >= self.filtered_crates.len().saturating_sub(1) {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.table_state.select(Some(i));
            self.scrollbar_state = self.scrollbar_state.position(i);
        }
    }

    pub fn previous(&mut self) {
        if self.filtered_crates.is_empty() {
            self.table_state.select(None)
        } else {
            // wrapping behavior
            let i = match self.table_state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.filtered_crates.len().saturating_sub(1)
                    } else {
                        i.saturating_sub(1)
                    }
                }
                None => 0,
            };
            self.table_state.select(Some(i));
            self.scrollbar_state = self.scrollbar_state.position(i);
        }
    }

    pub fn top(&mut self) {
        if self.filtered_crates.is_empty() {
            self.table_state.select(None)
        } else {
            self.table_state.select(Some(0));
            self.scrollbar_state = self.scrollbar_state.position(0);
        }
    }

    pub fn bottom(&mut self) {
        if self.filtered_crates.is_empty() {
            self.table_state.select(None)
        } else {
            self.table_state
                .select(Some(self.filtered_crates.len() - 1));
            self.scrollbar_state = self
                .scrollbar_state
                .position(self.filtered_crates.len() - 1);
        }
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

    fn reload_data(&mut self) {
        self.table_state.select(None);
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
                            for _crate in page.crates.iter() {
                                all_crates.push(_crate.clone())
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

    fn get_info(&mut self) {
        if self.filtered_crates.is_empty() {
            return;
        }

        let tx = self.tx.clone();

        let index = self.table_state.selected().unwrap_or_default();
        let name = self.filtered_crates[index].name.clone();

        if !name.is_empty() {
            let crate_info = self.crate_info.clone();
            let loading_status = self.loading_status.clone();
            tokio::spawn(async move {
                loading_status.store(true, Ordering::SeqCst);
                match crates_io_api::AsyncClient::new(
                    "crates-tui (crates-tui@kdheepak.com)",
                    std::time::Duration::from_millis(1000),
                ) {
                    Ok(client) => match client.get_crate(&name).await {
                        Ok(_crate_info) => {
                            *crate_info.lock().unwrap() = Some(_crate_info.crate_data)
                        }
                        Err(err) => tx
                            .send(Action::Error(format!(
                                "Unable to get crate information: {err}"
                            )))
                            .unwrap_or_default(),
                    },
                    Err(err) => tx
                        .send(Action::Error(format!("Error creating client: {err:?}")))
                        .unwrap_or_default(),
                }
                loading_status.store(false, Ordering::SeqCst);
            });
        }
    }

    fn tick(&mut self) {
        self.last_events.drain(..);
        self.update_filtered_crates();
        self.update_scrollbar_state();
    }

    fn update_scrollbar_state(&mut self) {
        self.scrollbar_state = self
            .scrollbar_state
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

    fn cargo_add(&mut self) {
        let crate_info = self.crate_info.lock().unwrap().clone();
        let tx = self.tx.clone();
        if let Some(ci) = crate_info {
            tokio::spawn(async move {
                let output = tokio::process::Command::new("cargo")
                    .arg("add")
                    .arg(ci.name)
                    .output()
                    .await;
                match output {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        if !stdout.is_empty() {
                            tx.send(Action::Info(stdout)).unwrap_or_default();
                        }
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        if !stderr.is_empty() {
                            tx.send(Action::Error(stderr)).unwrap_or_default();
                        }
                    }
                    Err(err) => {
                        let data = format!("ERROR: {:?}", err);
                        tx.send(Action::Error(data)).unwrap_or_default();
                    }
                }
            });
        }
    }
}

impl App {
    pub async fn run(&mut self, tui: &mut Tui, mut rx: UnboundedReceiver<Action>) -> Result<()> {
        let mut should_quit = false;
        let tx = self.tx.clone();

        tui.enter()?;

        loop {
            if let Some(e) = tui.next().await {
                match e {
                    tui::Event::Quit => tx.send(Action::Quit)?,
                    tui::Event::Tick => tx.send(Action::Tick)?,
                    tui::Event::KeyRefresh => tx.send(Action::KeyRefresh)?,
                    tui::Event::Render => tx.send(Action::Render)?,
                    tui::Event::Resize(x, y) => tx.send(Action::Resize(x, y))?,
                    tui::Event::Key(key) => {
                        log::debug!("Received key {:?}", key);
                        if let Some(action) = self.handle_key_events(key)? {
                            tx.send(action)?;
                        }
                        if let Some(action) = self.handle_key_events_from_config(key) {
                            tx.send(action)?;
                        }
                    }
                    _ => {}
                }
            }

            while let Ok(action) = rx.try_recv() {
                if action != Action::Tick && action != Action::Render {
                    log::info!("{action:?}");
                }
                if let Some(action) = self.update(action.clone())? {
                    tx.send(action)?
                };
                match action {
                    Action::KeyRefresh => {
                        self.last_tick_key_events.drain(..);
                    }
                    Action::Quit => should_quit = true,
                    Action::Resize(w, h) => {
                        tui.resize(Rect::new(0, 0, w, h))?;
                        tx.send(Action::Render)?;
                    }
                    Action::Render => {
                        tui.draw(|f| {
                            self.draw(f, f.size());
                        })?;
                    }
                    _ => {}
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
            Action::StoreTotalNumberOfCrates(n) => self.total_num_crates = Some(n),
            Action::ScrollUp if self.mode == Mode::Popup => {
                self.popup_scroll = self.popup_scroll.saturating_sub(1)
            }
            Action::ScrollDown if self.mode == Mode::Popup => {
                self.popup_scroll = self.popup_scroll.saturating_add(1)
            }
            Action::ReloadData => self.reload_data(),
            Action::IncrementPage => self.increment_page(),
            Action::DecrementPage => self.decrement_page(),
            Action::CargoAddCrate => self.cargo_add(),
            Action::ScrollUp => {
                self.previous();
                return Ok(Some(Action::GetInfo));
            }
            Action::ScrollDown => {
                self.next();
                return Ok(Some(Action::GetInfo));
            }
            Action::ScrollTop => {
                self.top();
                return Ok(Some(Action::GetInfo));
            }
            Action::ScrollBottom => {
                self.bottom();
                return Ok(Some(Action::GetInfo));
            }
            Action::EnterSearchInsertMode => {
                self.mode = Mode::Search;
                self.input = self.input.clone().with_value(self.search.clone());
            }
            Action::EnterFilterInsertMode => {
                self.show_crate_info = false;
                self.mode = Mode::Filter;
                self.input = self.input.clone().with_value(self.filter.clone());
            }
            Action::EnterNormal => {
                self.mode = Mode::Picker;
                if !self.filtered_crates.is_empty() && self.table_state.selected().is_none() {
                    self.table_state.select(Some(0))
                }
            }
            Action::SubmitSearchWithQuery(search) => {
                self.mode = Mode::Picker;
                self.filter.clear();
                self.search = search;
                return Ok(Some(Action::ReloadData));
            }
            Action::SubmitSearch => {
                self.mode = Mode::Picker;
                self.filter.clear();
                self.search = self.input.value().into();
                return Ok(Some(Action::ReloadData));
            }
            Action::ToggleShowCrateInfo => {
                self.show_crate_info = !self.show_crate_info;
                if self.show_crate_info {
                    self.get_info()
                } else {
                    *self.crate_info.lock().unwrap() = None;
                }
            }
            Action::GetInfo => {
                if self.show_crate_info {
                    self.get_info();
                } else {
                    *self.crate_info.lock().unwrap() = None;
                }
            }
            Action::Error(err) => {
                log::error!("Error: {err}");
                self.error = Some(err);
                self.mode = Mode::Popup;
            }
            Action::Info(info) => {
                log::info!("Info: {info}");
                self.info = Some(info);
                self.mode = Mode::Popup;
            }
            Action::ClosePopup => {
                self.error = None;
                self.info = None;
                self.mode = Mode::Search;
            }
            _ => {}
        }
        Ok(None)
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
                    self.table_state.select(None);
                    return Ok(None);
                }
            },
            _ => return Ok(None),
        };
    }

    pub fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        f.render_widget(
            Block::default().bg(config::get().style.background_color),
            area,
        );

        let [table, prompt] = Layout::vertical([
            Constraint::Fill(0),
            Constraint::Length(3 + config::get().prompt_padding * 2),
        ])
        .areas(area);

        let table = match self.crate_info.lock().unwrap().clone() {
            Some(ci) if self.show_crate_info => {
                let [table, info] =
                    Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(table);
                f.render_widget(CrateInfo::new(ci), info);
                table
            }
            _ => table,
        };

        f.render_stateful_widget(
            CratesTable::new(&self.filtered_crates, self.mode == Mode::Picker),
            table,
            &mut (&mut self.table_state, &mut self.scrollbar_state),
        );

        let loading_status = self.loading_status.load(Ordering::SeqCst);
        let selected = self.table_state.selected().map_or(0, |n| {
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
        f.render_widget(&p, prompt);
        p.render_cursor(f, prompt);
        if loading_status {
            p.render_spinner(f, prompt);
        }

        if let Some(err) = &self.error {
            f.render_widget(Popup::new("Error", err, self.popup_scroll), area);
        }
        if let Some(info) = &self.info {
            f.render_widget(Popup::new("Info", info, self.popup_scroll), area);
        }

        f.render_widget(
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
            f.size(),
        );
    }
}
