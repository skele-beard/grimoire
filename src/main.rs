mod app;
mod config;
mod secret;
mod ui;

use app::{App, CurrentScreen, CurrentlyEditing};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::Backend;
use ratatui::crossterm::event::DisableMouseCapture;
use ratatui::crossterm::event::EnableMouseCapture;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, enable_raw_mode};
use ratatui::crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::error::Error;
use std::io;
use ui::ui;

#[allow(clippy::single_match)]
fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<bool> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                // Skip events that are not KeyEventKind::Press
                continue;
            }
            match app.current_screen {
                CurrentScreen::Init => match key.code {
                    KeyCode::Enter => {
                        app.set_master_password();
                        app.clear_input_fields();
                        app.current_screen = CurrentScreen::Main;
                    }
                    KeyCode::Esc => return Ok(true),
                    KeyCode::Backspace | KeyCode::Char('\x08') | KeyCode::Char('\x7f') => {
                        app.scratch.pop();
                    }
                    KeyCode::Char(value) => {
                        app.scratch.push(value);
                    }
                    _ => {}
                },
                CurrentScreen::Login => match key.code {
                    KeyCode::Enter => {
                        let attempt = app.authenticate(&app.scratch.clone()).unwrap();
                        if attempt {
                            app.current_screen = CurrentScreen::Main;
                        }
                        app.clear_input_fields();
                    }
                    KeyCode::Esc => return Ok(true),
                    KeyCode::Backspace | KeyCode::Char('\x08') | KeyCode::Char('\x7f') => {
                        app.scratch.pop();
                    }
                    KeyCode::Char(value) => {
                        app.scratch.push(value);
                    }
                    _ => {}
                },
                CurrentScreen::Main => match key.code {
                    KeyCode::Char('q') => return Ok(true),
                    KeyCode::Esc => {
                        if let Some(_) = app.currently_selected_secret_idx {
                            app.currently_selected_secret_idx = None;
                        } else {
                            return Ok(true);
                        }
                    }
                    KeyCode::Char('n') => {
                        app.current_screen = CurrentScreen::New;
                        app.currently_editing = Some(CurrentlyEditing::Name);
                    }
                    KeyCode::Enter => {
                        app.current_screen = CurrentScreen::Editing;
                        app.currently_editing = Some(CurrentlyEditing::Name);
                        app.populate_input_fields_from_secret();
                    }
                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                        app.select_new_secret(key.code);
                    }
                    _ => {}
                },
                CurrentScreen::New => match key.code {
                    KeyCode::Esc => {
                        app.current_screen = CurrentScreen::Main;
                        app.clear_input_fields();
                    }
                    KeyCode::Tab => {
                        app.increment_currently_editing();
                    }
                    KeyCode::Enter => {
                        app.add_secret(
                            // This could  be improved by not passing
                            // args since they are included in app's state. For now, I'm going to leave
                            // it this way.
                            &app.name_input.clone(),
                            &app.username_input.clone(),
                            &app.password_input.clone(),
                        );
                        app.current_screen = CurrentScreen::Main;
                        app.clear_input_fields();
                    }
                    KeyCode::BackTab => {
                        app.decrement_currently_editing();
                    }
                    KeyCode::Backspace | KeyCode::Char('\x08') | KeyCode::Char('\x7f') => {
                        if let Some(editing) = &mut app.currently_editing {
                            match editing {
                                CurrentlyEditing::Name => {
                                    app.name_input.pop();
                                }
                                CurrentlyEditing::Username => {
                                    app.username_input.pop();
                                }
                                CurrentlyEditing::Password => {
                                    app.password_input.pop();
                                }
                            }
                        }
                    }
                    KeyCode::Char(value) => {
                        if let Some(editing) = &app.currently_editing {
                            match editing {
                                CurrentlyEditing::Name => app.name_input.push(value),
                                CurrentlyEditing::Username => app.username_input.push(value),
                                CurrentlyEditing::Password => app.password_input.push(value),
                            }
                        }
                    }
                    _ => {}
                },
                CurrentScreen::Editing => match key.code {
                    KeyCode::Esc => {
                        app.current_screen = CurrentScreen::Main;
                        app.clear_input_fields();
                    }
                    KeyCode::Tab => {
                        app.increment_currently_editing();
                    }
                    KeyCode::Enter => {
                        // You need to save the new value
                        app.update_secret();
                        app.clear_input_fields();
                        app.current_screen = CurrentScreen::Main;
                    }
                    KeyCode::BackTab => {
                        app.decrement_currently_editing();
                    }
                    KeyCode::Backspace | KeyCode::Char('\x08') | KeyCode::Char('\x7f') => {
                        if let Some(editing) = &mut app.currently_editing {
                            match editing {
                                CurrentlyEditing::Name => {
                                    app.name_input.pop();
                                }
                                CurrentlyEditing::Username => {
                                    app.username_input.pop();
                                }
                                CurrentlyEditing::Password => {
                                    app.password_input.pop();
                                }
                            }
                        }
                    }
                    KeyCode::Char(value) => {
                        if let Some(editing) = &app.currently_editing {
                            match editing {
                                CurrentlyEditing::Name => app.name_input.push(value),
                                CurrentlyEditing::Username => app.username_input.push(value),
                                CurrentlyEditing::Password => app.password_input.push(value),
                            }
                        }
                    }
                    _ => {}
                },
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::new("1234");
    let _res = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
