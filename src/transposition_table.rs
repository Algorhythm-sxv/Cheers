use std::sync::{atomic::*, Arc};

use crate::{moves::Move, types::PieceIndex};

pub const TT_DEFAULT_SIZE: usize = 1 << 22; // 2^22 entries for ~64MB

pub enum NodeType {
    Exact,
    UpperBound,
    LowerBound,
}

impl NodeType {
    pub fn from_u8(n: u8) -> Self {
        match n {
            0 => NodeType::Exact,
            1 => NodeType::UpperBound,
            2 => NodeType::LowerBound,
            _ => unreachable!(),
        }
    }
}

pub struct TTEntry {
    pub move_start: u8,
    pub move_target: u8,
    pub promotion: PieceIndex,
    pub depth: u8,
    pub score: i32,
    pub node_type: NodeType,
}

impl TTEntry {
    pub fn from_data(data: u64) -> Self {
        Self {
            node_type: NodeType::from_u8((data & 0x1F) as u8),
            promotion: PieceIndex::from_u8(((data >> 5) & 0x07) as u8),
            move_target: ((data >> 8) & 0x3F) as u8,
            move_start: ((data >> 16) & 0xFF) as u8,
            depth: ((data >> 24) & 0xFF) as u8,
            score: (data >> 32) as i32,
        }
    }
}

#[derive(Default)]
struct Entry {
    key: AtomicU64,
    data: AtomicU64,
}

#[derive(Clone)]
pub struct TranspositionTable {
    table: Arc<Vec<Entry>>,
}

impl TranspositionTable {
    pub fn new(table_size: usize) -> Self {
        let mut table = Vec::with_capacity(table_size);
        for _ in 0..table_size {
            table.push(Entry::default());
        }
        Self {
            table: Arc::new(table),
        }
    }

    pub fn set(&self, hash: u64, best_move: Move, depth: u8, score: i32, node_type: NodeType) {
        use Ordering::*;

        let index = hash as usize % self.table.len();
        let stored_depth = (self.table[index].data.load(Acquire) >> 24) & 0xFF;
        if stored_depth > depth as u64 {
            // depth-preferred replacement
            return;
        }

        let mut data = 0u64;
        data |= (score as u32 as u64) << 32;
        data |= (depth as u64) << 24;
        data |= (best_move.start() as u64) << 16;
        data |= (best_move.target() as u64) << 8;
        data |= (best_move.promotion() as u64) << 5;
        data |= node_type as u64;

        self.table[index].key.store(hash ^ data, Release);
        self.table[index].data.store(data, Release);
    }

    pub fn get(&self, hash: u64) -> Option<TTEntry> {
        use Ordering::*;

        let index = hash as usize % self.table.len();
        let data = self.table[index].data.load(Acquire);

        if self.table[index].key.load(Acquire) ^ data == hash {
            // entry is valid, return data
            Some(TTEntry::from_data(data))
        } else {
            // key and data didn't match, invalid entry
            None
        }
    }
}
