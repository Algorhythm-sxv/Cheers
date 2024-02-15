use std::{fmt::Display, ops::Index};

use crate::{board::Board, types::*};
use Piece::*;

use cheers_bitboards::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move(u32);

impl Move {
    pub fn from_pair<T: AsRef<str>>(board: &Board, pair: T) -> Self {
        let pair = pair.as_ref();
        let from = Square::from_coord(&pair[0..2]);
        let mut to = Square::from_coord(&pair[2..4]);
        let promotion = match pair.chars().nth(4) {
            Some('n') => Knight,
            Some('b') => Bishop,
            Some('r') => Rook,
            Some('q') => Queen,
            _ => Pawn,
        };

        let piece = board.piece_on(from).unwrap_or(Pawn);
        if piece == King {
            // correct for castling
            if to.file() > from.file() && to.file().abs_diff(from.file()) > 1 {
                // kingside castling
                to = board.castling_rights()[board.current_player()][0].first_square()
            } else if to.file() < from.file() && to.file().abs_diff(from.file()) > 1 {
                // queenside castling
                to = board.castling_rights()[board.current_player()][1].first_square()
            }
        }

        Self::new(piece, from, to, promotion)
    }

    pub fn coords_960(&self) -> String {
        format!("{self}")
    }

    pub fn coords(&self) -> String {
        let coords = format!("{self}");
        match (self.piece(), coords.as_str()) {
            (King, "e1h1") => String::from("e1g1"),
            (King, "e1a1") => String::from("e1c1"),
            (King, "e8h8") => String::from("e8g8"),
            (King, "e8a8") => String::from("e8c8"),
            _ => coords,
        }
    }

    pub fn null() -> Self {
        Self(0)
    }

    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
    pub fn new(piece: Piece, from: Square, to: Square, promotion: Piece) -> Self {
        Self(
            piece as u32 | ((*from as u32) << 3) | ((*to as u32) << 9) | ((promotion as u32) << 15),
        )
    }
    pub fn piece(&self) -> Piece {
        Piece::from_u8(self.0 as u8 & 0b111)
    }
    pub fn from(&self) -> Square {
        Square::from((self.0 >> 3) as u8 & 0b111111)
    }
    pub fn to(&self) -> Square {
        Square::from((self.0 >> 9) as u8 & 0b111111)
    }
    pub fn promotion(&self) -> Piece {
        Piece::from_u8((self.0 >> 15) as u8 & 0b111)
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let promo = match self.promotion() {
            Knight => "n",
            Bishop => "b",
            Rook => "r",
            Queen => "q",
            _ => "",
        };
        write!(f, "{}", self.from().coord() + &self.to().coord() + promo)
    }
}

impl Default for Move {
    fn default() -> Self {
        Self::null()
    }
}

pub struct MoveMask {
    pub piece: Piece,
    pub start: Square,
    pub moves: BitBoard,
}

impl MoveMask {
    pub fn len(&self) -> usize {
        let len = if self.piece == Pawn {
            self.moves.count_ones() + (self.moves & (EIGHTH_RANK | FIRST_RANK)).count_ones() * 3
        } else {
            self.moves.count_ones()
        };
        len as usize
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
impl IntoIterator for MoveMask {
    type Item = Move;

    type IntoIter = MoveMaskIter;

    fn into_iter(self) -> Self::IntoIter {
        MoveMaskIter {
            moves: self,
            promotion_counter: 0,
        }
    }
}

pub struct MoveMaskIter {
    moves: MoveMask,
    promotion_counter: u8,
}

impl Iterator for MoveMaskIter {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        if self.moves.moves.is_empty() {
            return None;
        }
        let target = self.moves.moves.first_square();
        if self.moves.piece == Pawn && matches!(target.rank(), 0 | 7) {
            let promotion = match self.promotion_counter {
                0 => Queen,
                1 => Rook,
                2 => Knight,
                3 => Bishop,
                _ => unreachable!(),
            };
            if self.promotion_counter < 3 {
                self.promotion_counter += 1;
            } else {
                self.promotion_counter = 0;
                self.moves.moves ^= target.bitboard();
            }
            Some(Move::new(Pawn, self.moves.start, target, promotion))
        } else {
            self.moves.moves ^= target.bitboard();
            Some(Move::new(self.moves.piece, self.moves.start, target, Pawn))
        }
    }
}

// constants to score moves by type without overlap
pub const TT_MOVE_SCORE: i32 = 400_000;
pub const WINNING_CAPTURE_SCORE: i32 = 300_000;
pub const KILLER_MOVE_SCORE: i32 = 200_000;
pub const COUNTERMOVE_SCORE: i32 = 100_000;
pub const QUIET_SCORE: i32 = 0;
pub const LOSING_CAPTURE_SCORE: i32 = -100_000;
pub const UNDERPROMO_SCORE: i32 = -200_000;

#[derive(Copy, Clone, Debug)]
pub struct SortingMove {
    pub mv: Move,
    pub score: i32,
}

impl SortingMove {
    pub fn new(mv: Move) -> Self {
        Self { mv, score: 0 }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MoveList {
    len: usize,
    inner: [SortingMove; 218],
}

impl MoveList {
    pub fn new() -> Self {
        Self {
            len: 0,
            inner: [SortingMove::new(Move::null()); 218],
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn inner(&self) -> &[SortingMove] {
        &self.inner[..self.len]
    }

    pub fn inner_mut(&mut self) -> &mut [SortingMove] {
        &mut self.inner[..self.len]
    }

    pub fn push(&mut self, mv: SortingMove) {
        self.inner[self.len] = mv;
        self.len += 1;
    }

    pub fn reset(&mut self) {
        self.len = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn score(&mut self, idx: usize) -> &mut i32 {
        &mut self.inner[idx].score
    }

    #[inline(always)]
    pub fn pick_move(&mut self, current_index: usize) -> (Move, i32) {
        let moves = &self.inner[(current_index + 1)..self.len];
        let mut best_index = current_index;
        let mut best_score = self.inner[current_index].score;

        for (i, mv) in moves.iter().enumerate() {
            let new = mv.score;
            let replace = new > best_score;
            best_index =
                (best_index * !replace as usize) | ((i + current_index + 1) * replace as usize);
            best_score = (best_score * !replace as i32) | (new * replace as i32);
        }

        self.inner.swap(current_index, best_index);

        (
            self.inner[current_index].mv,
            self.inner[current_index].score,
        )
    }
}

impl Index<usize> for MoveList {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index].mv
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self {
            len: 0,
            inner: [SortingMove::new(Move::default()); 218],
        }
    }
}

pub const NUM_KILLER_MOVES: usize = 2;
#[derive(Copy, Clone)]
pub struct KillerMoves<const N: usize>([Move; N]);

impl<const N: usize> KillerMoves<N> {
    pub fn new() -> Self {
        Self([Move::null(); N])
    }
    pub fn push(&mut self, m: Move) {
        let moves = &mut self.0;
        if !moves.contains(&m) {
            for i in (1..N).rev() {
                moves[i] = moves[i - 1];
            }
            moves[0] = m;
        }
    }
    pub fn contains(&self, mv: &Move) -> bool {
        self.0.contains(mv)
    }
}

impl<const N: usize> Default for KillerMoves<N> {
    fn default() -> Self {
        Self::new()
    }
}

pub const PV_MAX_LEN: usize = 16;
#[derive(Copy, Clone, Default, Debug)]
pub struct PrincipalVariation {
    len: usize,
    moves: [Move; PV_MAX_LEN],
    chess_960: bool,
}

impl PrincipalVariation {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn update_from(&mut self, next: Move, other: &Self) {
        self.moves[0] = next;
        self.moves[1..(other.len + 1).min(PV_MAX_LEN)]
            .copy_from_slice(&other.moves[..(other.len.min(PV_MAX_LEN - 1))]);
        self.len = (other.len + 1).min(PV_MAX_LEN);
    }
    pub fn clear(&mut self) {
        self.len = 0;
    }
    pub fn iter(&self) -> std::slice::Iter<'_, Move> {
        self.moves[..self.len].iter()
    }
    pub fn chess_960(mut self, chess_960: bool) -> Self {
        self.chess_960 = chess_960;
        self
    }
}
impl Display for PrincipalVariation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, m) in self.moves.iter().take(self.len).enumerate() {
            let coords = if self.chess_960 {
                m.coords_960()
            } else {
                m.coords()
            };
            if i == 0 {
                write!(f, "{}", coords)?;
            } else {
                write!(f, " {}", coords)?;
            }
        }
        Ok(())
    }
}
impl Index<usize> for PrincipalVariation {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.moves[index]
    }
}
