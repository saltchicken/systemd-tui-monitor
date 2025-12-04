// The central application controller and event loop.

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
        let tick_rate = Duration::from_secs(2);

        loop {
            let current_view_services = self.get_current_view_services();

            terminal.draw(|f| {
                ui::render(
                    f,
                    &current_view_services,
                    &mut self.list_state,
                    self.show_only_user_config,
                )
            })?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => self.should_quit = true,

                        KeyCode::Char('j') => self.next(&current_view_services),
                        KeyCode::Char('k') => self.previous(&current_view_services),

                        KeyCode::Tab => {
                            self.show_only_user_config = !self.show_only_user_config;
                            self.list_state.select(Some(0)); // Reset selection to avoid out of bounds
                        }

                        KeyCode::Char('s') => self.perform_action(
                            systemd::ServiceAction::Start,
                            &current_view_services,
                        )?,
                        KeyCode::Char('x') => self
                            .perform_action(systemd::ServiceAction::Stop, &current_view_services)?,
                        KeyCode::Char('r') => self.perform_action(
                            systemd::ServiceAction::Restart,
                            &current_view_services,
                        )?,
                        _ => {}
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.refresh_services()?;
                last_tick = Instant::now();
            }

            if self.should_quit {
                return Ok(());
            }
        }
    }

    fn refresh_services(&mut self) -> Result<()> {
        let new_services = systemd::get_user_services()?;
        self.services = new_services;

        let current_view_len = self.get_current_view_services().len();
        if let Some(selected) = self.list_state.selected() {
            if current_view_len == 0 {
                self.list_state.select(None);
            } else if selected >= current_view_len {
                self.list_state
                    .select(Some(current_view_len.saturating_sub(1)));
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
                // Ignore result here to prevent crash on permission error
                let _ = systemd::control_service(&service.name, action);
                self.refresh_services()?;
            }
        }
        Ok(())
    }
}

