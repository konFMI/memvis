use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

use crate::memory::stack_pointer::{StackPointerReader, PtraceStackPointerReader, SyscallStackPointerReader};

#[derive(Clone, Debug)]
pub struct AddressRange {
    pub start: usize,
    pub end: usize,
}

impl AddressRange {
    pub fn contains(&self, address: usize) -> bool {
        self.start <= address && address < self.end
    }
}

impl PartialEq for AddressRange {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}
impl Eq for AddressRange {}

impl PartialOrd for AddressRange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.start.cmp(&other.start))
    }
}
impl Ord for AddressRange {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start.cmp(&other.start)
    }
}
impl std::hash::Hash for AddressRange {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.end.hash(state);
    }
}

#[derive(Clone, Debug)]
pub struct AddressSpaceMetadata {
    pub range: AddressRange,
    pub permissions: String,
    pub offset: usize,
    pub device: String,
    pub inode: usize,
    pub path: String,
    pub size: usize,
}

#[derive(Clone, Debug)]
pub struct MemoryMap {
    pub pid: i32,
    pub metadata: AddressSpaceMetadata,
    pub memory: Vec<u8>,
}

impl MemoryMap {
    pub fn slice(&self, start: usize, end: usize) -> &[u8] {
        let offset = start.saturating_sub(self.metadata.range.start);
        let size = end.saturating_sub(start).min(self.memory.len().saturating_sub(offset));
        &self.memory[offset..offset + size]
    }
}

#[derive(Clone)]
pub struct MemoryReader {
    pid: i32,
    maps: Vec<AddressSpaceMetadata>,
    stack_reader: Box<dyn StackPointerReader>,
}

impl MemoryReader {
    pub fn new(pid: i32, use_ptrace: bool) -> Self {
        let stack_reader: Box<dyn StackPointerReader> = if use_ptrace {
            Box::new(PtraceStackPointerReader {})
        } else {
            Box::new(SyscallStackPointerReader {})
        };

        let mut reader = MemoryReader {
            pid,
            maps: vec![],
            stack_reader,
        };

        reader.refresh_maps();
        reader
    }

    pub fn get_stack_pointer(&self) -> usize {
        self.stack_reader.read(self.pid)
    }

    pub fn read_memory(&mut self) -> Vec<MemoryMap> {
        self.refresh_maps();
        let mut maps = Vec::new();
        for meta in &self.maps {
            if meta.permissions.contains("r") {
                let memory = self.read_memory_segment(meta);
                maps.push(MemoryMap {
                    pid: self.pid,
                    metadata: meta.clone(),
                    memory,
                });
            }
        }
        maps
    }

    fn refresh_maps(&mut self) {
        let path = format!("/proc/{}/maps", self.pid);
        let file = File::open(&path).expect("Could not open /proc/[pid]/maps");
        let reader = BufReader::new(file);

        self.maps = reader
            .lines()
            .filter_map(|line| line.ok().and_then(|l| parse_maps_line(&l)))
            .collect();
    }

    fn read_memory_segment(&self, meta: &AddressSpaceMetadata) -> Vec<u8> {
        let mem_path = format!("/proc/{}/mem", self.pid);
        let mut file = match File::open(&mem_path) {
            Ok(f) => f,
            Err(_) => return vec![0; meta.size],
        };

        let mut buffer = vec![0; meta.size];
        if file.seek(SeekFrom::Start(meta.range.start as u64)).is_ok() {
            if let Err(_) = file.read_exact(&mut buffer) {
                return vec![0; meta.size];
            }
        }

        buffer
    }
}

fn parse_maps_line(line: &str) -> Option<AddressSpaceMetadata> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }

    let addresses: Vec<&str> = parts[0].split('-').collect();
    let start = usize::from_str_radix(addresses[0], 16).ok()?;
    let end = usize::from_str_radix(addresses[1], 16).ok()?;

    Some(AddressSpaceMetadata {
        range: AddressRange { start, end },
        permissions: parts[1].to_string(),
        offset: usize::from_str_radix(parts[2], 16).unwrap_or(0),
        device: parts[3].to_string(),
        inode: parts[4].parse().unwrap_or(0),
        path: parts.get(5).cloned().unwrap_or("").to_string(),
        size: end - start,
    })
}
