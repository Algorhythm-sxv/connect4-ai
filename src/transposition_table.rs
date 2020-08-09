use std::sync::{atomic::*, Arc};

#[derive(Copy, Clone)]
struct Entry {
    key: u32,
    value: u8,
}
impl Entry {
    pub fn new() -> Self {
        Self { key: 0, value: 0 }
    }
}

const TABLE_MAX_SIZE: usize = (1 << 23) + 9; // prime value
#[derive(Clone)]
pub struct TranspositionTable {
    entries: Vec<Entry>,
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            entries: vec![Entry::new(); TABLE_MAX_SIZE],
        }
    }
    pub fn set(&mut self, key: u64, value: u8) {
        let mut entry = Entry::new();
        entry.key = key as u32;
        entry.value = value;

        let len = self.entries.len();
        self.entries[key as usize % len] = entry;
    }
    pub fn get(&self, key: u64) -> u8 {
        let entry = self.entries[key as usize % self.entries.len()];
        if entry.key == key as u32 {
            return entry.value;
        } else {
            return 0;
        }
    }
}

struct SharedEntry {
    key: AtomicU32,
    value: AtomicU8,
}
impl SharedEntry {
    pub fn new() -> Self {
        Self {
            key: AtomicU32::new(0),
            value: AtomicU8::new(0),
        }
    }
    pub fn store(&self, key: u32, value: u8) {
        self.key.store(key as u32, Ordering::Relaxed);
        self.value.store(value as u8, Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub struct SharedTranspositionTable {
    entries: Arc<Vec<SharedEntry>>,
}
impl SharedTranspositionTable {
    pub fn new() -> Self {
        let mut entries = Vec::with_capacity(TABLE_MAX_SIZE);
        for _ in 0..TABLE_MAX_SIZE {
            entries.push(SharedEntry::new());
        }
        Self {
            entries: Arc::new(entries),
        }
    }
    pub fn set(&mut self, key: u64, value: u8) {
        let i = key as usize % self.entries.len();
        self.entries[i].store(key as u32 ^ value as u32, value);
    }
    pub fn get(&self, key: u64) -> u8 {
        let entry = &self.entries[key as usize % self.entries.len()];
        let data = entry.value.load(Ordering::Relaxed);
        if entry.key.load(Ordering::Relaxed) == key as u32 ^ data as u32 {
            return data
        } else {
            return 0;
        }
    }
}
