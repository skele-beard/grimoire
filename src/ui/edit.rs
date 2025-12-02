use super::utils::centered_rect;
use crate::app::{App, CurrentlyEditing};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

pub fn render_edit_popup(frame: &mut Frame, app: &App) {
    frame.render_widget(Clear, frame.area());
    let full_area = centered_rect(70, 80, frame.area());

    let pairs_to_render = app.secret_scratch_content.clone();
    let name_to_render = app.name_input.clone();

    // Layout: name, spacer, each pair, new entry, hint
    let mut constraints = vec![
        Constraint::Length(3), // name field
        Constraint::Length(1), // spacer
    ];
    constraints.extend(std::iter::repeat(Constraint::Length(3)).take(pairs_to_render.len()));
    constraints.push(Constraint::Length(3)); // new entry
    constraints.push(Constraint::Length(1)); // hint

    let layout_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(constraints)
        .split(full_area);

    // --- Name field ---
    let name_border_style = if matches!(app.currently_editing, Some(CurrentlyEditing::Name)) {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let name_field = Paragraph::new(name_to_render.clone())
        .alignment(Alignment::Center)
        .style(if name_to_render.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Green)
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(name_border_style)
                .title("Name"),
        );

    frame.render_widget(name_field, layout_chunks[0]);

    // Calculate the longest key length (including the new entry inputs)
    let longest_key = pairs_to_render
        .iter()
        .map(|p| p.key.len())
        .chain(std::iter::once(app.key_input.len()))
        .max()
        .unwrap_or(0);

    // --- Pairs ---
    let offset = 2;
    for (i, pair) in pairs_to_render.iter().enumerate() {
        let selected = matches!(app.currently_editing, Some(CurrentlyEditing::Key(idx)) if i == idx)
            || matches!(app.currently_editing, Some(CurrentlyEditing::Value(idx)) if i == idx);

        let border_style = if selected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        let pair_text = Line::from(vec![
            Span::styled(
                format!("{:<width$}", pair.key, width = longest_key),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" : "),
            Span::styled(&pair.value, Style::default().fg(Color::Yellow)),
        ]);

        let pair_block = Paragraph::new(pair_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .border_type(BorderType::Rounded)
                    .title(format!("Entry {}", i + 1)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(pair_block, layout_chunks[i + offset]);
    }

    // --- New entry field ---
    let selected_new_entry = matches!(app.currently_editing, Some(CurrentlyEditing::Key(idx)) if pairs_to_render.len() == idx)
        || matches!(app.currently_editing, Some(CurrentlyEditing::Value(idx)) if pairs_to_render.len() == idx);
    let new_entry_border_style = if selected_new_entry {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let new_entry_text = if app.key_input.is_empty() && app.value_input.is_empty() {
        Line::from(vec![
            Span::styled(
                format!("{:<width$}", "<new key>", width = longest_key),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(" : ", Style::default().fg(Color::DarkGray)),
            Span::styled("<new value>", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!("{:<width$}", app.key_input, width = longest_key),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(" : ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.value_input, Style::default().fg(Color::DarkGray)),
        ])
    };

    let new_entry_block = Paragraph::new(new_entry_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(new_entry_border_style)
            .title("Add new entry"),
    );

    frame.render_widget(
        new_entry_block,
        layout_chunks[pairs_to_render.len() + offset],
    );

    // --- Hint ---
    let hint = Paragraph::new("TAB to move, ENTER to edit, ESC to cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(hint, *layout_chunks.last().unwrap());
}

