use std::io::{stdin, stdout, Write};
use std::{thread, time};
use termion::raw::IntoRawMode;
use termion::input::TermRead;
use termion::event::Key;
use termion::{clear, cursor};

use crate::concurrent::atomic_memory::AtomicMemoryReference;
use crate::memory::reader::{MemoryReader, MemoryMap};
use crate::cli::table::render_memory_table;

pub struct Console {
    pid: i32,
    start: usize,
    width: usize,
    height: usize,
    convert_ascii: bool,
    memref: AtomicMemoryReference,
}

impl Console {
    pub fn new(
        pid: i32,
        start: usize,
        memref: AtomicMemoryReference,
        height: usize,
        width: usize,
        convert_ascii: bool,
    ) -> Self {
        Self {
            pid,
            start,
            width,
            height,
            convert_ascii,
            memref,
        }
    }

    pub fn start(&mut self) {
        let input = stdin();  // Get user input from stdin
        let mut stdout = stdout().into_raw_mode().unwrap();  // Set terminal to raw mode for real-time input
        let mut memory_reader = MemoryReader::new(self.pid, true);  // Read memory from the process
        let mut selected_region: Option<MemoryMap> = None;  // Store the selected memory region
        let mut cursor_position: usize = 0;  // Track the selected region in the list
        let mut memory_offset: usize = 0;  // Offset for memory viewing (scrolling)
        let mut num = 1;  // Line number for displaying content

        loop {
            // Clear the screen before rendering new content
            clear_screen();

            // Display the list of available regions
            let regions = memory_reader.read_memory();
            if regions.is_empty() {
                write!(stdout, "No memory regions found.\n").unwrap();
            } else {
                write!(stdout, "{}Available memory regions:\n", termion::cursor::Goto(1, num)).unwrap();
                num += 1;
                for (i, region) in regions.iter().enumerate() {
                    let region_number = i + 1;
                    if cursor_position == i {
                        write!(stdout, "> ").unwrap();  // Highlight the selected region
                    }
                    write!(
                        stdout,
                        "{}{}\t: {:#018x} - {:#018x}, {} \tbytes, \t{}, \t{}\n",
                        termion::cursor::Goto(1, num),
                        region_number,
                        region.metadata.range.start,
                        region.metadata.range.end,
                        region.metadata.size,
                        region.metadata.permissions,
                        region.metadata.path
                    )
                    .unwrap();
                    num += 1;  // Increment line number for each region
                }
                write!(stdout, "{}Press 'q' to quit, or choose a region by number.\n", termion::cursor::Goto(1, num)).unwrap();
                num += 1;
            }

            // Get user input for navigation
            for key in input.lock().keys() {
                match key.unwrap() {
                    Key::Char('q') => {
                        if selected_region.is_none() {
                            return; // Exit the program
                        } else {
                            selected_region = None;
                            memory_offset = 0;
                            break;
                        }
                    }
                    Key::Up => {
                        if cursor_position > 0 {
                            cursor_position -= 1;  // Move the cursor up in the region list
                        }
                    }
                    Key::Down => {
                        if cursor_position < regions.len() - 1 {
                            cursor_position += 1;  // Move the cursor down in the region list
                        }
                    }
                    Key::Char(c) if c.is_digit(10) => {
                        let index = c.to_digit(10).unwrap() as usize - 1;
                        if index < regions.len() {
                            selected_region = Some(regions[index].clone());
                            memory_offset = 0;  // Reset memory offset when selecting a new region
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if let Some(ref region) = selected_region {
                // Clear the screen before rendering the memory content
                clear_screen();
                num = 1;
                write!(stdout, "{}Displaying memory for region: {:#018x} - {:#018x}\n",
                        termion::cursor::Goto(1, num),
                        region.metadata.range.start, 
                        region.metadata.range.end).unwrap();
                num += 1;
            
                // Calculate memory_offset and slice it correctly
                // `region.metadata.range.start` is an address, but `region.memory` is a byte slice.
                // We need to calculate the index into the memory array.
                let memory_len = region.memory.len();
            
                let start_offset = memory_offset;  // This is where we start in the memory slice.
                
                // Calculate the end of the range in memory (start + width * height)
                let end_offset = (start_offset + self.width * self.height).min(memory_len);
            
                // Now slice the memory properly based on the `start_offset` and `end_offset`
                let bytes = &region.memory[start_offset..end_offset];
            
                // Render the memory content
                let table_output = render_memory_table(
                    self.start,
                    bytes,
                    self.width,
                    self.height,
                    self.convert_ascii,
                    Some(region.clone()),  // Pass borrowed region reference
                );
            
                write!(stdout, "{}", table_output).unwrap();
                stdout.flush().unwrap();
            
                // Process user input for navigation within the selected region
                for key in input.lock().keys() {
                    match key.unwrap() {
                        Key::Char('q') => {
                            selected_region = None;
                            memory_offset = 0;
                            break;
                        }
                        Key::Up => {
                            if memory_offset > 0 {
                                memory_offset -= self.width;  // Move the memory window up by one row
                            }
                        }
                        Key::Down => {
                            if memory_offset + (self.width * self.height) < region.memory.len() {
                                memory_offset += self.width;  // Move the memory window down by one row
                            }
                        }
                        _ => {}
                    }
                }
            }
            thread::sleep(time::Duration::from_millis(100));  // Sleep for 100ms for responsive input
        }
    }
}

/// Function to clear the screen and move cursor to the top-left corner
fn clear_screen() {
    let mut stdout = stdout();
    write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1)).unwrap();
    stdout.flush().unwrap();
}
