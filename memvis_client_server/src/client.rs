use std::{
    env,
    io::{stdout, Read, Write},
    net::{TcpStream, SocketAddr},
    process,
};

use crossterm::{
    execute,
    cursor::MoveTo,
    event::{self, Event, KeyCode},
    terminal::{enable_raw_mode, disable_raw_mode, Clear, ClearType},
};

use prost::Message;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/memory.rs"));
}

use proto::{Command, MemoryDump};
use proto::command::CommandType;

const DUMP_BUFFER_SIZE: usize = 4096;
const REFRESH_KEY_UP: CommandType = CommandType::Up;
const REFRESH_KEY_DOWN: CommandType = CommandType::Down;
const INIT_COMMAND: CommandType = CommandType::Pid;

fn print_aligned_line(args: std::fmt::Arguments) {
    let mut out = stdout();
    write!(out, "\r{}\n", args).unwrap();
    out.flush().unwrap();
}

macro_rules! println_aligned {
    ($($arg:tt)*) => {
        print_aligned_line(format_args!($($arg)*))
    };
}

fn get_arguments() -> (SocketAddr, u32) {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("❌ Usage: {} <server_address>:<port> <pid>", args[0]);
        process::exit(1);
    }

    let address = args[1].parse().unwrap_or_else(|e| {
        eprintln!("❌ Invalid address '{}': {}", args[1], e);
        process::exit(1);
    });

    let pid = args[2].parse().unwrap_or_else(|_| {
        eprintln!("❌ PID must be a positive integer");
        process::exit(1);
    });

    (address, pid)
}

fn send_command(stream: &mut TcpStream, command_type: CommandType, pid: u32) -> std::io::Result<()> {
    let command = Command {
        command_type: command_type as i32,
        pid,
    };

    let mut buf = Vec::new();
    command.encode(&mut buf)?;
    stream.write_all(&buf)?;
    Ok(())
}

fn read_memory_dump(stream: &mut TcpStream) -> Option<MemoryDump> {
    let mut buffer = vec![0; DUMP_BUFFER_SIZE];
    let size = stream.read(&mut buffer).ok()?;
    MemoryDump::decode(&buffer[..size]).ok()
}

fn handle_input(stream: &mut TcpStream, pid: u32) {
    send_command(stream, INIT_COMMAND, pid).unwrap();
    println_aligned!("Use ↑/↓ to navigate memory. Press 'q' to quit.\n");

    enable_raw_mode().expect("Failed to enable raw mode");

    loop {
        if let Event::Key(event) = event::read().unwrap() {
            let command = match event.code {
                KeyCode::Up => Some(REFRESH_KEY_UP),
                KeyCode::Down => Some(REFRESH_KEY_DOWN),
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    println_aligned!("Exiting...");
                    break;
                }
                _ => None,
            };

            if let Some(cmd_type) = command {
                send_command(stream, cmd_type, pid).unwrap();

                execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).unwrap();

                if let Some(dump) = read_memory_dump(stream) {
                    if dump.status.contains("Insufficient permission") {
                        println_aligned!("{}", dump.status);
                        println_aligned!("Client exiting due to insufficient rights.");
                        disable_raw_mode().expect("Failed to disable raw mode");
                        println_aligned!("");
                        process::exit(1);
                    }

                    println_aligned!("{}", dump.status);

                    println_aligned!(
                        "Region [{}] 0x{:X} - 0x{:X} | Name: {}",
                        dump.region_index,
                        dump.region_start,
                        dump.region_end,
                        dump.region_name
                    );

                    for line in dump.lines.iter() {
                        let clean = line.trim_end_matches(&['\r', '\n'][..]);
                        println_aligned!("{}", clean)
                    }

                } else {
                    println_aligned!("Failed to decode memory dump.");
                }
            }
        }
    }

    disable_raw_mode().expect("Failed to disable raw mode");
    println_aligned!("");
}

fn main() {
    let (address, pid) = get_arguments();

    let mut stream = TcpStream::connect(address).unwrap_or_else(|e| {
        eprintln!("❌ Failed to connect to server at {}: {}", address, e);
        process::exit(1);
    });

    handle_input(&mut stream, pid);
}
