use crate::app::{App, CurrentScreen};
use crate::secret::Secret;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn render_main(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_title(frame, chunks[0]);
    render_secret_grid(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);
}

pub fn render_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new("Grimoire")
        .block(
            Block::default().borders(Borders::ALL).style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::White),
            ),
        )
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(title, area);
}

pub fn render_secret_grid(frame: &mut Frame, app: &App, area: Rect) {
    let secrets = &app.secrets;
    let total = secrets.len();
    let cols = app.secrets_per_row;
    let rows = (total + cols - 1) / cols;

    let row_constraints = vec![Constraint::Length(9); rows];
    let row_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(row_constraints)
        .split(area);

    for (row_idx, row_chunk) in row_chunks.iter().enumerate() {
        let start = row_idx * cols;
        let end = ((row_idx + 1) * cols).min(total);
        let row = &secrets[start..end];

        let col_constraints = vec![Constraint::Ratio(1, cols as u32); row.len()];
        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(*row_chunk);

        for (i, secret) in row.iter().enumerate() {
            render_secret_card(frame, app, secret, start + i, col_chunks[i]);
        }
    }
}

pub fn render_secret_card(frame: &mut Frame, app: &App, secret: &Secret, idx: usize, area: Rect) {
    let selected = Some(idx) == app.currently_selected_secret_idx;
    let style = if selected {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let lines = format!(
        "Username : {}\nPassword : {}",
        secret.get_username(),
        secret.get_password()
    );

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(secret.get_name())
                .borders(Borders::ALL)
                .style(style),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

pub fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let hint = match app.current_screen {
        CurrentScreen::Main => "(q) to quit / (n) to make new secret",
        CurrentScreen::New => "(ESC) cancel / (Tab) switch / Enter complete",
        CurrentScreen::Editing => "(q) quit / (e) new pair",
        _ => "",
    };

    let footer = Paragraph::new(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::Red),
    )))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, area);
}
