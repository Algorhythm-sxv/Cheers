use std::ops::{Index, IndexMut};
pub struct White;
pub struct Black;

pub trait TypeColor {
    const WHITE: bool;
    const INDEX: usize;
    type Other: TypeColor;
}

impl TypeColor for White {
    const WHITE: bool = true;
    const INDEX: usize = 0;
    type Other = Black;
}

impl TypeColor for Black {
    const WHITE: bool = false;
    const INDEX: usize = 1;
    type Other = White;
}

pub struct InCheck;
pub struct NotInCheck;

pub trait TypeCheck {
    const IN_CHECK: bool;
}

impl TypeCheck for InCheck {
    const IN_CHECK: bool = true;
}

impl TypeCheck for NotInCheck {
    const IN_CHECK: bool = false;
}

pub struct Ep;
pub struct NoEp;

pub trait EpPossible {
    const EP_POSSIBLE: bool;
}

impl EpPossible for Ep {
    const EP_POSSIBLE: bool = true;
}

impl EpPossible for NoEp {
    const EP_POSSIBLE: bool = false;
}

pub struct Castling;
pub struct NoCastling;

pub trait CastlingPossible {
    const CASTLING_POSSIBLE: bool;
}

impl CastlingPossible for Castling {
    const CASTLING_POSSIBLE: bool = true;
}

impl CastlingPossible for NoCastling {
    const CASTLING_POSSIBLE: bool = false;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl Piece {
    pub fn from_u8(n: u8) -> Self {
        match n {
            0 => Pawn,
            1 => Knight,
            2 => Bishop,
            3 => Rook,
            4 => Queen,
            5 => King,
            _ => unreachable!()
        }
    }
}

impl<T, const N: usize> Index<Piece> for [T; N] {
    type Output = T;

    fn index(&self, index: Piece) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T, const N: usize> IndexMut<Piece> for [T; N] {
    fn index_mut(&mut self, index: Piece) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

use Piece::*;
pub const PIECES: [Piece; 6] = [Pawn, Knight, Bishop, Rook, Queen, King];
pub enum CastlingIndex {
    Queenside = 0,
    Kingside = 1,
}

impl<T, const N: usize> Index<CastlingIndex> for [T; N] {
    type Output = T;

    fn index(&self, index: CastlingIndex) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T, const N: usize> IndexMut<CastlingIndex> for [T; N] {
    fn index_mut(&mut self, index: CastlingIndex) -> &mut Self::Output {
        &mut self[index as usize]
    }
}
