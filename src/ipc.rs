use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

#[derive(Deserialize, Serialize)]
pub struct IpcRequest {
    pub action: String,
    pub domain: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct IpcResponse {
    pub ok: bool,
    pub username: Option<String>,
    pub password: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[cfg(unix)]
pub fn get_socket_path() -> String {
    "/tmp/grimoire.sock".to_string()
}

#[cfg(windows)]
pub fn get_pipe_name() -> String {
    r"\\.\pipe\grimoire".to_string()
}

// Send request and receive response over IPC
#[cfg(unix)]
pub fn send_ipc_request(request: &IpcRequest) -> std::io::Result<IpcResponse> {
    use std::os::unix::net::UnixStream;

    let socket_path = get_socket_path();
    let mut stream = UnixStream::connect(socket_path)?;

    let request_json = serde_json::to_string(request)?;
    stream.write_all(request_json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut response_buffer = Vec::new();
    let mut temp = [0u8; 1024];
    loop {
        match stream.read(&mut temp) {
            Ok(0) => break,
            Ok(n) => {
                response_buffer.extend_from_slice(&temp[..n]);
                if response_buffer.ends_with(&[b'\n']) {
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }

    if response_buffer.ends_with(&[b'\n']) {
        response_buffer.pop();
    }

    serde_json::from_slice(&response_buffer)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(windows)]
pub fn send_ipc_request(request: &IpcRequest) -> std::io::Result<IpcResponse> {
    use std::fs::OpenOptions;

    let pipe_name = get_pipe_name();
    let mut stream = OpenOptions::new().read(true).write(true).open(pipe_name)?;

    let request_json = serde_json::to_string(request)?;
    stream.write_all(request_json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut response_buffer = Vec::new();
    let mut temp = [0u8; 1024];
    loop {
        match stream.read(&mut temp) {
            Ok(0) => break,
            Ok(n) => {
                response_buffer.extend_from_slice(&temp[..n]);
                if response_buffer.ends_with(&[b'\n']) {
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }

    if response_buffer.ends_with(&[b'\n']) {
        response_buffer.pop();
    }

    serde_json::from_slice(&response_buffer)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
