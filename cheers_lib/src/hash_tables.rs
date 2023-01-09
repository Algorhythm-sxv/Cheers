use std::sync::atomic::*;

use cheers_bitboards::{BitBoard, Square};

use crate::{
    board::eval_types::EvalScore,
    moves::Move,
    types::{Piece, TypeColor},
};

#[derive(Clone, Copy, PartialEq, Eq)]
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
    pub score: i32,
    pub depth: i8,
    pub piece: Piece,
    pub move_from: Square,
    pub move_to: Square,
    pub promotion: Piece,
    pub node_type: NodeType,
}

impl TTEntry {
    pub fn from_data(data: u64) -> Self {
        Self {
            score: (data & 0xFFFFFFFF) as i32,
            depth: ((data >> 32) & 0xFF) as i8,
            piece: Piece::from_u8(((data >> (32 + 8)) & 0b111) as u8),
            move_from: ((data >> (32 + 8 + 3)) & 0xFF).into(),
            move_to: ((data >> (32 + 8 + 3 + 8)) & 0xFF).into(),
            promotion: Piece::from_u8(((data >> (32 + 8 + 3 + 8 + 8)) & 0b111) as u8),
            node_type: NodeType::from_u8(((data >> (32 + 8 + 3 + 8 + 8 + 3)) & 0b11) as u8),
        }
    }
}

#[derive(Default)]
struct Entry {
    key: AtomicU64,
    data: AtomicU64,
}

pub struct TranspositionTable {
    table: Vec<Entry>,
}

impl TranspositionTable {
    pub fn new(table_size_mb: usize) -> Self {
        let mut length = table_size_mb * 1024 * 1024 / std::mem::size_of::<Entry>();
        if length != 0 {
            length = length.next_power_of_two();
        }
        let mut table = Vec::with_capacity(length);
        for _ in 0..length {
            table.push(Entry::default());
        }
        Self { table }
    }

    pub fn set_size(&mut self, size_mb: usize) {
        let mut length = size_mb * 1024 * 1024 / std::mem::size_of::<Entry>();
        length = length.next_power_of_two();
        self.table.resize_with(length, Entry::default);
    }

    pub fn set(
        &self,
        hash: u64,
        best_move: Move,
        depth: i8,
        score: i32,
        node_type: NodeType,
        pv: bool,
    ) {
        use self::Ordering::*;
        let index = hash as usize & (self.table.len() - 1);

        let stored = match self.table.get(index) {
            Some(entry) => entry,
            None => return,
        };

        const DEPTH_OFFSET: i8 = 8;
        if node_type == NodeType::Exact
            || stored.key.load(Relaxed) ^ stored.data.load(Relaxed) != hash
            || depth - DEPTH_OFFSET + 2 * (pv as i8)
                > ((stored.data.load(Relaxed) >> 32) & 0xFF) as i8
        {
            let mut data = 0u64;
            data |= score as u32 as u64;
            data |= ((depth as u8) as u64) << 32;
            data |= (best_move.piece as u64) << (32 + 8);
            data |= (*best_move.from as u64) << (32 + 8 + 3);
            data |= (*best_move.to as u64) << (32 + 8 + 3 + 8);
            data |= (best_move.promotion as u64) << (32 + 8 + 3 + 8 + 8);
            data |= (node_type as u64) << (32 + 8 + 3 + 8 + 8 + 3);

            stored.key.store(hash ^ data, Release);
            stored.data.store(data, Release);
        }
    }

    pub fn get(&self, hash: u64) -> Option<TTEntry> {
        use self::Ordering::*;
        let index = hash as usize & (self.table.len() - 1);

        let stored = self.table.get(index)?;

        let data = stored.data.load(Acquire);

        if stored.key.load(Acquire) ^ data == hash {
            // entry is valid, return data
            Some(TTEntry::from_data(data))
        } else {
            // key and data didn't match, invalid entry
            None
        }
    }

    pub fn sample_fill(&self) -> usize {
        self.table[..1000]
            .iter()
            .filter(|e| e.data.load(Ordering::Relaxed) != 0)
            .count()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PawnHashEntry {
    pub hash: u64,
    pub mg: i32,
    pub eg: i32,
    passed_pawns: BitBoard,
}
impl Default for PawnHashEntry {
    fn default() -> Self {
        Self {
            hash: 1,
            mg: 0,
            eg: 0,
            passed_pawns: BitBoard::empty(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PawnHashTable {
    table: Vec<PawnHashEntry>,
    mask: u64,
}

impl PawnHashTable {
    pub fn new(table_size_mb: usize) -> Self {
        let mut length = table_size_mb * 1024 * 1024 / std::mem::size_of::<PawnHashEntry>();
        if length != 0 {
            length = length.next_power_of_two();
        }
        Self {
            table: vec![PawnHashEntry::default(); length],
            mask: (length as u64).saturating_sub(1),
        }
    }

    pub fn get<T: TypeColor>(&self, hash: u64) -> Option<(EvalScore, BitBoard)> {
        let entry = self.table[(hash & self.mask) as usize];
        if entry.hash == hash {
            let sign = if !T::WHITE { -1 } else { 1 };
            Some((
                EvalScore {
                    mg: sign * entry.mg,
                    eg: sign * entry.eg,
                },
                entry.passed_pawns,
            ))
        } else {
            None
        }
    }

    pub fn set<T: TypeColor>(&mut self, hash: u64, mg: i32, eg: i32, passed_pawns: BitBoard) {
        let (mg, eg) = if T::WHITE { (mg, eg) } else { (-mg, -eg) };
        self.table[(hash & self.mask) as usize] = PawnHashEntry {
            hash,
            mg,
            eg,
            passed_pawns,
        };
    }
}
