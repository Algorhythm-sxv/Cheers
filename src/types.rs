use crate::{evaluate::consts::PIECE_VALUES, piece_tables::GamePhase, utils::*};

/// Parses a pair of squares representing a move, returning the result of a promotion if it happened
pub fn parse_move_pair(pair: &str) -> Move {
    // TODO: null move handling
    let (x, yp) = pair.trim().split_at(2);
    let mut p = Pawn;

    let y = if yp.len() == 3 {
        p = match &yp[2..] {
            "q" => Queen,
            "r" => Rook,
            "n" => Knight,
            "b" => Bishop,
            _ => unreachable!(),
        };
        &yp[0..2]
    } else {
        yp
    };

    Move::new(coord_to_square(x), coord_to_square(y), p)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PieceIndex {
    Pawn = 0,
    Bishop = 1,
    Knight = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl PieceIndex {
    pub fn from_u8(from: u8) -> Self {
        match from {
            0 => Pawn,
            1 => Bishop,
            2 => Knight,
            3 => Rook,
            4 => Queen,
            5 => King,
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum ColorIndex {
    White = 0,
    Black = 1,
}

impl Default for ColorIndex {
    fn default() -> Self {
        White
    }
}

impl std::ops::Not for ColorIndex {
    type Output = ColorIndex;
    fn not(self) -> Self::Output {
        match self {
            White => Black,
            Black => White,
        }
    }
}

pub enum CastlingIndex {
    Queenside = 0,
    Kingside = 1,
}

pub use CastlingIndex::*;
pub use ColorIndex::*;
pub use PieceIndex::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Move {
    pub start: u8,
    pub target: u8,
    pub promotion: PieceIndex,
}

impl Move {
    pub fn new(start: u8, target: u8, promotion: PieceIndex) -> Self {
        Self {
            start,
            target,
            promotion,
        }
    }

    pub fn null() -> Self {
        Self::new(0, 0, Pawn)
    }

    pub fn is_null(&self) -> bool {
        self.start == 0 && self.target == 0
    }

    pub fn to_algebraic_notation(&self) -> String {
        let mut result = String::new();

        // null move
        if self.start == 0 && self.target == 0 {
            result.push_str("0000");
            return result;
        }
        result.push_str(&square_to_coord(self.start));
        result.push_str(&square_to_coord(self.target));
        result.push_str(match self.promotion {
            Pawn => "",
            Queen => "q",
            Rook => "r",
            Knight => "n",
            Bishop => "b",
            _ => unreachable!(),
        });
        result
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Capture {
    pub start: u8,
    pub target: u8,
    pub captor: PieceIndex,
    pub capture: PieceIndex,
    pub promotion: PieceIndex,
}

impl Capture {
    pub fn new(
        start: u8,
        target: u8,
        captor: PieceIndex,
        capture: PieceIndex,
        promotion: PieceIndex,
    ) -> Self {
        Self {
            start,
            target,
            captor,
            capture,
            promotion,
        }
    }

    pub fn null() -> Self {
        Self::new(0, 0, Pawn, Pawn, Pawn)
    }

    pub fn is_null(&self) -> bool {
        self.start == 0 && self.target == 0
    }

    pub fn to_algebraic_notation(&self) -> String {
        let mut result = String::new();

        // null move
        if self.start == 0 && self.target == 0 {
            result.push_str("0000");
            return result;
        }
        result.push_str(&square_to_coord(self.start));
        result.push_str(&square_to_coord(self.target));
        result.push_str(match self.promotion {
            Pawn => "",
            Queen => "q",
            Rook => "r",
            Knight => "n",
            Bishop => "b",
            _ => unreachable!(),
        });
        result
    }

    pub fn to_move(&self) -> Move {
        Move::new(self.start, self.target, self.promotion)
    }

    pub fn material_difference(&self) -> i32 {
        PIECE_VALUES[(GamePhase::MidGame, self.captor)]
            - PIECE_VALUES[(GamePhase::MidGame, self.capture)]
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct UnmakeMove {
    pub start: u8,
    pub target: u8,
    pub taken: Option<PieceIndex>,
    pub en_passent: bool,
    pub en_passent_mask: u64,
    pub castling: bool,
    pub castling_rights: CastlingRights,
    pub promotion: bool,
    pub halfmove_clock: u8,
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct ColorMasks(pub [u64; 2]);

impl std::ops::Index<ColorIndex> for ColorMasks {
    type Output = u64;

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
pub struct PieceMasks(pub [[u64; 6]; 2]);
impl std::ops::Index<(ColorIndex, PieceIndex)> for PieceMasks {
    type Output = u64;

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
