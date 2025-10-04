use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::app::{App, CurrentScreen, CurrentlyEditing};
use crate::secret::Secret;

pub fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let title_paragraph = Paragraph::new("Grimoire")
        .block(
            Block::default().borders(Borders::ALL).style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::White),
            ),
        )
        .alignment(Alignment::Center); // optional, to center the title text

    frame.render_widget(title_paragraph, chunks[0]);

    let secrets: &Vec<Secret> = &app.secrets;
    let total_cards = secrets.len();

    let cards_per_row = app.secrets_per_row;
    let row_count = (total_cards + cards_per_row - 1) / cards_per_row;

    let row_constraints = std::iter::repeat(Constraint::Length(9)) // taller rows
        .take(row_count)
        .collect::<Vec<_>>();

    let row_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(row_constraints)
        .split(chunks[1]);

    for (row_idx, row_chunk) in row_chunks.iter().enumerate() {
        let start = row_idx * cards_per_row;
        let end = ((row_idx + 1) * cards_per_row).min(total_cards);
        let cards_in_this_row = end - start;

        let col_constraints = std::iter::repeat(Constraint::Ratio(1, cards_per_row as u32))
            .take(cards_in_this_row)
            .collect::<Vec<_>>();

        let card_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(*row_chunk);

        for (i, secret) in secrets[start..end].iter().enumerate() {
            let global_idx = start + i;
            let is_selected = Some(global_idx) == app.currently_selected_secret_idx;

            let base_style = Style::default().fg(Color::White);
            let highlight_style = Style::default().fg(Color::Black).bg(Color::LightYellow);

            let style = if is_selected {
                highlight_style
            } else {
                base_style
            };

            let lines = vec![
                format!("Username : {}", secret.get_username()),
                format!("Password : {}", secret.get_password()),
            ];

            let paragraph = Paragraph::new(lines.join("\n"))
                .block(
                    Block::default()
                        .title(secret.get_name())
                        .borders(Borders::ALL)
                        .style(style),
                )
                .wrap(Wrap { trim: true });

            frame.render_widget(paragraph, card_chunks[i]);
        }
    }

    let current_keys_hint = {
        match app.current_screen {
            CurrentScreen::Main => Span::styled(
                "(q) to quit / (n) to make new secret",
                Style::default().fg(Color::Red),
            ),
            CurrentScreen::New => Span::styled(
                "(ESC) to cancel/(Tab) to switch boxes/enter to complete",
                Style::default().fg(Color::Red),
            ),
            CurrentScreen::Editing => Span::styled(
                "(q) to quit / (e) to make new pair",
                Style::default().fg(Color::Red),
            ),
        }
    };

    let key_notes_footer =
        Paragraph::new(Line::from(current_keys_hint)).block(Block::default().borders(Borders::ALL));
    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0)]) // Only one chunk for now
        .split(chunks[2]);

    // Render into the first footer chunk
    frame.render_widget(key_notes_footer, footer_chunks[0]);

    // Editing Secret
    if let Some(editing) = &app.currently_editing {
        let popup_block = Block::default()
            .title("Enter secret values")
            .borders(Borders::NONE)
            .style(Style::default())
            .bg(Color::DarkGray);

        let area = centered_rect(60, 25, frame.area());
        frame.render_widget(popup_block, area);

        let popup_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(area);

        let mut name_block = Block::default().title("Name").borders(Borders::ALL);
        let mut username_block = Block::default().title("Username").borders(Borders::ALL);
        let mut password_block = Block::default().title("Password").borders(Borders::ALL);

        let active_style = Style::default().bg(Color::LightYellow).fg(Color::Black);

        match editing {
            CurrentlyEditing::Name => name_block = name_block.style(active_style),
            CurrentlyEditing::Username => username_block = username_block.style(active_style),
            CurrentlyEditing::Password => password_block = password_block.style(active_style),
        };

        let name_text = Paragraph::new(app.name_input.clone()).block(name_block);
        frame.render_widget(name_text, popup_chunks[0]);

        let username_text = Paragraph::new(app.username_input.clone()).block(username_block);
        frame.render_widget(username_text, popup_chunks[1]);

        let password_text = Paragraph::new(app.password_input.clone()).block(password_block);
        frame.render_widget(password_text, popup_chunks[2]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}
