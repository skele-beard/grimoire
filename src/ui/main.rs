use crate::app::{App, CurrentScreen};
use crate::secret::Secret;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
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

    let pairs_to_render = secret.get_contents();

    let longest_key = pairs_to_render
        .iter()
        .map(|p| p.key.len())
        .chain(std::iter::once(app.key_input.len()))
        .max()
        .unwrap_or(0);

    let mut text = Text::default();
    for pair in secret.get_contents() {
        text.push_line(Line::from(vec![
            Span::styled(
                format!("{:<width$}", pair.key, width = longest_key),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(" : "),
            Span::raw(pair.value),
        ]));
    }

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(Span::styled(
                    secret.get_name(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .style(style),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

pub fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let hint = match app.current_screen {
        CurrentScreen::Main => "(q) to quit / (n) to make new secret / (/) to search",
        CurrentScreen::Searching => &format!(
            "{} - (Tab) to find next match / (ESC) to cancel",
            &app.scratch
        ),
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
