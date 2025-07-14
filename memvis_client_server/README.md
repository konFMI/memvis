# 🧠 Rust Memory Inspector

A terminal-based Rust application that lets users inspect and navigate the memory of a Linux process interactively. It uses a TCP client-server architecture with Protocol Buffers for efficient communication and `crossterm` for a sleek terminal UI.

---

## 📁 Project Structure
```
├── build.rs                    # Proto compiler integration
├── Cargo.toml                  # Dependencies & metadata
└── src/ ├── client.rs          # Interactive terminal client
         ├── server.rs          # TCP server that handles memory reading
         └── messages.proto     # Protobuf schema for Command and MemoryDump
```
---

## 🛠️ System Requirements

To compile and run this project successfully:

### 🧰 System
- **Linux OS**: Required for `/proc/<pid>/mem` and `/proc/<pid>/maps`
- **Rust (stable)**: [Install Rust here](https://www.rust-lang.org/tools/install)
- **Protocol Buffers (`protoc`) compiler**:
  ```bash
  sudo apt install protobuf-compiler

🚀 Build & Run

Build the project

``` bash
cargo build --release
```

Start the server
```bash
./target/release/server 127.0.0.1:9000
```
Run the client
```bash
./target/release/client 127.0.0.1:9000 <PID>
```

🔧 Use ↑ / ↓ to scroll memory.
❌ Press q to quit.
🔒 Run with sudo if needed to read restricted memory.

✨ Features
- Terminal memory viewer with smooth key navigation
- Displays hex and ASCII chunks
- Scrollable regions
- Human-readable errors for permission issues
- Modular client-server separation

🧠 How It Works
- client.rs: Sends commands (Up, Down, or Pid) and renders memory dump.
- server.rs: Parses /proc/<pid>/maps, reads memory chunks, sends formatted output.
- Communication via Protocol Buffers (prost)

⚠️ Disclaimer
This tool accesses raw system memory. Use responsibly and ensure proper authorization. Root access may be required depending on the target process.


📄 License

MIT License
Copyright © 2025 konFMI

---




