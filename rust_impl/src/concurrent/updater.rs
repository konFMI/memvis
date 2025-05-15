use std::thread::{self, JoinHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::concurrent::atomic_memory::AtomicMemoryReference;
use crate::memory::reader::MemoryReader;

pub struct MemoryUpdater {
    thread: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
    memory_reference: AtomicMemoryReference,
    reader: MemoryReader,
}

impl MemoryUpdater {
    pub fn new(pid: i32, memory_reference: AtomicMemoryReference, use_ptrace: bool) -> Self {
        Self {
            thread: None,
            running: Arc::new(AtomicBool::new(false)),
            memory_reference,
            reader: MemoryReader::new(pid, use_ptrace),
        }
    }

    pub fn start(&mut self) {
        let running = self.running.clone();
        let memory_reference = self.memory_reference.clone();
        let mut reader = self.reader.clone();

        running.store(true, Ordering::SeqCst);
        self.thread = Some(thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                let maps = reader.read_memory();
                memory_reference.set_maps(maps);
                thread::sleep(Duration::from_secs(5));
            }
        }));
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn get_stack_pointer(&self) -> usize {
        self.reader.get_stack_pointer()
    }
}
