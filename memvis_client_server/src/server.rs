use std::{
    env,
    fs::File,
    io::{BufRead, BufReader, Read, Seek, Write},
    net::{TcpListener, TcpStream, SocketAddr},
    process,
};

use prost::Message;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/memory.rs"));
}

use proto::{Command, MemoryDump};
use proto::command::CommandType;

const CHUNK_SIZE: usize = 128;             // Number of bytes sent per memory response
const BYTES_PER_LINE: usize = 16;          // Number of bytes displayed per formatted line
const HEX_DISPLAY_WIDTH: usize = 3 * BYTES_PER_LINE; // Width of hex section: 2 digits + space
const ASCII_WIDTH: usize = BYTES_PER_LINE; // Width of ASCII section
const MAX_BUFFER_SIZE: usize = 1024;       // Incoming buffer size per message

struct ClientState {
    pid: u32,
    region_index: usize,
    offset_within_region: usize,
    mem_regions: Vec<(usize, usize, String)>,
}

// 🎯 Parse command-line arguments
fn get_server_address() -> SocketAddr {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("❌ Usage: {} <client_address>:<port>", args[0]);
        process::exit(1);
    }
    args[1].parse().unwrap_or_else(|e| {
        eprintln!("❌ Invalid address '{}': {}", args[1], e);
        process::exit(1);
    })
}

// 📍 Parse /proc/<pid>/maps for memory regions
fn parse_maps(pid: u32) -> Vec<(usize, usize, String)> {
    let path = format!("/proc/{}/maps", pid);
    let reader = BufReader::new(File::open(path).expect("Failed to open maps file"));

    reader.lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            let (range, perms) = (parts.get(0)?, parts.get(1)?);
            if !perms.contains('r') {
                return None;
            }
            let mut range_parts = range.split('-');
            let start = usize::from_str_radix(range_parts.next()?, 16).ok()?;
            let end = usize::from_str_radix(range_parts.next()?, 16).ok()?;
            let name = parts.get(5).unwrap_or(&"[anonymous]").to_string();
            Some((start, end, name))
        })
        .collect()
}

// 🧾 Format raw memory bytes into aligned hex + ASCII strings
fn format_memory_chunk(address: usize, data: &[u8]) -> Vec<String> {
    data.chunks(BYTES_PER_LINE)
        .enumerate()
        .map(|(i, chunk)| {
            let line_addr = address + i * BYTES_PER_LINE;
            let hex: String = chunk.iter().map(|b| format!("{:02X} ", b)).collect();
            let hex_padded = format!("{:<width$}", hex, width = HEX_DISPLAY_WIDTH);
            let ascii: String = chunk
                .iter()
                .map(|&b| if (0x20..=0x7E).contains(&b) { b as char } else { '.' })
                .collect();
            format!(
                "Addr: 0x{:08X} | +{:03} | {} | {:<width$}",
                line_addr,
                i * BYTES_PER_LINE,
                hex_padded,
                ascii,
                width = ASCII_WIDTH
            )
        })
        .collect()
}

// 🧠 Read memory from /proc/<pid>/mem
fn read_memory(pid: u32, address: usize, size: usize) -> Result<Vec<u8>, std::io::Error> {
    let path = format!("/proc/{}/mem", pid);
    let mut file = File::open(path)?;
    file.seek(std::io::SeekFrom::Start(address as u64))?;
    let mut buffer = vec![0; size];
    file.read_exact(&mut buffer)?;
    Ok(buffer)
}


// ↕️ Update client state based on command
fn process_command(cmd: i32, state: &mut ClientState) {
    match CommandType::try_from(cmd) {
        Ok(CommandType::Up) => {
            if state.offset_within_region >= CHUNK_SIZE {
                state.offset_within_region -= CHUNK_SIZE;
            } else if state.region_index > 0 {
                state.region_index -= 1;
                let (start, end, _) = state.mem_regions[state.region_index];
                state.offset_within_region = (end - start).saturating_sub(CHUNK_SIZE);
            }
        }
        Ok(CommandType::Down) => {
            let (start, end, _) = state.mem_regions[state.region_index];
            let region_size = end - start;
            if state.offset_within_region + CHUNK_SIZE < region_size {
                state.offset_within_region += CHUNK_SIZE;
            } else if state.region_index + 1 < state.mem_regions.len() {
                state.region_index += 1;
                state.offset_within_region = 0;
            }
        }
        Ok(_) => {} // PID or Unknown
        Err(e) => eprintln!("Invalid command type: {} ({})", cmd, e),
    }
}

// 📦 Build and send memory dump as Protobuf
fn send_memory_dump(state: &ClientState, stream: &mut TcpStream) -> std::io::Result<()> {
    let (start, end, name) = &state.mem_regions[state.region_index];
    let address = start + state.offset_within_region;

    match read_memory(state.pid, address, CHUNK_SIZE) {
        Ok(data) => {
            let region_size = end - start;
            let progress = address - start;
            let percent = ((progress as f64 / region_size as f64) * 100.0).round();

            let dump = MemoryDump {
                status: format!("Progress: {:.0}% | Offset: {} / {} bytes", percent, progress, region_size),
                region_name: name.clone(),
                region_index: state.region_index as u32,
                region_start: *start as u64,
                region_end: *end as u64,
                lines: format_memory_chunk(address, &data),
            };

            let mut buf = Vec::new();
            dump.encode(&mut buf)?;
            stream.write_all(&buf)?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            let error_msg = format!("🚫 Insufficient permission to read memory of PID {}. Run with sudo or check access rights.", state.pid);
            let dump = MemoryDump {
                status: error_msg.clone(),  // clone here
                region_name: String::new(),
                region_index: 0,
                region_start: 0,
                region_end: 0,
                lines: vec![],
            };
            let mut buf = Vec::new();
            dump.encode(&mut buf)?;
            stream.write_all(&buf)?;
            eprintln!("{}", error_msg);      // original still usable
        }
        Err(e) => {
            eprintln!("Failed to read memory: {}", e);
        }
    }


    Ok(())
}

// 🔄 Handle one client session
fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; MAX_BUFFER_SIZE];
    let mut state: Option<ClientState> = None;

    while let Ok(n) = stream.read(&mut buffer) {
        if n == 0 {
            break;
        }

        let cmd = match Command::decode(&buffer[..n]) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to decode command: {}", e);
                break;
            }
        };

        match CommandType::try_from(cmd.command_type) {
            Ok(CommandType::Pid) if state.is_none() => {
                let regions = parse_maps(cmd.pid);
                if regions.is_empty() {
                    eprintln!("No readable memory regions found.");
                    continue;
                }
                state = Some(ClientState {
                    pid: cmd.pid,
                    region_index: 0,
                    offset_within_region: 0,
                    mem_regions: regions,
                });
            }
            Ok(_) if state.is_some() => {
                let s = state.as_mut().unwrap();
                process_command(cmd.command_type, s);
                let _ = send_memory_dump(s, &mut stream);
            }
            Err(_) => {
                eprintln!("Unknown command type: {}", cmd.command_type);
                continue;
            }
            _ => {}
        }
    }
}

// 🚀 Entry point
fn main() {
    let address = get_server_address();
    let listener = TcpListener::bind(address).expect("Could not bind");
    println!("🚀 Server listening on {}", address);

    for stream in listener.incoming() {
        match stream {
            Ok(s) => handle_client(s),
            Err(e) => eprintln!("🔴 Connection error: {}", e),
        }
    }
}
