use clap::Parser;

use rust_memvis::controller::memvis_controller::MemvisController;

#[derive(Parser)]
#[clap(name = "memvis", about = "A memory visualization tool for Linux processes.")]
struct Args {
    /// Process ID of the target process
    #[arg(short, long, help = "The PID of the target process.")]
    pid: i32,

    /// Starting address for memory visualization (in hexadecimal)
    #[arg(short = 's', long, help = "Start address for memory visualization (optional). Specify in hexadecimal format.")]
    start_address: Option<String>,

    /// Whether to disable ptrace
    #[arg(short, long, help = "Disable ptrace for reading memory.")]
    no_ptrace: bool,

    /// Width of the memory visualization (in number of bytes per row)
    #[arg(short = 'j', long, default_value = "10", help = "Width of the memory visualization (default: 10).")]
    width: usize,

    /// Height of the memory visualization (number of rows)
    #[arg(short = 'i', long, default_value = "26", help = "Height of the memory visualization (default: 26 rows).")]
    height: usize,

    /// Whether to print the raw bytes of the memory
    #[arg(short = 'b', long, help = "Print memory as raw bytes.")]
    print_bytes: bool,
}

fn main() {
    // Initialize logging
    env_logger::init();

    // Parse command-line arguments
    let args = Args::parse();

    // Create and configure the MemvisController with parsed arguments
    let controller = MemvisController::new(
        args.pid,
        args.width,
        args.height,
        args.start_address,
        !args.no_ptrace,
        !args.print_bytes,
    );

    // Start the memory visualization controller
    controller.start();
}
