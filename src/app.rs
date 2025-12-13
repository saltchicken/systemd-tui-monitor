use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{Terminal, backend::Backend, widgets::ListState};
use std::time::{Duration, Instant};

pub mod model;
pub mod systemd;
pub mod ui;

use model::Service;

pub struct App {
    services: Vec<Service>,
    list_state: ListState,
    should_quit: bool,
    show_only_user_config: bool,

    showing_logs: bool,
    logs: Vec<String>,
    log_scroll: u16,
    stick_to_bottom: bool,

    last_data_tick: Instant,
    data_tick_rate: Duration,
}

impl App {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            services: Vec::new(),
            list_state,
            should_quit: false,
            show_only_user_config: true,

            showing_logs: false,
            logs: Vec::new(),
            log_scroll: 0,
            stick_to_bottom: true,

            last_data_tick: Instant::now(),
            data_tick_rate: Duration::from_secs(2),
        }
    }

    fn get_current_view_services(&self) -> Vec<Service> {
        if self.show_only_user_config {
            self.services
                .iter()
                .filter(|s| s.is_user_config)
                .cloned()
                .collect()
        } else {
            self.services.clone()
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        self.refresh_services()?;

        let mut last_tick = Instant::now();

        let tick_rate = Duration::from_millis(100);

        loop {
            let current_view_services = self.get_current_view_services();
            let terminal_size = terminal.size()?;

            // We only fetch if NOT showing logs (to prevent UI jumps/lag while reading)
            if !self.showing_logs && self.last_data_tick.elapsed() >= self.data_tick_rate {
                self.refresh_services()?;
                self.last_data_tick = Instant::now();
            }

            if self.showing_logs {
                if let Some(index) = self.list_state.selected() {
                    if let Some(service) = current_view_services.get(index) {
                        // Ideally this should also be throttled, but for now we keep it
                        // to ensure "live" logs feel live.
                        if let Ok(new_logs) = systemd::get_service_logs(&service.name) {
                            self.logs = new_logs;

                            if self.stick_to_bottom {
                                let popup_height =
                                    (terminal_size.height * 80 / 100).saturating_sub(2);
                                self.log_scroll =
                                    (self.logs.len() as u16).saturating_sub(popup_height);
                            }
                        }
                    }
                }
            }

            terminal.draw(|f| {
                ui::render(
                    f,
                    &current_view_services,
                    &mut self.list_state,
                    self.show_only_user_config,
                    self.showing_logs,
                    &self.logs,
                    self.log_scroll,
                    self.stick_to_bottom,
                )
            })?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if self.showing_logs {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('l') => {
                                self.showing_logs = false;
                                self.logs.clear();
                                self.log_scroll = 0;
                                self.stick_to_bottom = true;

                                self.force_next_refresh();
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                self.stick_to_bottom = false;
                                if self.log_scroll < (self.logs.len() as u16).saturating_sub(1) {
                                    self.log_scroll += 1;
                                }
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                self.stick_to_bottom = false;
                                if self.log_scroll > 0 {
                                    self.log_scroll -= 1;
                                }
                            }

                            KeyCode::Char('G') | KeyCode::End => {
                                self.stick_to_bottom = true;
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => self.should_quit = true,

                            KeyCode::Char('j') => self.next(&current_view_services),
                            KeyCode::Char('k') => self.previous(&current_view_services),

                            KeyCode::Tab => {
                                self.show_only_user_config = !self.show_only_user_config;
                                self.list_state.select(Some(0));
                            }

                            KeyCode::Char('l') => {
                                if let Some(index) = self.list_state.selected() {
                                    if let Some(service) = current_view_services.get(index) {
                                        match systemd::get_service_logs(&service.name) {
                                            Ok(logs) => {
                                                self.logs = logs;
                                                self.showing_logs = true;
                                                self.log_scroll = 0;
                                                self.stick_to_bottom = true;
                                            }
                                            Err(_) => {
                                                // Handle error
                                            }
                                        }
                                    }
                                }
                            }

                            KeyCode::Char('s') => self.perform_action(
                                systemd::ServiceAction::Start,
                                &current_view_services,
                            )?,
                            KeyCode::Char('x') => self.perform_action(
                                systemd::ServiceAction::Stop,
                                &current_view_services,
                            )?,
                            KeyCode::Char('r') => self.perform_action(
                                systemd::ServiceAction::Restart,
                                &current_view_services,
                            )?,
                            _ => {}
                        }
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }

            if self.should_quit {
                return Ok(());
            }
        }
    }

    fn force_next_refresh(&mut self) {
        // We set the last_tick to the past, ensuring elapsed() > 2s
        self.last_data_tick = Instant::now()
            .checked_sub(self.data_tick_rate * 2)
            .unwrap_or(Instant::now());
    }

    fn refresh_services(&mut self) -> Result<()> {
        let new_services = systemd::get_user_services()?;

        self.services = new_services;

        // Logic to correct cursor if list shrunk
        if let Some(selected) = self.list_state.selected() {
            let current_len = self.get_current_view_services().len();
            if current_len == 0 {
                self.list_state.select(None);
            } else if selected >= current_len {
                self.list_state.select(Some(current_len.saturating_sub(1)));
            }
        }
        Ok(())
    }

    fn next(&mut self, services: &[Service]) {
        if services.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= services.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self, services: &[Service]) {
        if services.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    services.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn perform_action(
        &mut self,
        action: systemd::ServiceAction,
        services: &[Service],
    ) -> Result<()> {
        if let Some(index) = self.list_state.selected() {
            if let Some(service) = services.get(index) {
                // In a production app, we would spawn a thread here.
                let _ = systemd::control_service(&service.name, action);

                // we force the next loop iteration to refresh data.
                self.force_next_refresh();
            }
        }
        Ok(())
    }
}
