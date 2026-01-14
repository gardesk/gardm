//! Test client for gardmd authentication
//!
//! Usage: test-auth
//! Prompts for username and password, then tests authentication against gardmd.

use anyhow::Result;
use gardm_ipc::{Request, Response, SOCKET_PATH};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

fn main() -> Result<()> {
    // Connect to daemon
    let stream = UnixStream::connect(SOCKET_PATH)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut stream = stream;

    println!("Connected to gardmd at {}", SOCKET_PATH);

    // Get username
    print!("Username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().lock().read_line(&mut username)?;
    let username = username.trim().to_string();

    // Send CreateSession
    let request = Request::CreateSession { username };
    send_request(&mut stream, &request)?;

    // Read response
    let response = read_response(&mut reader)?;
    println!("Response: {:?}", response);

    match response {
        Response::AuthPrompt { prompt, echo } => {
            // Get password
            if echo {
                print!("{} ", prompt);
                io::stdout().flush()?;
                let mut password = String::new();
                io::stdin().lock().read_line(&mut password)?;
                send_auth(&mut stream, &mut reader, password.trim())?;
            } else {
                print!("{} ", prompt);
                io::stdout().flush()?;
                let password = rpassword::read_password()?;
                send_auth(&mut stream, &mut reader, &password)?;
            }
        }
        Response::Error { message } => {
            eprintln!("Error: {}", message);
            return Ok(());
        }
        _ => {
            eprintln!("Unexpected response: {:?}", response);
            return Ok(());
        }
    }

    Ok(())
}

fn send_request(stream: &mut UnixStream, request: &Request) -> Result<()> {
    let json = serde_json::to_string(request)?;
    writeln!(stream, "{}", json)?;
    stream.flush()?;
    Ok(())
}

fn read_response(reader: &mut BufReader<UnixStream>) -> Result<Response> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let response: Response = serde_json::from_str(&line)?;
    Ok(response)
}

fn send_auth(
    stream: &mut UnixStream,
    reader: &mut BufReader<UnixStream>,
    password: &str,
) -> Result<()> {
    let request = Request::Authenticate {
        response: password.to_string(),
    };
    send_request(stream, &request)?;

    let response = read_response(reader)?;
    match response {
        Response::Success => {
            println!("Authentication successful!");
        }
        Response::AuthError { message } => {
            println!("Authentication failed: {}", message);
        }
        Response::Error { message } => {
            println!("Error: {}", message);
        }
        _ => {
            println!("Unexpected response: {:?}", response);
        }
    }

    Ok(())
}
