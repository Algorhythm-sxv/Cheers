use std::fmt::Display;

use crate::types::{PieceIndex, PieceIndex::*};

fn coord(square: u8) -> String {
    let mut res = String::new();
    res.push(match square % 8 {
        0 => 'a',
        1 => 'b',
        2 => 'c',
        3 => 'd',
        4 => 'e',
        5 => 'f',
        6 => 'g',
        7 => 'h',
        _ => unreachable!()
    });
    res.push(match square / 8 {
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

pub struct Move {
    start: u8,
    target: u8,
    piece: PieceIndex,
    promotion: PieceIndex,
    capture: bool,
    double_pawn_push: bool,
    enpassent_capture: bool,
    castling: bool,
}

impl Move {
    pub fn new(
        start: u8,
        target: u8,
        piece: PieceIndex,
        promotion: PieceIndex,
        capture: bool,
        double_pawn_push: bool,
        enpassent_capture: bool,
        castling: bool,
    ) -> Self {
        Self {
            start,
            target,
            piece,
            promotion,
            capture,
            double_pawn_push,
            enpassent_capture,
            castling,
        }
    }

    pub fn pawn_push(start: u8, target: u8) -> Self {
        Self::new(start, target, Pawn, NoPiece, false, false, false, false)
    }
    pub fn pawn_double_push(start: u8, target: u8) -> Self {
        Self::new(start, target, Pawn, NoPiece, false, true, false, false)
    }
    pub fn pawn_push_promotion(start: u8, target: u8, promotion: PieceIndex) -> Self {
        Self::new(start, target, Pawn, promotion, false, false, false, false)
    }
    pub fn pawn_capture(start: u8, target: u8) -> Self {
        Self::new(start, target, Pawn, NoPiece, true, false, false, false)
    }
    pub fn pawn_capture_promotion(start: u8, target: u8, promotion: PieceIndex) -> Self {
        Self::new(start, target, Pawn, promotion, true, false, false, false)
    }
    pub fn pawn_enpassent_capture(start: u8, target: u8) -> Self {
        Self::new(start, target, Pawn, NoPiece, true, false, true, false)
    }

    pub fn king_move(start: u8, target: u8, capture: bool) -> Self {
        Self::new(start, target, King, NoPiece, capture, false, false, false)
    }

    pub fn king_castle(start: u8, target: u8) -> Self {
        Self::new(start, target, King, NoPiece, false, false, false, true)
    }

    pub fn knight_move(start: u8, target: u8, capture: bool) -> Self {
        Self::new(start, target, Knight, NoPiece, capture, false, false, false)
    }

    pub fn bishop_move(start: u8, target: u8, capture: bool) -> Self {
        Self::new(start, target, Bishop, NoPiece, capture, false, false, false)
    }

    pub fn rook_move(start: u8, target: u8, capture: bool) -> Self {
        Self::new(start, target, Rook, NoPiece, capture, false, false, false)
    }

    pub fn queen_move(start: u8, target: u8, capture: bool) -> Self {
        Self::new(start, target, Queen, NoPiece, capture, false, false, false)
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\ns/e\t\tpiece\t\tprom\t\tcapture\t\tdp\t\tep\t\tcastle")?;
        Ok(write!(
            f,
            "{}{}\t\t{}\t\t{}\t\t{}\t\t{}\t\t{}\t\t{}",
            coord(self.start),
            coord(self.target),
            self.piece,
            self.promotion,
            self.capture,
            self.double_pawn_push,
            self.enpassent_capture,
            self.castling
        )?)
    }
}
