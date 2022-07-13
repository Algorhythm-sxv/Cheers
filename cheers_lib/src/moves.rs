use std::{fmt::Display, ops::Index};

use crate::{
    chessgame::ChessGame,
    types::{CastlingRights, PieceIndex, PieceIndex::*},
};
use cheers_bitboards::{BitBoard, Square};

pub fn coord(square: Square) -> String {
    let mut res = String::new();
    res.push(match square.file() {
        0 => 'a',
        1 => 'b',
        2 => 'c',
        3 => 'd',
        4 => 'e',
        5 => 'f',
        6 => 'g',
        7 => 'h',
        _ => unreachable!(),
    });
    res.push(match square.rank() {
        0 => '1',
        1 => '2',
        2 => '3',
        3 => '4',
        4 => '5',
        5 => '6',
        6 => '7',
        7 => '8',
        _ => unreachable!(),
    });
    res
}

pub fn square(coord: &str) -> Square {
    let mut result = match coord.chars().next().unwrap() {
        'a' => 0,
        'b' => 1,
        'c' => 2,
        'd' => 3,
        'e' => 4,
        'f' => 5,
        'g' => 6,
        'h' => 7,
        _ => unreachable!(),
    };
    result += match coord.chars().nth(1).unwrap() {
        '1' => 0,
        '2' => 8,
        '3' => 2 * 8,
        '4' => 3 * 8,
        '5' => 4 * 8,
        '6' => 5 * 8,
        '7' => 6 * 8,
        '8' => 7 * 8,
        _ => unreachable!(),
    };
    result.into()
}

// start: 0-7
// target: 8-15
// piece: 16-18
// promotion: 19-21
// capture: 22
// double_pawn_push: 23
// enpassent_capture: 24
// castling: 25
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Move {
    data: u32,
    pub score: i32,
}

impl Move {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        start: Square,
        target: Square,
        piece: PieceIndex,
        promotion: PieceIndex,
        capture: bool,
        double_pawn_push: bool,
        enpassent_capture: bool,
        castling: bool,
    ) -> Self {
        let mut res = 0u32;
        res |= *start as u32
            | (*target as u32) << 8
            | (piece as u32) << (8 + 8)
            | (promotion as u32) << (8 + 8 + 3)
            | (capture as u32) << (8 + 8 + 3 + 3)
            | (double_pawn_push as u32) << (8 + 8 + 3 + 3 + 1)
            | (enpassent_capture as u32) << (8 + 8 + 3 + 3 + 1 + 1)
            | (castling as u32) << (8 + 8 + 3 + 3 + 1 + 1 + 1);

        Self {
            data: res,
            score: 0,
        }
    }

    pub fn null() -> Self {
        Self::new(
            Square::A1,
            Square::A1,
            NoPiece,
            NoPiece,
            false,
            false,
            false,
            false,
        )
    }

    pub fn from_pair(boards: &ChessGame, xy: impl AsRef<str>) -> Self {
        let (x, yp) = xy.as_ref().trim().split_at(2);
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
        let start = square(x).into();
        let target = square(y).into();
        let piece = boards.piece_at(start);
        Self::new(
            start,
            target,
            piece,
            p,
            boards.piece_at(target) != NoPiece,
            piece == Pawn && (target).abs_diff(*start) == 16,
            piece == Pawn && Some(target) == boards.en_passent_square(),
            piece == King && (target).abs_diff(*start) == 2,
        )
    }
    pub fn pawn_push(start: Square, target: Square) -> Self {
        Self::new(start, target, Pawn, NoPiece, false, false, false, false)
    }
    pub fn pawn_double_push(start: Square, target: Square) -> Self {
        Self::new(start, target, Pawn, NoPiece, false, true, false, false)
    }
    pub fn pawn_push_promotion(start: Square, target: Square, promotion: PieceIndex) -> Self {
        Self::new(start, target, Pawn, promotion, false, false, false, false)
    }
    pub fn pawn_capture(start: Square, target: Square) -> Self {
        Self::new(start, target, Pawn, NoPiece, true, false, false, false)
    }
    pub fn pawn_capture_promotion(start: Square, target: Square, promotion: PieceIndex) -> Self {
        Self::new(start, target, Pawn, promotion, true, false, false, false)
    }
    pub fn pawn_enpassent_capture(start: Square, target: Square) -> Self {
        Self::new(start, target, Pawn, NoPiece, true, false, true, false)
    }

    pub fn king_move(start: Square, target: Square, capture: bool) -> Self {
        Self::new(start, target, King, NoPiece, capture, false, false, false)
    }

    pub fn king_castle(start: Square, target: Square) -> Self {
        Self::new(start, target, King, NoPiece, false, false, false, true)
    }

    pub fn knight_move(start: Square, target: Square, capture: bool) -> Self {
        Self::new(start, target, Knight, NoPiece, capture, false, false, false)
    }

    pub fn bishop_move(start: Square, target: Square, capture: bool) -> Self {
        Self::new(start, target, Bishop, NoPiece, capture, false, false, false)
    }

    pub fn rook_move(start: Square, target: Square, capture: bool) -> Self {
        Self::new(start, target, Rook, NoPiece, capture, false, false, false)
    }

    pub fn queen_move(start: Square, target: Square, capture: bool) -> Self {
        Self::new(start, target, Queen, NoPiece, capture, false, false, false)
    }

    pub fn start(&self) -> Square {
        (self.data & 0xff).into()
    }

    pub fn target(&self) -> Square {
        ((self.data >> 8) & 0xff).into()
    }

    pub fn piece(&self) -> PieceIndex {
        PieceIndex::from_u8(((self.data >> (8 + 8)) & 0b111) as u8)
    }

    pub fn promotion(&self) -> PieceIndex {
        PieceIndex::from_u8(((self.data >> (8 + 8 + 3)) & 0b111) as u8)
    }

    pub fn capture(&self) -> bool {
        ((self.data >> (8 + 8 + 3 + 3)) & 0x1) == 1
    }

    pub fn double_pawn_push(&self) -> bool {
        ((self.data >> (8 + 8 + 3 + 3 + 1)) & 0x1) == 1
    }

    pub fn en_passent(&self) -> bool {
        ((self.data >> (8 + 8 + 3 + 3 + 1 + 1)) & 0x1) == 1
    }

    pub fn castling(&self) -> bool {
        ((self.data >> (8 + 8 + 3 + 3 + 1 + 1 + 1)) & 0x1) == 1
    }

    pub fn coords(&self) -> String {
        format!(
            "{}{}{}",
            coord(self.start()),
            coord(self.target()),
            match self.promotion() {
                Knight => "n",
                Bishop => "b",
                Rook => "r",
                Queen => "q",
                _ => "",
            }
        )
    }

    pub fn is_null(&self) -> bool {
        self.start() == self.target()
    }
}

impl Default for Move {
    fn default() -> Self {
        Self::null()
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\ns/e\t\tpiece\t\tprom\t\tcapture\t\tdp\t\tep\t\tcastle")?;
        Ok(write!(
            f,
            "{}{}\t\t{}\t\t{}\t\t{}\t\t{}\t\t{}\t\t{}",
            coord(self.start()),
            coord(self.target()),
            self.piece(),
            self.promotion(),
            self.capture(),
            self.double_pawn_push(),
            self.en_passent(),
            self.castling()
        )?)
    }
}

#[derive(Clone, Copy)]
pub struct UnMove {
    pub start: Square,
    pub target: Square,
    pub promotion: bool,
    pub capture: PieceIndex,
    pub en_passent: bool,
    pub en_passent_mask: BitBoard,
    pub castling: bool,
    pub castling_rights: CastlingRights,
    pub halfmove_clock: u8,
}

impl UnMove {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        start: Square,
        target: Square,
        promotion: bool,
        capture: PieceIndex,
        en_passent: bool,
        en_passent_mask: BitBoard,
        castling: bool,
        castling_rights: CastlingRights,
        halfmove_clock: u8,
    ) -> Self {
        Self {
            start,
            target,
            promotion,
            capture,
            en_passent,
            en_passent_mask,
            castling,
            castling_rights,
            halfmove_clock,
        }
    }
}

pub fn pick_move(move_list: &mut [Move], current_index: usize) {
    let mut best_index = current_index;

    for i in (current_index + 1)..move_list.len() {
        if move_list[i].score > move_list[best_index].score {
            best_index = i;
        }
    }

    move_list.swap(current_index, best_index);
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
