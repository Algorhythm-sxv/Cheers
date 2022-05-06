#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColorIndex {
    White = 0,
    Black = 1,
}

use std::fmt::Display;

use self::ColorIndex::*;

use crate::bitboard::BitBoard;

impl Default for ColorIndex {
    fn default() -> Self {
        White
    }
}

impl std::ops::Not for ColorIndex {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            White => Black,
            Black => White,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PieceIndex {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
    NoPiece = 6,
}

impl PieceIndex {
    pub fn is_slider(self) -> bool {
        self == Self::Bishop || self == Self::Rook || self == Self::Queen
    }
    pub fn from_u8(n: u8) -> Self {
        use self::PieceIndex::*;
        match n {
            0 => Pawn,
            1 => Knight,
            2 => Bishop,
            3 => Rook,
            4 => Queen,
            5 => King,
            _ => NoPiece,
        }
    }
}

impl Display for PieceIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use self::PieceIndex::*;
        Ok(write!(
            f,
            "{}",
            match self {
                Pawn => "Pawn",
                Knight => "Knight",
                Bishop => "Bishop",
                Rook => "Rook",
                Queen => "Queen",
                King => "King",
                NoPiece => "None",
            }
        )?)
    }
}

pub enum CastlingIndex {
    Queenside = 0,
    Kingside = 1,
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct ColorMasks(pub [BitBoard; 2]);

impl std::ops::Index<ColorIndex> for ColorMasks {
    type Output = BitBoard;

    fn index(&self, index: ColorIndex) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl std::ops::IndexMut<ColorIndex> for ColorMasks {
    fn index_mut(&mut self, index: ColorIndex) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct PieceMasks(pub [[BitBoard; 6]; 2]);
impl std::ops::Index<(ColorIndex, PieceIndex)> for PieceMasks {
    type Output = BitBoard;

    fn index(&self, index: (ColorIndex, PieceIndex)) -> &Self::Output {
        &self.0[index.0 as usize][index.1 as usize]
    }
}
impl std::ops::IndexMut<(ColorIndex, PieceIndex)> for PieceMasks {
    fn index_mut(&mut self, index: (ColorIndex, PieceIndex)) -> &mut Self::Output {
        &mut self.0[index.0 as usize][index.1 as usize]
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct CastlingRights(pub [[bool; 2]; 2]);
impl std::ops::Index<(ColorIndex, CastlingIndex)> for CastlingRights {
    type Output = bool;

    fn index(&self, index: (ColorIndex, CastlingIndex)) -> &Self::Output {
        &self.0[index.0 as usize][index.1 as usize]
    }
}
impl std::ops::IndexMut<(ColorIndex, CastlingIndex)> for CastlingRights {
    fn index_mut(&mut self, index: (ColorIndex, CastlingIndex)) -> &mut Self::Output {
        &mut self.0[index.0 as usize][index.1 as usize]
    }
}
impl std::ops::Index<ColorIndex> for CastlingRights {
    type Output = [bool; 2];

    fn index(&self, index: ColorIndex) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl std::ops::IndexMut<ColorIndex> for CastlingRights {
    fn index_mut(&mut self, index: ColorIndex) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}
