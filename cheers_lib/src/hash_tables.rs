use std::sync::atomic::*;

use cheers_bitboards::{BitBoard, Square};

use crate::{
    board::{eval_types::EvalScore, evaluate::CHECKMATE_SCORE},
    moves::Move,
    search::SEARCH_MAX_PLY,
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
            depth: ((data >> 16 + 16) & 0xFF) as i8,
            move_from: ((data >> (16 + 16 + 8)) & 0xFF).into(),
            move_to: ((data >> (16 + 16 + 8 + 8)) & 0xFF).into(),
            piece: Piece::from_u8(((data >> (16 + 16 + 8 + 8 + 8)) & 0b111) as u8),
            promotion: Piece::from_u8(((data >> (16 + 16 + 8 + 8 + 8 + 3)) & 0b111) as u8),
            node_type: NodeType::from_u8(((data >> (16 + 16 + 8 + 8 + 8 + 3 + 3)) & 0b11) as u8),
        }
    }
}

// key: 16 bits
// score: 16 bits
// depth: 8 bits
// from: 8 bits
// to: 8 bits
// piece: 3 bits
// promotion: 3 bits
// node type: 2 bits
#[derive(Default)]
struct Entry {
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
        score: i16,
        node_type: NodeType,
        pv: bool,
    ) {
        use self::Ordering::*;
        let index = hash as usize & (self.table.len() - 1);

        let stored = match self.table.get(index) {
            Some(entry) => entry,
            None => return,
        };

        let incoming_tt_key = hash_to_tt_key(hash);
        let data = stored.data.load(Relaxed);

        const DEPTH_OFFSET: i8 = 8;
        if node_type == NodeType::Exact
            || data as u16 != incoming_tt_key
            || depth - DEPTH_OFFSET + 2 * (pv as i8)
                > ((data >> 32) & 0xFF) as i8
        {
            let mut data = 0u64;
            data |= (incoming_tt_key & 0xFFFF) as u64;
            data |= (score as u16 as u64) << 16;
            data |= (depth as u8 as u64) << (16 + 16);
            data |= (*best_move.from as u64) << (16 + 16 + 8);
            data |= (*best_move.to as u64) << (16 + 16 + 8 + 8);
            data |= (best_move.piece as u64) << (16 + 16 + 8 + 8 + 8);
            data |= (best_move.promotion as u64) << (16 + 16 + 8 + 8 + 8 + 3);
            data |= (node_type as u64) << (16 + 16 + 8 + 8 + 8 + 3 + 3);

            stored.data.store(data, Release);
        }
    }

    pub fn get(&self, hash: u64) -> Option<TTEntry> {
        use self::Ordering::*;
        let index = hash as usize & (self.table.len() - 1);

        let stored = self.table.get(index)?;

        let data = stored.data.load(Acquire);

        if data as u16 == hash_to_tt_key(hash) {
            // entry is valid, return data
            if (data >> (16 + 16 + 8 + 8 + 8)) & 0b111 > 5 {
                println!("broken TT entry");
                println!("{data:064b}");
            }
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
    (hash as u16) ^ ((hash >> 16) as u16) ^ ((hash >> 32) as u16) ^ ((hash >> 48) as u16)
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
