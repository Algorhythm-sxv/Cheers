use std::{fmt::Display, ops::Index};

use crate::{board::Board, types::*};
use Piece::*;

use cheers_bitboards::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub piece: Piece,
    pub from: Square,
    pub to: Square,
    pub promotion: Piece,
}

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

        Self {
            piece,
            from,
            to,
            promotion,
        }
    }

    pub fn coords(&self) -> String {
        format!("{self}")
    }

    pub fn null() -> Self {
        Self {
            piece: Pawn,
            from: Square::A1,
            to: Square::A1,
            promotion: Pawn,
        }
    }

    pub fn is_null(&self) -> bool {
        self.from == self.to
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let promo = match self.promotion {
            Knight => "n",
            Bishop => "b",
            Rook => "r",
            Queen => "q",
            _ => "",
        };
        write!(f, "{}", self.from.coord() + &self.to.coord() + promo)
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
            Some(Move {
                piece: Pawn,
                from: self.moves.start,
                to: target,
                promotion,
            })
        } else {
            self.moves.moves ^= target.bitboard();
            Some(Move {
                piece: self.moves.piece,
                from: self.moves.start,
                to: target,
                promotion: Pawn,
            })
        }
    }
}

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
    pub fn pick_move(&mut self, current_index: usize) -> (Move, i32) {
        let mut best_index = current_index;

        for i in (current_index + 1)..self.len {
            if self.inner[i].score > self.inner[best_index].score {
                best_index = i;
            }
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

#[derive(Copy, Clone)]
pub struct KillerMoves<const N: usize>([[Move; N]; 128]);

impl<const N: usize> KillerMoves<N> {
    pub fn new() -> Self {
        Self([[Move::null(); N]; 128])
    }
    pub fn push(&mut self, m: Move, ply: usize) {
        let mut moves = self.0[ply];
        if !moves.contains(&m) {
            for i in (1..N).rev() {
                moves[i] = moves[i - 1];
            }
            moves[0] = m;
        }
    }
}

impl<const N: usize> Index<usize> for KillerMoves<N> {
    type Output = [Move; N];

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<const N: usize> Default for KillerMoves<N> {
    fn default() -> Self {
        Self::new()
    }
}
