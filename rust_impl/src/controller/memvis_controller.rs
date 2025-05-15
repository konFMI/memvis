use crate::cli::console::Console;
use crate::concurrent::atomic_memory::AtomicMemoryReference;
use crate::concurrent::updater::MemoryUpdater;

/// MemvisController manages memory reading and rendering.
pub struct MemvisController {
    memory_updater: MemoryUpdater,
    console: Console,
}

impl MemvisController {
    /// Create a new controller
    ///
    /// - `pid`: Target process ID
    /// - `width`, `height`: UI dimensions
    /// - `start_address`: Optional starting memory address (hex string)
    /// - `use_ptrace`: Whether to use ptrace for stack pointer
    /// - `convert_ascii`: Whether to render memory bytes as ASCII
    pub fn new(
        pid: i32,
        width: usize,
        height: usize,
        start_address: Option<String>,
        use_ptrace: bool,
        convert_ascii: bool,
    ) -> Self {
        let memory_reference = AtomicMemoryReference::new();
        let memory_updater = MemoryUpdater::new(pid, memory_reference.clone(), use_ptrace);

        // If no address provided, fetch stack pointer
        let start_address = match start_address {
            Some(s) => usize::from_str_radix(s.trim_start_matches("0x"), 16)
                .expect("Invalid start address format"),
            None => memory_updater.get_stack_pointer(),
        };

        let console = Console::new(
            pid,
            start_address,
            memory_reference.clone(),
            height,
            width,
            convert_ascii,
        );

        Self {
            memory_updater,
            console,
        }
    }

    /// Start the application: spawn updater, run console, stop updater
    pub fn start(mut self) {
        self.memory_updater.start();
        self.console.start();
        self.memory_updater.stop();
    }
}
