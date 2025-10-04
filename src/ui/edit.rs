use super::utils::centered_rect;
use crate::app::{App, CurrentlyEditing};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
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

    let active_style = Style::default().bg(Color::LightYellow).fg(Color::Black);
    let mut name = Block::default().title("Name").borders(Borders::ALL);
    let mut user = Block::default().title("Username").borders(Borders::ALL);
    let mut pass = Block::default().title("Password").borders(Borders::ALL);

    match app.currently_editing {
        Some(CurrentlyEditing::Name) => name = name.style(active_style),
        Some(CurrentlyEditing::Username) => user = user.style(active_style),
        Some(CurrentlyEditing::Password) => pass = pass.style(active_style),
        None => {}
    }

    frame.render_widget(
        Paragraph::new(app.name_input.clone()).block(name),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(app.username_input.clone()).block(user),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(app.password_input.clone()).block(pass),
        chunks[2],
    );
}
