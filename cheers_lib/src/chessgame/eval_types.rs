use std::ops::{Add, AddAssign, Index, Sub, IndexMut};

use bytemuck::{Pod, Zeroable};

use crate::{bitboard::BitBoard, types::PieceIndex};

use super::EvalTrace;

pub struct CoeffArray<T, const N: usize>(pub [T; N]);

impl<T, const N: usize, I: Into<usize>> Index<I> for CoeffArray<T, N> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        &self.0[index.into()]
    }
}

pub struct EvalInfo {
    pub mobility_area: [BitBoard; 2],
    pub behind_pawns: [BitBoard; 2],
    pub outposts: [BitBoard; 2],
    pub seventh_rank: [BitBoard; 2],
    pub king_square: [i32; 2],
    pub king_area: [BitBoard; 2],
}

#[derive(Copy, Clone)]
pub struct EvalScore {
    pub mg: i32,
    pub eg: i32,
}

impl EvalScore {
    pub fn zero() -> Self {
        Self { mg: 0, eg: 0 }
    }
}

impl Add<EvalScore> for EvalScore {
    type Output = Self;

    fn add(self, rhs: EvalScore) -> Self::Output {
        Self {
            mg: self.mg + rhs.mg,
            eg: self.eg + rhs.eg,
        }
    }
}

impl AddAssign<EvalScore> for EvalScore {
    fn add_assign(&mut self, rhs: EvalScore) {
        self.mg += rhs.mg;
        self.eg += rhs.eg;
    }
}

impl Sub<EvalScore> for EvalScore {
    type Output = Self;

    fn sub(self, rhs: EvalScore) -> Self::Output {
        Self {
            mg: self.mg - rhs.mg,
            eg: self.eg - rhs.eg,
        }
    }
}

pub trait TraceTarget {
    const TRACING: bool = false;
    fn term(&mut self, _term: impl FnMut(&mut EvalTrace)) {}
}

impl TraceTarget for EvalTrace {
    const TRACING: bool = true;
    fn term(&mut self, mut term: impl FnMut(&mut EvalTrace)) {
        term(self)
    }
}
impl TraceTarget for () {}

#[derive(Clone, Copy, Debug)]
pub enum GamePhase {
    Midgame = 0,
    Endgame = 1,
}

impl<T, const N: usize> Index<GamePhase> for [T; N] {
    type Output = T;

    fn index(&self, index: GamePhase) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T, const N: usize> IndexMut<GamePhase> for [T;N] {
    fn index_mut(&mut self, index: GamePhase) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct PieceTables(pub [[[i32; 2]; 64]; 6]);
impl std::ops::Index<(GamePhase, PieceIndex, usize)> for PieceTables {
    type Output = i32;
    fn index(&self, index: (GamePhase, PieceIndex, usize)) -> &Self::Output {
        &self.0[index.1 as usize][index.2][index.0 as usize]
    }
}

impl Default for PieceTables {
    fn default() -> Self {
        PieceTables([[[0; 2]; 64]; 6])
    }
}

#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)]
pub struct PieceValues(pub [[i32; 2]; 6]);

impl std::ops::Index<(GamePhase, PieceIndex)> for PieceValues {
    type Output = i32;
    fn index(&self, index: (GamePhase, PieceIndex)) -> &Self::Output {
        &self.0[index.1 as usize][index.0 as usize]
    }
}
