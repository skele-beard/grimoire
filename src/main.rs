mod app;
mod secret;
mod ui;

use app::{App, CurrentScreen, CurrentlyEditing};
use cli_clipboard::ClipboardProvider;
use crossterm::event::ModifierKeyCode;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use cursive::reexports::time::format_description::modifier;
use ratatui::backend::Backend;
use ratatui::crossterm::event::DisableMouseCapture;
use ratatui::crossterm::event::EnableMouseCapture;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, enable_raw_mode};
use ratatui::crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode};
use ratatui::{Terminal, backend::CrosstermBackend};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::Read;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use ui::ui;

#[derive(Deserialize)]
struct HttpRequest {
    action: String,
    domain: Option<String>,
}

#[derive(Serialize)]
struct HttpResponse {
    ok: bool,
    username: Option<String>,
    password: Option<String>,
    error: Option<String>,
}

fn send_http_response(mut stream: TcpStream, status: &str, body: String) {
    let response = format!(
        "HTTP/1.1 {}\r\n\
         Content-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
        status,
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}

fn handle_http_client(stream: TcpStream, app: Arc<Mutex<App>>) {
    let mut reader = BufReader::new(stream.try_clone().expect("Failed to clone stream"));
    let mut request_line = String::new();

    // Read the request line
    if reader.read_line(&mut request_line).is_err() {
        return;
    }

    // Read headers to find content length
    let mut headers = Vec::new();
    let mut content_length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            return;
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if line.to_lowercase().starts_with("content-length:") {
            if let Some(len_str) = line.split(':').nth(1) {
                content_length = len_str.trim().parse().unwrap_or(0);
            }
        }
        headers.push(line);
    }

    // Handle OPTIONS request (CORS preflight)
    if request_line.starts_with("OPTIONS") {
        send_http_response(stream, "200 OK", String::new());
        return;
    }

    // Read body if present
    let mut body = vec![0u8; content_length];
    if reader.read_exact(&mut body).is_err() {
        return;
    }

    // Parse JSON request
    let response = if content_length > 0 {
        match serde_json::from_slice::<HttpRequest>(&body) {
            Ok(request) => match request.action.as_str() {
                "get_credentials" => {
                    if let Some(domain) = request.domain {
                        let app = app.lock().unwrap();
                        if !app.unlocked {
                            HttpResponse {
                                ok: false,
                                username: None,
                                password: None,
                                error: Some("App is locked".to_string()),
                            }
                        } else {
                            match app.get_credentials_for_domain(&domain) {
                                Some((username, password)) => HttpResponse {
                                    ok: true,
                                    username: Some(username),
                                    password: Some(password),
                                    error: None,
                                },
                                None => HttpResponse {
                                    ok: false,
                                    username: None,
                                    password: None,
                                    error: Some("No credentials found for domain".to_string()),
                                },
                            }
                        }
                    } else {
                        HttpResponse {
                            ok: false,
                            username: None,
                            password: None,
                            error: Some("Domain not specified".to_string()),
                        }
                    }
                }
                "ping" => {
                    let app = app.lock().unwrap();
                    if app.unlocked {
                        HttpResponse {
                            ok: true,
                            username: None,
                            password: None,
                            error: None,
                        }
                    } else {
                        HttpResponse {
                            ok: false,
                            username: None,
                            password: None,
                            error: Some("App is locked".to_string()),
                        }
                    }
                }
                _ => HttpResponse {
                    ok: false,
                    username: None,
                    password: None,
                    error: Some("Unknown action".to_string()),
                },
            },
            Err(e) => HttpResponse {
                ok: false,
                username: None,
                password: None,
                error: Some(format!("Invalid JSON: {}", e)),
            },
        }
    } else {
        HttpResponse {
            ok: false,
            username: None,
            password: None,
            error: Some("Empty request".to_string()),
        }
    };

    let response_json = serde_json::to_string(&response).unwrap();
    send_http_response(stream, "200 OK", response_json);
}

fn start_http_server(app: Arc<Mutex<App>>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let addr = "127.0.0.1:47777";

        let listener = match TcpListener::bind(addr) {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to bind to {}: {}", addr, e);
                eprintln!("Make sure port 47777 is not already in use");
                return;
            }
        };

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let app_clone = Arc::clone(&app);
                    thread::spawn(move || {
                        handle_http_client(stream, app_clone);
                    });
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                }
            }
        }
    })
}

#[allow(clippy::single_match)]
fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: Arc<Mutex<App>>) -> io::Result<bool> {
    loop {
        terminal.draw(|f| {
            let app = app.lock().unwrap();
            ui(f, &app);
        })?;
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                // Skip events that are not KeyEventKind::Press
                continue;
            }
            let mut app = app.lock().unwrap();
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
                        let scratch_clone = app.scratch.clone();
                        let attempt = app.authenticate(&scratch_clone).unwrap();
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
                    KeyCode::Char('x') | KeyCode::Delete => {
                        app.delete_secret();
                        app.clear_input_fields();
                    }
                    KeyCode::Enter => {
                        app.load_secret();
                        app.current_screen = CurrentScreen::Editing;
                        app.currently_editing = Some(CurrentlyEditing::Name);
                    }
                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                        app.select_new_secret(key.code);
                    }
                    KeyCode::Char('/') => {
                        app.clear_input_fields();
                        app.current_screen = CurrentScreen::Searching
                    }
                    _ => {}
                },
                CurrentScreen::Searching => match key.code {
                    KeyCode::Esc => app.current_screen = CurrentScreen::Main,
                    KeyCode::Backspace | KeyCode::Char('\x08') | KeyCode::Char('\x7f') => {
                        if !app.scratch.is_empty() {
                            app.scratch.pop();
                        }
                    }
                    KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let text = app.clipboard.get_contents().unwrap();
                        app.scratch.push_str(&text);
                    }
                    KeyCode::Enter => {
                        app.load_secret();
                        app.current_screen = CurrentScreen::Editing;
                        app.currently_editing = Some(CurrentlyEditing::Name);
                    }
                    KeyCode::Tab => {
                        app.increment_search_buffer();
                    }
                    KeyCode::Char(value) => {
                        app.scratch.push(value);
                        app.search_secrets();
                    }
                    _ => {}
                },
                CurrentScreen::New => match key.code {
                    KeyCode::Esc => {
                        app.current_screen = CurrentScreen::Main;
                        app.save_secret();
                        app.clear_input_fields();
                    }
                    KeyCode::Tab => {
                        app.increment_currently_editing();
                    }
                    KeyCode::Delete => {
                        app.delete_pair();
                    }
                    KeyCode::Enter => {
                        app.add_pair();
                        app.clear_key_value_fields();
                    }
                    KeyCode::BackTab => {
                        app.decrement_currently_editing();
                    }
                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                        app.select_new_pair(key.code);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Some(editing) = app.currently_editing.clone() {
                            let len = app.secret_scratch_content.len();
                            match editing {
                                CurrentlyEditing::Name => {
                                    let text = app.name_input.clone();
                                    app.clipboard.set_contents(text).unwrap();
                                }
                                CurrentlyEditing::Key(idx) => {
                                    if idx == len {
                                        let text = app.key_input.clone();
                                        app.clipboard.set_contents(text).unwrap();
                                    } else {
                                        let text = app.secret_scratch_content[idx].key.clone();
                                        app.clipboard.set_contents(text).unwrap();
                                    }
                                }
                                CurrentlyEditing::Value(idx) => {
                                    if idx == len {
                                        let text = app.value_input.clone();
                                        app.clipboard.set_contents(text).unwrap();
                                    }
                                    let text = app.secret_scratch_content[idx].value.clone();
                                    app.clipboard.set_contents(text).unwrap();
                                }
                            }
                        }
                    }
                    KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Some(editing) = app.currently_editing.clone() {
                            let len = app.secret_scratch_content.len();
                            match editing {
                                CurrentlyEditing::Name => {
                                    let text = app.clipboard.get_contents().unwrap().clone();
                                    app.name_input.push_str(&text)
                                }
                                CurrentlyEditing::Key(idx) => {
                                    let text = app.clipboard.get_contents().unwrap().clone();
                                    if idx == len {
                                        app.key_input.push_str(&text)
                                    } else {
                                        app.secret_scratch_content[idx].key.push_str(&text)
                                    }
                                }
                                CurrentlyEditing::Value(idx) => {
                                    let text = app.clipboard.get_contents().unwrap().clone();
                                    if idx == len {
                                        app.value_input.push_str(&text)
                                    } else {
                                        app.secret_scratch_content[idx].value.push_str(&text)
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Backspace | KeyCode::Char('\x08') | KeyCode::Char('\x7f') => {
                        if let Some(editing) = &app.currently_editing {
                            match editing {
                                CurrentlyEditing::Name => {
                                    app.name_input.pop();
                                }
                                CurrentlyEditing::Key(_) => {
                                    app.key_input.pop();
                                }
                                CurrentlyEditing::Value(_) => {
                                    app.value_input.pop();
                                }
                            }
                        }
                    }
                    KeyCode::Char(value) => {
                        if let Some(editing) = &app.currently_editing {
                            match editing {
                                CurrentlyEditing::Name => app.name_input.push(value),
                                CurrentlyEditing::Key(_) => app.key_input.push(value),
                                CurrentlyEditing::Value(_) => app.value_input.push(value),
                            }
                        }
                    }
                    _ => {}
                },
                CurrentScreen::Editing => match key.code {
                    KeyCode::Esc => {
                        app.current_screen = CurrentScreen::Main;
                        app.update_secret();
                        app.clear_input_fields();
                    }
                    KeyCode::Tab => {
                        app.increment_currently_editing();
                    }
                    KeyCode::Delete => {
                        app.delete_pair();
                    }
                    KeyCode::Enter => {
                        app.add_pair();
                        app.clear_key_value_fields();
                        app.increment_currently_editing();
                    }
                    KeyCode::BackTab => {
                        app.decrement_currently_editing();
                    }
                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                        app.select_new_pair(key.code);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Some(editing) = app.currently_editing.clone() {
                            let len = app.secret_scratch_content.len();
                            match editing {
                                CurrentlyEditing::Name => {
                                    let text = app.name_input.clone();
                                    app.clipboard.set_contents(text).unwrap();
                                }
                                CurrentlyEditing::Key(idx) => {
                                    if idx == len {
                                        let text = app.key_input.clone();
                                        app.clipboard.set_contents(text).unwrap();
                                    } else {
                                        let text = app.secret_scratch_content[idx].key.clone();
                                        app.clipboard.set_contents(text).unwrap();
                                    }
                                }
                                CurrentlyEditing::Value(idx) => {
                                    if idx == len {
                                        let text = app.value_input.clone();
                                        app.clipboard.set_contents(text).unwrap();
                                    }
                                    let text = app.secret_scratch_content[idx].value.clone();
                                    app.clipboard.set_contents(text).unwrap();
                                }
                            }
                        }
                    }
                    KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Some(editing) = app.currently_editing.clone() {
                            let len = app.secret_scratch_content.len();
                            match editing {
                                CurrentlyEditing::Name => {
                                    let text = app.clipboard.get_contents().unwrap().clone();
                                    app.name_input.push_str(&text)
                                }
                                CurrentlyEditing::Key(idx) => {
                                    let text = app.clipboard.get_contents().unwrap().clone();
                                    if idx == len {
                                        app.key_input.push_str(&text)
                                    } else {
                                        app.secret_scratch_content[idx].key.push_str(&text)
                                    }
                                }
                                CurrentlyEditing::Value(idx) => {
                                    let text = app.clipboard.get_contents().unwrap().clone();
                                    if idx == len {
                                        app.value_input.push_str(&text)
                                    } else {
                                        app.secret_scratch_content[idx].value.push_str(&text)
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Backspace | KeyCode::Char('\x08') | KeyCode::Char('\x7f') => {
                        if let Some(editing) = app.currently_editing.clone() {
                            let len = app.secret_scratch_content.len();
                            match editing {
                                CurrentlyEditing::Name => {
                                    app.name_input.pop();
                                }
                                CurrentlyEditing::Key(idx) => {
                                    if idx == len {
                                        app.key_input.pop();
                                    } else {
                                        app.secret_scratch_content[idx].key.pop();
                                    }
                                }
                                CurrentlyEditing::Value(idx) => {
                                    if idx == len {
                                        app.value_input.pop();
                                    } else {
                                        app.secret_scratch_content[idx].value.pop();
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char(value) => {
                        if let Some(editing) = app.currently_editing.clone() {
                            let len = app.secret_scratch_content.len();
                            match editing {
                                CurrentlyEditing::Name => app.name_input.push(value),
                                CurrentlyEditing::Key(idx) => {
                                    if idx == len {
                                        app.key_input.push(value)
                                    } else {
                                        app.secret_scratch_content[idx].key.push(value)
                                    }
                                }
                                CurrentlyEditing::Value(idx) => {
                                    if idx == len {
                                        app.value_input.push(value)
                                    } else {
                                        app.secret_scratch_content[idx].value.push(value)
                                    }
                                }
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
    let app = Arc::new(Mutex::new(App::new()));

    // Start HTTP server in background thread
    let _http_handle = start_http_server(Arc::clone(&app));

    let _res = run_app(&mut terminal, Arc::clone(&app));

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
