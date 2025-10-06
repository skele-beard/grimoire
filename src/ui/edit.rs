use super::utils::centered_rect;
use crate::app::{App, CurrentlyEditing};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn render_edit_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 25, frame.area());
    let popup = Block::default()
        .title("Enter secret values")
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(popup, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    let active_style = Style::default().bg(Color::Cyan).fg(Color::Black);
    let mut name = Block::default().title("Name").borders(Borders::ALL);
    let mut key = Block::default().title("Key").borders(Borders::ALL);
    let mut value = Block::default().title("Value").borders(Borders::ALL);

    match app.currently_editing {
        Some(CurrentlyEditing::Name) => name = name.style(active_style),
        Some(CurrentlyEditing::Key) => key = key.style(active_style),
        Some(CurrentlyEditing::Value) => value = value.style(active_style),
        None => {}
    }

    frame.render_widget(
        Paragraph::new(app.name_input.clone())
            .wrap(Wrap { trim: true })
            .block(name),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(app.key_input.clone())
            .wrap(Wrap { trim: true })
            .block(key),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(app.value_input.clone())
            .wrap(Wrap { trim: true })
            .block(value),
        chunks[2],
    );
}
