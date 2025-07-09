use std::{
    io::{Read, Write, BufRead, BufReader, Seek},
    net::{TcpListener, TcpStream, SocketAddr},
    fs::File,
    env,
    process,
};

const CHUNK_SIZE: usize = 128;

/// Stores client state across commands
struct ClientState {
    pid: u32,
    region_index: usize,
    offset_within_region: usize,
    mem_regions: Vec<(usize, usize, String)>, // (start, end, name)
}

fn get_server_address() -> SocketAddr {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("❌ Usage: {} <client_address>:<port>", args[0]);
        process::exit(1);
    }

    match args[1].parse::<SocketAddr>() {
        Ok(addr) => {
            println!("✅ Valid address: {}", addr);
            addr
        }
        Err(e) => {
            eprintln!("❌ Invalid address format '{}': {}", args[1], e);
            process::exit(1);
        }
    }
}

/// Parses the /proc/<pid>/maps file for readable memory regions
fn parse_maps(pid: u32) -> Vec<(usize, usize, String)> {
    let path = format!("/proc/{}/maps", pid);
    let file = File::open(path).expect("Failed to open maps file");
    let reader = BufReader::new(file);

    reader.lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            let range = parts.get(0)?;
            let perms = parts.get(1)?;

            // Skip regions without read permission
            if !perms.contains('r') {
                return None;
            }

            let mut range_parts = range.split('-');
            let start = usize::from_str_radix(range_parts.next()?, 16).ok()?;
            let end = usize::from_str_radix(range_parts.next()?, 16).ok()?;

            // Grab path or use "[anonymous]"
            let name = parts.get(5).unwrap_or(&"[anonymous]").to_string();

            Some((start, end, name))
        })
        .collect()
}

/// Formats a memory chunk into lines of hex and ASCII representations.
fn format_memory_chunk(address: usize, data: &[u8]) -> Vec<String> {
    let mut lines = Vec::new();

    for i in 0..(data.len() / 16) {
        let line_addr = address + i * 16;
        let line_data = &data[i * 16..(i + 1) * 16];

        let hex = line_data
            .iter()
            .map(|b| format!("{:02X} ", b))
            .collect::<String>()
            .trim_end()
            .to_string();

        let ascii = line_data
            .iter()
            .map(|b| {
                let c = *b as char;
                if c.is_ascii_graphic() || c == ' ' {
                    c
                } else {
                    '.'
                }
            })
            .collect::<String>();

        let line = format!(
            "Addr: 0x{:08X} | +{:03} | {:<48} | {}",
            line_addr,
            i * 16,
            hex,
            ascii
        );

        lines.push(line);
    }

    lines
}

/// Reads memory from /proc/<pid>/mem at a given address
fn read_memory(pid: u32, address: usize, size: usize) -> Option<Vec<u8>> {
    let path = format!("/proc/{}/mem", pid);
    let mut file = File::open(path).ok()?;
    file.seek(std::io::SeekFrom::Start(address as u64)).ok()?;

    let mut buffer = vec![0; size];
    file.read_exact(&mut buffer).ok()?;
    Some(buffer)
}

/// Sends formatted memory dump to the client, including progress and region info.
fn send_memory_dump(state: &ClientState, stream: &mut TcpStream) {
    let (start, end, name) = &state.mem_regions[state.region_index];
    let address = start + state.offset_within_region;

    match read_memory(state.pid, address, CHUNK_SIZE) {
        Some(data) => {
            let region_size = end - start;
            let progress = address - start;
            let percentage = ((progress as f64 / region_size as f64) * 100.0).round();

            let status_line = format!(
                "Progress through region: {:.0}% | Offset: {} / {} bytes\n",
                percentage, progress, region_size
            );
            let _ = stream.write_all(status_line.as_bytes());

            let header = format!(
                "Range[{}]: 0x{:X}-0x{:X} | Name: {}\n",
                state.region_index, start, end, name
            );
            let _ = stream.write_all(header.as_bytes());

            let formatted_lines = format_memory_chunk(address, &data);
            for line in formatted_lines {
                let _ = stream.write_all(format!("{}\n", line).as_bytes());
            }

            let _ = stream.write_all(b"END\n");
        }
        None => {
            let _ = stream.write_all(b"Failed to read memory.\nEND\n");
        }
    }
}
/// Processes user commands from the client
fn process_command(input: &str, state: &mut ClientState) -> bool {
    match input {
        "UP" => {
            if state.offset_within_region >= CHUNK_SIZE {
                state.offset_within_region -= CHUNK_SIZE;
            } else if state.region_index > 0 {
                state.region_index -= 1;
                let (start, end, _) = state.mem_regions[state.region_index];
                let region_size = end - start;
                state.offset_within_region = region_size.saturating_sub(CHUNK_SIZE);
            } else {
                return false; // Beginning reached
            }
        }
        "DOWN" => {
            let (start, end, _) = state.mem_regions[state.region_index];
            let region_size = end - start;
            if state.offset_within_region + CHUNK_SIZE < region_size {
                state.offset_within_region += CHUNK_SIZE;
            } else if state.region_index + 1 < state.mem_regions.len() {
                state.region_index += 1;
                state.offset_within_region = 0;
            } else {
                return false; // End reached
            }
        }
        _ => {}
    }

    true
}

/// Handles an individual client connection
fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut state: Option<ClientState> = None;

    loop {
        let size = match stream.read(&mut buffer) {
            Ok(0) => break, // Disconnected
            Ok(n) => n,
            Err(_) => break,
        };

        let input = String::from_utf8_lossy(&buffer[..size]).trim().to_string();

        if input.starts_with("PID") {
            if let Some(pid_str) = input.split_whitespace().nth(1) {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    let regions = parse_maps(pid);
                    if regions.is_empty() {
                        let _ = stream.write_all(b"Could not read memory regions.\nEND\n");
                        continue;
                    }

                    state = Some(ClientState {
                        pid,
                        region_index: 0,
                        offset_within_region: 0,
                        mem_regions: regions,
                    });

                    let _ = stream.write_all(b"PID received. Use UP/DOWN to navigate.\nEND\n");
                }
            }
        } else if let Some(ref mut s) = state {
            if !process_command(&input, s) {
                let _ = stream.write_all(b"Memory boundary reached.\nEND\n");
                continue;
            }
            send_memory_dump(s, &mut stream);
        }
    }
}

/// Main entry point: sets up server and listens for connections
fn main() {
    let address = get_server_address();

    let listener = TcpListener::bind(address).expect("Could not bind!");
    println!("🚀 Server listening on {}", address);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream),
            Err(e) => eprintln!("🔴 Error accepting connection: {}", e),
        }
    }
}
