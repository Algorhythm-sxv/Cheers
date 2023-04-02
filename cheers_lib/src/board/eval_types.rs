use std::{
    fmt::{Debug, Display},
    ops::{Add, AddAssign, Index, IndexMut, Mul, Neg, Sub, SubAssign},
};

#[cfg(feature = "eval-tracing")]
use bytemuck::{Pod, Zeroable};

use crate::types::Piece;
use cheers_bitboards::{BitBoard, Square};

use super::eval_params::EvalTrace;

pub struct EvalInfo {
    pub mobility_area: [BitBoard; 2],
    pub behind_pawns: [BitBoard; 2],
    pub outposts: [BitBoard; 2],
    pub seventh_rank: [BitBoard; 2],
    pub king_square: [Square; 2],
    pub king_area: [BitBoard; 2],
    pub passed_pawns: [BitBoard; 2],
}

#[cfg_attr(feature = "eval-tracing", derive(Pod, Zeroable))]
#[derive(Copy, Clone, Eq, PartialEq, Default)]
#[repr(C)]
pub struct EvalScore(i32);

impl EvalScore {
    pub const fn new(mg: i16, eg: i16) -> Self {
        Self(((eg as i32) << 16).wrapping_add(mg as i32))
    }
    pub const fn zero() -> Self {
        Self(0)
    }
    pub const fn mg(&self) -> i16 {
        self.0 as i16
    }
    pub const fn eg(&self) -> i16 {
        ((self.0 + 0x8000) >> 16) as i16
    }
    pub fn div_by(&mut self, n: i16) {
        *self = Self::new(self.mg() / n, self.eg() / n)
    }
}

#[macro_export]
macro_rules! s {
    ($mg:literal, $eg:literal) => {
        EvalScore::new($mg, $eg)
    };
}

pub use s;

impl Add<Self> for EvalScore {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign<Self> for EvalScore {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub<Self> for EvalScore {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign<Self> for EvalScore {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Neg for EvalScore {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.mg(), -self.eg())
    }
}

impl Mul<i16> for EvalScore {
    type Output = Self;

    fn mul(self, rhs: i16) -> Self::Output {
        Self(self.0 * (rhs as i32))
    }
}

impl Display for EvalScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "s!({}, {})", self.mg(), self.eg())
    }
}

impl Debug for EvalScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
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

impl<T, const N: usize> IndexMut<GamePhase> for [T; N] {
    fn index_mut(&mut self, index: GamePhase) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

#[cfg_attr(feature = "eval-tracing", derive(Pod, Zeroable))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct PieceTables(pub [[EvalScore; 64]; 6]);
impl std::ops::Index<(Piece, Square)> for PieceTables {
    type Output = EvalScore;
    fn index(&self, index: (Piece, Square)) -> &Self::Output {
        &self.0[index.0 as usize][index.1]
    }
}

impl Default for PieceTables {
    fn default() -> Self {
        PieceTables([[EvalScore::default(); 64]; 6])
    }
}

#[cfg_attr(feature = "eval-tracing", derive(Pod, Zeroable))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct PieceValues(pub [EvalScore; 6]);

impl std::ops::Index<Piece> for PieceValues {
    type Output = EvalScore;
    fn index(&self, index: Piece) -> &Self::Output {
        &self.0[index as usize]
    }
}
