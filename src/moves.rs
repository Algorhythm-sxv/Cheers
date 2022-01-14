use std::fmt::Display;

use crate::{
    bitboards::BitBoards,
    types::{PieceIndex, PieceIndex::*},
};

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
        _ => unreachable!(),
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

pub fn square(coord: &str) -> u8 {
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
    result
}

#[derive(Clone, Copy)]
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
    #[allow(clippy::too_many_arguments)]
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

    pub fn null() -> Self {
        Self::new(0, 0, NoPiece, NoPiece, false, false, false, false)
    }

    pub fn from_pair(boards: &BitBoards, xy: impl AsRef<str>) -> Self {
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
        let start = square(x) as usize;
        let target = square(y) as usize;
        let piece = boards.piece_at(start);
        Self::new(
            start as u8,
            target as u8,
            piece,
            p,
            boards.piece_at(target) != NoPiece,
            // TODO: replace with `abs_diff` once stabilised
            piece == Pawn && (target as isize - start as isize).abs() == 16,
            piece == Pawn && target == boards.enpassent_square(),
            piece == King && (target as isize - start as isize).abs() == 2,
        )
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

    pub fn start(&self) -> u8 {
        self.start
    }

    pub fn target(&self) -> u8 {
        self.target
    }

    pub fn piece(&self) -> PieceIndex {
        self.piece
    }

    pub fn promotion(&self) -> PieceIndex {
        self.promotion
    }

    pub fn capture(&self) -> bool {
        self.capture
    }

    pub fn en_passent(&self) -> bool {
        self.enpassent_capture
    }

    pub fn castling(&self) -> bool {
        self.castling
    }

    pub fn double_pawn_push(&self) -> bool {
        self.double_pawn_push
    }

    pub fn coords(&self) -> String {
        format!(
            "{}{}{}",
            coord(self.start),
            coord(self.target),
            match self.promotion() {
                Knight => "n",
                Bishop => "b",
                Rook => "r",
                Queen => "q",
                _ => "",
            }
        )
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
