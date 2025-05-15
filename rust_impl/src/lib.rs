pub mod controller {
    pub mod memvis_controller;
}

pub mod cli {
    pub mod console;
    pub mod table;
}

pub mod memory {
    pub mod reader;
    pub mod stack_pointer;
}

pub mod concurrent {
    pub mod atomic_memory;
    pub mod updater;
}
