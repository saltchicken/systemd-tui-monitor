// Handles the rendering of widgets to the terminal frame.

use super::model::Service;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

/// Main render function called every frame.
pub fn render(f: &mut Frame, services: &[Service], list_state: &mut ListState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    render_service_list(f, chunks[0], services, list_state);
    render_footer(f, chunks[1]);
}

fn render_service_list(f: &mut Frame, area: Rect, services: &[Service], state: &mut ListState) {
    let items: Vec<ListItem> = services
        .iter()
        .map(|service| {
            let (status_symbol, color) = if service.is_running() {
                ("●", Color::Green)
            } else if service.active_state == "failed" {
                ("✖", Color::Red)
            } else {
                ("○", Color::DarkGray)
            };

            let content = Line::from(vec![
                Span::styled(format!("{} ", status_symbol), Style::default().fg(color)),
                Span::raw(format!("{:<40}", service.name)),
                // This "uses" the field, silencing the warning.
                Span::styled(
                    format!("[{}::{}]", service.loaded_state, service.sub_state),
                    Style::default().fg(Color::Gray),
                ),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" User Systemd Services "),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, state);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::raw("Nav: "),
        Span::styled("j/k ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("| Action: "),
        Span::styled(
            "s(start) x(stop) r(restart) ",
            Style::default().fg(Color::Cyan),
        ),
        Span::raw("| Quit: "),
        Span::styled("q", Style::default().fg(Color::Red)),
    ]);

    let paragraph =
        Paragraph::new(help_text).block(Block::default().borders(Borders::ALL).title(" Controls "));

    f.render_widget(paragraph, area);
}

