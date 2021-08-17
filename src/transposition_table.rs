use std::sync::{atomic::*, Arc};

use crate::{bitboard::BitBoards, types::*};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeType {
    Exact = 0,
    UpperBound = 1,
    LowerBound = 2,
}

impl NodeType {
    pub fn from_u8(from: u8) -> Self {
        match from {
            0 => Self::Exact,
            1 => Self::UpperBound,
            2 => Self::LowerBound,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Default)]
struct Entry {
    key: AtomicU64,
    data: AtomicU64,
}

fn extract_data(data: u64) -> ((u8, u8), u8, PieceIndex, i32, NodeType) {
    let node_type = NodeType::from_u8((data & 0x1F) as u8);
    let promotion = PieceIndex::from_u8(((data >> 5) & 0x07) as u8);
    let move_target = ((data >> 8) & 0x3F) as u8;
    let move_start = ((data >> 16) & 0xFF) as u8;
    let depth = ((data >> 24) & 0xFF) as u8;
    let score = (data >> 32) as i32;

    ((move_start, move_target), depth, promotion, score, node_type)
}

#[derive(Clone, Default, Debug)]
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
    pub fn set(&self, boards: &BitBoards, best_move: Move, depth: u8, score: i32, node_type: NodeType) {
        use Ordering::*;

        let index = boards.position_hash as usize % self.table.len();
        let stored_depth = (self.table[index].data.load(Acquire) >> 24) & 0xFF;
        if stored_depth > depth as u64 {
            // depth-preferred replacement
            return;
        }

        let mut data: u64 = 0;
        data |= (score as u32 as u64) << 32;
        data |= (depth as u64) << 24;
        data |= (best_move.start as u64) << 16;
        data |= (best_move.target as u64) << 8;
        data |= (best_move.promotion as u64) << 5;
        data |= node_type as u64;

        self.table[index].key.store(boards.position_hash ^ data, Release);
        self.table[index].data.store(data, Release);
    }

    pub fn get(&self, boards: &BitBoards) -> Option<((u8, u8), u8, PieceIndex, i32, NodeType)> {
        use Ordering::*;
        let index = boards.position_hash as usize % self.table.len();
        let data = self.table[index].data.load(Acquire);
        if self.table[index].key.load(Acquire) ^ data == boards.position_hash {
            // entry is valid, return data
            Some(extract_data(data))
        } else {
            // key and data didn't match, invalid entry
            None
        }
    }
}
