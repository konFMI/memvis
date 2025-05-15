use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::memory::reader::{MemoryMap, AddressRange};

#[derive(Clone)]
pub struct AtomicMemoryReference {
    inner: Arc<Mutex<MemoryRefData>>,
}

struct MemoryRefData {
    maps: HashMap<AddressRange, MemoryMap>,
    ranges: Vec<AddressRange>,
}

impl AtomicMemoryReference {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MemoryRefData {
                maps: HashMap::new(),
                ranges: Vec::new(),
            })),
        }
    }

    pub fn get_range(&self, start: usize, end: usize) -> (usize, Option<MemoryMap>, Vec<u8>) {
        let data = self.inner.lock().unwrap();
        let mut result = Vec::new();
        let mut meta = None;
        
        for (i, range) in data.ranges.iter().enumerate() {
            if let Some(map) = data.maps.get(range) {
                if range.contains(start) {
                    meta = Some(map.clone()); // Capture the memory map (including path)
                    result.extend_from_slice(map.slice(start, end)); // Add memory content
                    return (i, meta, result); // Return the content and metadata
                }
            }
        }
        
        (0, meta, vec![0; end - start]) // Fallback if not found
    }
    

    pub fn set_maps(&self, memory_maps: Vec<MemoryMap>) {
        let mut data = self.inner.lock().unwrap();
        data.maps.clear();
        data.ranges.clear();
        for map in memory_maps {
            let range = map.metadata.range.clone();
            data.maps.insert(range.clone(), map);
            data.ranges.push(range);
        }
        data.ranges.sort(); // Ensure deterministic order
    }

    pub fn ranges(&self) -> Vec<AddressRange> {
        self.inner.lock().unwrap().ranges.clone()
    }
}
