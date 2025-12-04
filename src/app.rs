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
}

impl App {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0)); // Start with first item selected

        Self {
            services: Vec::new(),
            list_state,
            should_quit: false,
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Initial fetch
        self.refresh_services()?;

        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_secs(2); // Auto-refresh every 2 seconds

        loop {
            terminal.draw(|f| ui::render(f, &self.services, &mut self.list_state))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => self.should_quit = true,
                        KeyCode::Char('j') => self.next(),
                        KeyCode::Char('k') => self.previous(),

                        KeyCode::Char('s') => self.perform_action(systemd::ServiceAction::Start)?,
                        KeyCode::Char('x') => self.perform_action(systemd::ServiceAction::Stop)?,
                        KeyCode::Char('r') => {
                            self.perform_action(systemd::ServiceAction::Restart)?
                        }
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

        if let Some(selected) = self.list_state.selected() {
            if selected >= self.services.len() {
                self.list_state
                    .select(Some(self.services.len().saturating_sub(1)));
            }
        }
        Ok(())
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.services.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.services.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn perform_action(&mut self, action: systemd::ServiceAction) -> Result<()> {
        if let Some(index) = self.list_state.selected() {
            if let Some(service) = self.services.get(index) {
                // Ignore result here to prevent crash on permission error,
                // but ideally we would show a popup message.
                let _ = systemd::control_service(&service.name, action);
                self.refresh_services()?;
            }
        }
        Ok(())
    }
}

