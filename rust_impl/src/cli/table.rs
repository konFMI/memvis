use std::fmt::Write;
use crate::memory::reader::MemoryMap;

pub fn render_memory_table(
    start_address: usize,
    bytes: &[u8],
    width: usize,
    height: usize,
    show_ascii: bool,
    meta: Option<MemoryMap>,
) -> String {
    let mut num = 2;
    let mut output = String::new();

    // Handle `meta`, which is Option<MemoryMap>
    let path = match meta {
        Some(map) => map.metadata.path,  // Extract path from the MemoryMap
        None => "No memory region found".to_string(),  // If no MemoryMap, provide a default message
    };

    for row in 0..height {
        let row_start = row * width;
        if row_start >= bytes.len() {
            break;
        }

        let row_end = (row_start + width).min(bytes.len());
        let slice = &bytes[row_start..row_end];

        write!(output, "{}{:#018x}: ",termion::cursor::Goto(1, num), start_address + row_start).unwrap();

        for byte in slice {
            write!(output, "{:02x} ", byte).unwrap();
        }

        for _ in slice.len()..width {
            write!(output, "   ").unwrap();
        }

        if show_ascii {
            write!(output, " |").unwrap();
            for &byte in slice {
                let ch = byte as char;
                let printable = if ch.is_ascii_graphic() || ch == ' ' { ch } else { '.' };
                write!(output, "{}", printable).unwrap();
            }
            write!(output, "|").unwrap();
        }


        // Add the path (name) of the memory region at the end
        write!(output, "   {}", path).unwrap(); // Print the memory region's path/name
        write!(output, "\n{}", termion::cursor::Goto(1, num), ).unwrap();
        num += 1;
    }
    output
}
