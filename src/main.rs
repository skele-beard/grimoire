use cli_clipboard::ClipboardProvider;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use grimoire::app::{App, CurrentScreen, CurrentlyEditing};
use grimoire::ipc;
use grimoire::ipc::{IpcRequest, IpcResponse};
use grimoire::ui::ui;
use ratatui::backend::Backend;
use ratatui::crossterm::event::DisableMouseCapture;
use ratatui::crossterm::event::EnableMouseCapture;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, enable_raw_mode};
use ratatui::crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::error::Error;
use std::io::Read;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(unix)]
fn start_ipc_server(app: Arc<Mutex<App>>) -> thread::JoinHandle<()> {
    use std::os::unix::net::UnixListener;

    thread::spawn(move || {
        let socket_path = ipc::get_socket_path();

        // Remove existing socket file
        let _ = std::fs::remove_file(&socket_path);

        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to bind Unix socket: {}", e);
                return;
            }
        };

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let app_clone = Arc::clone(&app);
                    thread::spawn(move || {
                        handle_ipc_client(stream, app_clone);
                    });
                }
                Err(e) => {
                    eprintln!("IPC connection error: {}", e);
                }
            }
        }
    })
}

#[cfg(windows)]
fn start_ipc_server(app: Arc<Mutex<App>>) -> thread::JoinHandle<()> {
    use std::fs::File;
    use std::os::windows::io::FromRawHandle;
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::winbase::{CreateNamedPipeA, PIPE_ACCESS_DUPLEX, PIPE_TYPE_BYTE, PIPE_WAIT};

    thread::spawn(move || {
        let pipe_name = ipc::get_pipe_name();
        eprintln!("IPC server listening on {}", pipe_name);

        loop {
            unsafe {
                let pipe_name_cstr = std::ffi::CString::new(pipe_name.clone()).unwrap();
                let handle = CreateNamedPipeA(
                    pipe_name_cstr.as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_BYTE | PIPE_WAIT,
                    255,
                    4096,
                    4096,
                    0,
                    std::ptr::null_mut(),
                );

                if handle == INVALID_HANDLE_VALUE {
                    eprintln!("Failed to create named pipe");
                    return;
                }

                let stream = File::from_raw_handle(handle as *mut _);

                let app_clone = Arc::clone(&app);
                thread::spawn(move || {
                    handle_ipc_client(stream, app_clone);
                });
            }
        }
    })
}

#[cfg(unix)]
fn handle_ipc_client(mut stream: std::os::unix::net::UnixStream, app: Arc<Mutex<App>>) {
    handle_ipc_request(&mut stream, app);
}

#[cfg(windows)]
fn handle_ipc_client(mut stream: std::fs::File, app: Arc<Mutex<App>>) {
    handle_ipc_request(&mut stream, app);
}

fn handle_ipc_request<S: Read + Write>(stream: &mut S, app: Arc<Mutex<App>>) {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 1024];

    loop {
        match stream.read(&mut temp) {
            Ok(0) => break,
            Ok(n) => {
                buffer.extend_from_slice(&temp[..n]);
                if buffer.ends_with(&[b'\n']) {
                    break;
                }
            }
            Err(_) => return,
        }
    }

    if buffer.ends_with(&[b'\n']) {
        buffer.pop();
    }

    let response = match serde_json::from_slice::<IpcRequest>(&buffer) {
        Ok(request) => process_request(request, app),
        Err(e) => IpcResponse {
            ok: false,
            username: None,
            password: None,
            message: None,
            error: Some(format!("Invalid JSON: {}", e)),
        },
    };

    let response_json = serde_json::to_string(&response).unwrap();
    let _ = stream.write_all(response_json.as_bytes());
    let _ = stream.write_all(b"\n");
    let _ = stream.flush();
}

fn process_request(request: IpcRequest, app: Arc<Mutex<App>>) -> IpcResponse {
    match request.action.as_str() {
        "get_credentials" => {
            if let Some(domain) = request.domain {
                let app = app.lock().unwrap();
                if !app.unlocked {
                    IpcResponse {
                        ok: false,
                        username: None,
                        password: None,
                        message: None,
                        error: Some("App is locked".to_string()),
                    }
                } else {
                    match app.get_credentials_for_domain(&domain) {
                        Some((username, password)) => IpcResponse {
                            ok: true,
                            username: Some(username),
                            password: Some(password),
                            message: None,
                            error: None,
                        },
                        None => IpcResponse {
                            ok: false,
                            username: None,
                            password: None,
                            message: None,
                            error: Some("No credentials found for domain".to_string()),
                        },
                    }
                }
            } else {
                IpcResponse {
                    ok: false,
                    username: None,
                    password: None,
                    message: None,
                    error: Some("Domain not specified".to_string()),
                }
            }
        }
        "set_credentials" => {
            if let (Some(domain), Some(username), Some(password)) =
                (request.domain, request.username, request.password)
            {
                let mut app = app.lock().unwrap();
                if !app.unlocked {
                    IpcResponse {
                        ok: false,
                        username: None,
                        password: None,
                        message: None,
                        error: Some("App is locked".to_string()),
                    }
                } else {
                    app.save_credentials_for_domain(&domain, &username, &password);
                    IpcResponse {
                        ok: true,
                        username: None,
                        password: None,
                        message: Some("Credentials saved".to_string()),
                        error: None,
                    }
                }
            } else {
                IpcResponse {
                    ok: false,
                    username: None,
                    password: None,
                    message: None,
                    error: Some("Domain, username, and password must be specified".to_string()),
                }
            }
        }
        "ping" => {
            let app = app.lock().unwrap();
            if app.unlocked {
                IpcResponse {
                    ok: true,
                    username: None,
                    password: None,
                    message: Some("pong".to_string()),
                    error: None,
                }
            } else {
                IpcResponse {
                    ok: false,
                    username: None,
                    password: None,
                    message: None,
                    error: Some("App is locked".to_string()),
                }
            }
        }
        _ => IpcResponse {
            ok: false,
            username: None,
            password: None,
            message: None,
            error: Some("Unknown action".to_string()),
        },
    }
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
                    KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Some(editing) = app.currently_editing.clone() {
                            let len = app.secret_scratch_content.len();
                            match editing {
                                CurrentlyEditing::Name => {
                                    let generated_name = app.generate_password();
                                    app.name_input.push_str(&generated_name);
                                }
                                CurrentlyEditing::Key(idx) => {
                                    let generated_key = app.generate_password();
                                    if idx == len {
                                        app.key_input.push_str(&generated_key)
                                    } else {
                                        app.secret_scratch_content[idx].key.push_str(&generated_key)
                                    }
                                }
                                CurrentlyEditing::Value(idx) => {
                                    let generated_value = app.generate_password();
                                    if idx == len {
                                        app.value_input.push_str(&generated_value)
                                    } else {
                                        app.secret_scratch_content[idx]
                                            .value
                                            .push_str(&generated_value)
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
                    KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Some(editing) = app.currently_editing.clone() {
                            let len = app.secret_scratch_content.len();
                            match editing {
                                CurrentlyEditing::Name => {
                                    let generated_name = app.generate_password();
                                    app.name_input.push_str(&generated_name);
                                }
                                CurrentlyEditing::Key(idx) => {
                                    let generated_key = app.generate_password();
                                    if idx == len {
                                        app.key_input.push_str(&generated_key)
                                    } else {
                                        app.secret_scratch_content[idx].key.push_str(&generated_key)
                                    }
                                }
                                CurrentlyEditing::Value(idx) => {
                                    let generated_value = app.generate_password();
                                    if idx == len {
                                        app.value_input.push_str(&generated_value)
                                    } else {
                                        app.secret_scratch_content[idx]
                                            .value
                                            .push_str(&generated_value)
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

    let app = Arc::new(Mutex::new(App::new()));

    // Start IPC server (NEW)
    let _ipc_handle = start_ipc_server(Arc::clone(&app));

    let _res = run_app(&mut terminal, Arc::clone(&app));

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Clean up socket file on Unix
    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(ipc::get_socket_path());
    }

    Ok(())
}
