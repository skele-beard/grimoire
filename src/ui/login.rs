use super::utils::centered_rect;
use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

const LOCK_ART: &str = include_str!("../../assets/lock.txt");
const TITLE_ART: &str = include_str!("../../assets/title.txt");

pub fn render_login(frame: &mut Frame, app: &App) {
    frame.render_widget(Clear, frame.area());
    let full_area = centered_rect(80, 80, frame.area()); // taller to fit art below

    // Split vertically: top = block, bottom = lock art
    let layout_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(40),
            Constraint::Percentage(40),
        ])
        .split(full_area);

    let title_area = layout_chunks[0];
    let lock_area = layout_chunks[1];
    let block_area = layout_chunks[2];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // input
            Constraint::Length(1), // spacer
            Constraint::Length(1), // hint
        ])
        .split(block_area);

    // masked input
    let masked_input = "*".repeat(app.scratch.len());
    let input_paragraph = Paragraph::new(masked_input)
        .style(Style::default().fg(Color::Yellow))
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
                .border_type(BorderType::Rounded)
                .title("Master Password"),
        );
    frame.render_widget(input_paragraph, chunks[0]);

    // hint
    let hint = Paragraph::new("Press Enter to unlock, or ESC to quit.")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(hint, chunks[2]);

    let title_text = Paragraph::new(Text::from(TITLE_ART))
        .style(Style::default().fg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(title_text, title_area);

    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // left spacer
            Constraint::Percentage(30), // center column (ASCII art)
            Constraint::Percentage(35), // right spacer
        ])
        .split(lock_area);

    let art_paragraph = Paragraph::new(LOCK_ART)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::SLOW_BLINK),
        )
        .wrap(Wrap { trim: false })
        .alignment(ratatui::layout::Alignment::Left);

    frame.render_widget(art_paragraph, horizontal_chunks[1]);
}
