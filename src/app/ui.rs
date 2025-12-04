// Handles the rendering of widgets to the terminal frame.

use super::model::Service;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

/// Main render function called every frame.

pub fn render(
    f: &mut Frame,
    services: &[Service],
    list_state: &mut ListState,
    show_only_config: bool,
    showing_logs: bool,
    logs: &[String],
    log_scroll: u16,
    stick_to_bottom: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    render_service_list(f, chunks[0], services, list_state, show_only_config);
    render_footer(f, chunks[1], showing_logs);


    if showing_logs {
        render_logs(f, logs, log_scroll, stick_to_bottom);
    }
}

fn render_service_list(
    f: &mut Frame,
    area: Rect,
    services: &[Service],
    state: &mut ListState,
    show_only_config: bool,
) {
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

            let config_indicator = if service.is_user_config { "*" } else { " " };

            let content = Line::from(vec![
                Span::styled(
                    format!("{}{}", config_indicator, status_symbol),
                    Style::default().fg(color),
                ),
                Span::raw(format!(" {:<40}", service.name)),
                Span::styled(
                    format!("[{}::{}]", service.loaded_state, service.sub_state),
                    Style::default().fg(Color::Gray),
                ),
            ]);

            ListItem::new(content)
        })
        .collect();

    let title = if show_only_config {
        " ~/.config/systemd/user Services "
    } else {
        " All User Services "
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, state);
}


fn render_footer(f: &mut Frame, area: Rect, showing_logs: bool) {
    let help_text = if showing_logs {
        Line::from(vec![
            Span::raw("Scroll: "),
            Span::styled("j/k ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("| Auto-Scroll: "),
            Span::styled("G ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("| Close: "),
            Span::styled("Esc/q/l ", Style::default().fg(Color::Red)),
        ])
    } else {
        Line::from(vec![
            Span::raw("Nav: "),
            Span::styled("j/k ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("| View: "),
            Span::styled("Tab ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("| Logs: "),
            Span::styled("l ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("| Action: "),
            Span::styled(
                "s(start) x(stop) r(restart) ",
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("| Quit: "),
            Span::styled("q", Style::default().fg(Color::Red)),
        ])
    };

    let paragraph =
        Paragraph::new(help_text).block(Block::default().borders(Borders::ALL).title(" Controls "));

    f.render_widget(paragraph, area);
}


fn render_logs(f: &mut Frame, logs: &[String], scroll: u16, stick_to_bottom: bool) {
    let area = centered_rect(80, 80, f.area());

    f.render_widget(Clear, area);


    let title = if stick_to_bottom {
        " Service Logs (Live | Auto-scroll: ON) - Press 'j/k' to pause "
    } else {
        " Service Logs (Paused | Auto-scroll: OFF) - Press 'G' to resume "
    };

    let block = Block::default().borders(Borders::ALL).title(title);

    let content: Vec<Line> = logs.iter().map(|s| Line::from(s.as_str())).collect();

    let paragraph = Paragraph::new(content).block(block).scroll((scroll, 0));

    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}