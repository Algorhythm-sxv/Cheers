use std::sync::atomic::*;

use cheers_bitboards::{BitBoard, Square};

use crate::{
    board::{eval_types::EvalScore, evaluate::CHECKMATE_SCORE},
    moves::Move,
    search::{MINUS_INF, SEARCH_MAX_PLY},
    types::{Piece, TypeColor},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Exact = 0,
    UpperBound = 1,
    LowerBound = 2,
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
    pub score: i16,
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
            score: ((data >> 16) & 0xFFFF) as i16,
            depth: ((data >> (16 + 16)) & 0xFF) as i8,
            move_from: ((data >> (16 + 16 + 8)) & 0xFF).into(),
            move_to: ((data >> (16 + 16 + 8 + 8)) & 0xFF).into(),
            piece: Piece::from_u8(((data >> (16 + 16 + 8 + 8 + 8)) & 0b111) as u8),
            promotion: Piece::from_u8(((data >> (16 + 16 + 8 + 8 + 8 + 3)) & 0b111) as u8),
            node_type: NodeType::from_u8(((data >> (16 + 16 + 8 + 8 + 8 + 3 + 3)) & 0b11) as u8),
        }
    }
}

impl From<u64> for TTEntry {
    fn from(data: u64) -> Self {
        Self {
            score: ((data >> 16) & 0xFFFF) as i16,
            depth: ((data >> (16 + 16)) & 0xFF) as i8,
            move_from: ((data >> (16 + 16 + 8)) & 0xFF).into(),
            move_to: ((data >> (16 + 16 + 8 + 8)) & 0xFF).into(),
            piece: Piece::from_u8(((data >> (16 + 16 + 8 + 8 + 8)) & 0b111) as u8),
            promotion: Piece::from_u8(((data >> (16 + 16 + 8 + 8 + 8 + 3)) & 0b111) as u8),
            node_type: NodeType::from_u8(((data >> (16 + 16 + 8 + 8 + 8 + 3 + 3)) & 0b11) as u8),
        }
    }
}

fn data_into_u64(key: u16, score: i16, depth: i8, best_move: Move, node_type: NodeType) -> u64 {
    let mut data = 0u64;
    data |= key as u64; // bits 0:15
    data |= ((score as u16) as u64) << 16; // bits 16:31
    data |= ((depth as u8) as u64) << (16 + 16); // bits 32:39
    data |= (*best_move.from as u64) << (16 + 16 + 8); // bits 40:47
    data |= (*best_move.to as u64) << (16 + 16 + 8 + 8); // bits 48:55
    data |= (best_move.piece as u64) << (16 + 16 + 8 + 8 + 8); // bits 56:58
    data |= (best_move.promotion as u64) << (16 + 16 + 8 + 8 + 8 + 3); // bits 59:62
    data |= (node_type as u64) << (16 + 16 + 8 + 8 + 8 + 3 + 3); // bits 62:63
    data
}

// key: 16 bits
// score: 16 bits
// depth: 8 bits
// from: 8 bits
// to: 8 bits
// piece: 3 bits
// promotion: 3 bits
// node type: 2 bits
struct Entry {
    data: AtomicU64,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            data: AtomicU64::new(data_into_u64(
                0xFFFF,
                MINUS_INF,
                -100,
                Move::null(),
                NodeType::LowerBound,
            )),
        }
    }
}

pub struct TranspositionTable {
    table: Vec<Entry>,
}

impl TranspositionTable {
    pub fn new(table_size_mb: usize) -> Self {
        let length = table_size_mb * 1024 * 1024 / std::mem::size_of::<Entry>();
        let mut table = Vec::with_capacity(length);
        for _ in 0..length {
            table.push(Entry::default());
        }
        Self { table }
    }

    pub fn set_size(&mut self, size_mb: usize) {
        let length = size_mb * 1024 * 1024 / std::mem::size_of::<Entry>();
        self.table.resize_with(length, Entry::default);
    }

    pub fn prefetch(&self, hash: u64) {
        let index = self.wrap_hash(hash);
        let entry = &self.table[index];
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
            _mm_prefetch((entry as *const Entry).cast::<i8>(), _MM_HINT_T0);
        }
    }

    pub fn set(
        &self,
        hash: u64,
        best_move: Move,
        depth: i8,
        score: i16,
        node_type: NodeType,
        pv: bool,
    ) {
        use self::Ordering::*;
        let index = self.wrap_hash(hash);

        let stored = match self.table.get(index) {
            Some(entry) => entry,
            None => return,
        };

        let incoming_tt_key = hash_to_tt_key(hash);
        let data = stored.data.load(Relaxed);

        const DEPTH_OFFSET: i8 = 8;
        if node_type == NodeType::Exact
            || data as u16 != incoming_tt_key
            || depth - DEPTH_OFFSET + 2 * (pv as i8) > ((data >> 32) & 0xFF) as i8
        {
            let data = data_into_u64(incoming_tt_key, score, depth, best_move, node_type);

            stored.data.store(data, Release);
        }
    }

    pub fn get(&self, hash: u64) -> Option<TTEntry> {
        use self::Ordering::*;
        let index = self.wrap_hash(hash);

        let stored = self.table.get(index)?;

        let data = stored.data.load(Acquire);

        let key = hash_to_tt_key(hash);

        if (data & 0xFFFF) as u16 == key {
            // entry is valid, return data
            Some(TTEntry::from(data))
        } else {
            // key and data didn't match, invalid entry
            None
        }
    }

    fn wrap_hash(&self, hash: u64) -> usize {
        let key = u128::from(hash);
        let len = self.table.len() as u128;
        ((key * len) >> 64) as usize
    }

    pub fn sample_fill(&self) -> usize {
        let default = Entry::default().data.load(Ordering::Relaxed);
        self.table[..1000]
            .iter()
            .filter(|e| e.data.load(Ordering::Relaxed) != default)
            .count()
    }
}

pub fn score_from_tt(score: i16, ply: usize) -> i16 {
    // mate scores out of the TT in plies from the current position
    if CHECKMATE_SCORE.abs_diff(score) <= SEARCH_MAX_PLY as u16 {
        // move the mate further away to the plies from the root
        score - ply as i16
    } else if (-CHECKMATE_SCORE).abs_diff(score) <= SEARCH_MAX_PLY as u16 {
        // move the mate further away to the plies from the root
        score + ply as i16
    } else {
        score
    }
}

pub fn score_into_tt(score: i16, ply: usize) -> i16 {
    // mate scores into the TT are in plies from the root
    if CHECKMATE_SCORE.abs_diff(score) <= SEARCH_MAX_PLY as u16 {
        // move the mate closer to the plies from the current position
        score + ply as i16
    } else if (-CHECKMATE_SCORE).abs_diff(score) <= SEARCH_MAX_PLY as u16 {
        // move the mate closer to the plies from the current position
        score - ply as i16
    } else {
        score
    }
}

fn hash_to_tt_key(hash: u64) -> u16 {
    hash as u16
}

#[derive(Copy, Clone, Debug)]
pub struct PawnHashEntry {
    pub hash: u64,
    pub mg: i16,
    pub eg: i16,
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

    pub fn set<T: TypeColor>(&mut self, hash: u64, mg: i16, eg: i16, passed_pawns: BitBoard) {
        let (mg, eg) = if T::WHITE { (mg, eg) } else { (-mg, -eg) };
        self.table[(hash & self.mask) as usize] = PawnHashEntry {
            hash,
            mg,
            eg,
            passed_pawns,
        };
    }
}

#[cfg(test)]
mod tests {
    use cheers_bitboards::Square;

    use crate::{board::Board, moves::Move, types::Piece};

    use super::{NodeType, TranspositionTable};

    #[test]
    fn test_tt() -> Result<(), &'static str> {
        let board =
            Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
                .unwrap();
        let score = 30000;
        let best_move = Move {
            piece: Piece::King,
            from: Square::E1,
            to: Square::A1,
            promotion: Piece::Pawn,
        };
        let node_type = NodeType::Exact;
        let depth = 7;

        let tt = TranspositionTable::new(1);

        tt.set(board.hash(), best_move, depth, score, node_type, true);

        let entry = tt.get(board.hash()).ok_or("TT entry not found!")?;
        assert!(entry.score == score, "Scores not equal!");
        assert!(entry.move_from == best_move.from, "Move from not equal!");
        assert!(entry.move_to == best_move.to, "Move to not equal!");
        assert!(entry.depth == depth, "depth not equal!");
        assert!(entry.node_type == node_type, "Node type not equal!");
        assert!(entry.piece == best_move.piece, "Piece not equal!");
        assert!(
            entry.promotion == best_move.promotion,
            "Promotion not equal!"
        );

        Ok(())
    }
}
