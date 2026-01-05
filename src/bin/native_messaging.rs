use grimoire::ipc::{IpcRequest, IpcResponse, send_ipc_request};
use std::io::{Read, Write};

fn main() {
    if let Err(e) = run() {
        eprintln!("Native messaging error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> std::io::Result<()> {
    loop {
        // Read message from browser (stdin)
        let request = match read_native_message() {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Failed to read native message: {}", e);
                break;
            }
        };

        // Forward to main grimoire process via IPC
        let response = match send_ipc_request(&request) {
            Ok(resp) => resp,
            Err(e) => IpcResponse {
                ok: false,
                username: None,
                password: None,
                message: None,
                error: Some(format!("Grimoire is not running: {}", e)),
            },
        };

        // Send response back to browser (stdout)
        if let Err(e) = send_native_message(&response) {
            eprintln!("Failed to send response to browser: {}", e);
            break;
        }
    }

    Ok(())
}

fn read_native_message() -> std::io::Result<IpcRequest> {
    let mut length_bytes = [0u8; 4];
    std::io::stdin().read_exact(&mut length_bytes)?;

    let length = u32::from_ne_bytes(length_bytes) as usize;

    let mut buffer = vec![0u8; length];
    std::io::stdin().read_exact(&mut buffer)?;

    serde_json::from_slice(&buffer)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

fn send_native_message(response: &IpcResponse) -> std::io::Result<()> {
    let json = serde_json::to_string(response)?;
    let bytes = json.as_bytes();
    let length = (bytes.len() as u32).to_ne_bytes();

    std::io::stdout().write_all(&length)?;
    std::io::stdout().write_all(bytes)?;
    std::io::stdout().flush()?;

    Ok(())
}
