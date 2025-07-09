use std::{
    env,
    io::{Write, BufRead, BufReader, stdout},
    net::{TcpStream, SocketAddr},
    process,
};

use crossterm::{
    execute,
    cursor::MoveTo,
    event::{self, Event, KeyCode},
    terminal::{enable_raw_mode, disable_raw_mode, Clear, ClearType},
};

/// Validates and retrieves the server address and PID from command-line arguments
fn get_arguments() -> (SocketAddr, u32) {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("❌ Usage: {} <server_address>:<port> <pid>", args[0]);
        process::exit(1);
    }

    let address = match args[1].parse::<SocketAddr>() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("❌ Invalid address '{}': {}", args[1], e);
            process::exit(1);
        }
    };

    let pid = match args[2].parse::<u32>() {
        Ok(pid) => pid,
        Err(_) => {
            eprintln!("❌ PID must be a positive integer");
            process::exit(1);
        }
    };

    (address, pid)
}

/// Handles interactive memory navigation
fn handle_input(stream: &mut TcpStream, pid: u32) {
    // Send PID to server
    stream.write_all(format!("PID {}\n", pid).as_bytes()).unwrap();

    // Read and discard server handshake message
    let reader = BufReader::new(stream.try_clone().unwrap());
    for line in reader.lines() {
        match line {
            Ok(text) => {
                if text == "END" {
                    break;
                }
                // Optional: silently discard or log the message
            }
            Err(e) => {
                eprintln!("Handshake failed: {}", e);
                return;
            }
        }
    }

    // Now that handshake is complete, show instructions
    println!("Use ↑/↓ to navigate memory. Press 'q' to quit.");


    enable_raw_mode().expect("Failed to enable raw mode");

    loop {
        // Listen for key events
        if let Event::Key(key_event) = event::read().unwrap() {
            let command = match key_event.code {
                KeyCode::Up => Some("UP\n"),
                KeyCode::Down => Some("DOWN\n"),
                KeyCode::Char('q') => {
                    println!("Exiting...");
                    break;
                }
                _ => None,
            };

            if let Some(cmd) = command {
                // Send command to server
                if let Err(e) = stream.write_all(cmd.as_bytes()) {
                    eprintln!("Failed to send command: {}", e);
                    break;
                }

                // Clear screen before showing updated memory
                execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).unwrap();

                let reader = BufReader::new(stream.try_clone().unwrap());

                // Print each line until "END"
                for line in reader.lines() {
                    match line {
                        Ok(text) => {
                            if text == "END" {
                                break;
                            }
                            print!("{}\r\n", text);
                        }
                        Err(e) => {
                            eprintln!("Failed to read line: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    }

    disable_raw_mode().expect("Failed to disable raw mode");
}

/// Main entry point
fn main() {
    let (address, pid) = get_arguments();

    let mut stream = match TcpStream::connect(address) {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("❌ Failed to connect to server at {}: {}", address, e);
            process::exit(1);
        }
    };

    handle_input(&mut stream, pid);
}