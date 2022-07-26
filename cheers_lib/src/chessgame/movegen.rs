use std::ops::Index;

use crate::{
    lookup_tables::*,
    moves::Move,
    types::{CastlingIndex::*, ColorIndex::*, PieceIndex::*},
};
use cheers_bitboards::*;

use super::ChessGame;

#[derive(Copy, Clone, Debug)]
pub struct MoveList {
    moves: [Move; 218],
    length: usize,
}

impl MoveList {
    pub fn new() -> Self {
        Self {
            moves: [Move::null(); 218],
            length: 0,
        }
    }
    pub fn push(&mut self, move_: Move) {
        self.moves[self.length] = move_;
        self.length += 1;
    }
    pub fn reset(&mut self) {
        self.length = 0;
    }
    pub fn inner(&self) -> &[Move] {
        &self.moves[..self.length]
    }
    pub fn inner_mut(&mut self) -> &mut [Move] {
        &mut self.moves[..self.length]
    }
    pub fn len(&self) -> usize {
        self.length
    }
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl Index<usize> for MoveList {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.moves[index]
    }
}

pub trait Capture {
    const CAPTURE: bool;
}
pub struct Captures {}
impl Capture for Captures {
    const CAPTURE: bool = true;
}
pub struct All {}
impl Capture for All {
    const CAPTURE: bool = false;
}

impl ChessGame {
    pub fn legal_moves(&self) -> MoveList {
        let mut moves = MoveList::new();
        self.generate_legal_moves::<All>(&mut moves);
        moves
    }

    pub fn generate_legal_moves<T: Capture>(&self, moves: &mut MoveList) {
        moves.reset();
        let color = self.current_player;

        let king_square = self.piece_masks[(color, King)].first_square();

        // King moves
        let kingless_blocking_mask =
            (self.color_masks[color] ^ self.piece_masks[(color, King)]) | self.color_masks[!color];
        let attacked_squares = self.all_attacks(!color, kingless_blocking_mask);
        let king_moves =
            self.king_attacks(color) & (attacked_squares | self.color_masks[color]).inverse();
        for target in king_moves {
            let capture = (target.bitboard() & self.color_masks[!color]).is_not_empty();
            if !T::CAPTURE || (T::CAPTURE && capture) {
                moves.push(Move::king_move(king_square, target, capture));
            }
        }

        // Check evasions
        let checkers = (lookup_pawn_attack(king_square, color) & self.piece_masks[(!color, Pawn)])
            | (lookup_knight(king_square) & self.piece_masks[(!color, Knight)])
            | (lookup_bishop(king_square, self.combined)
                & (self.piece_masks[(!color, Bishop)] | self.piece_masks[(!color, Queen)]))
            | (lookup_rook(king_square, self.combined)
                & (self.piece_masks[(!color, Rook)] | self.piece_masks[(!color, Queen)]));

        let num_checkers = checkers.count_ones();
        // - Double Check
        // only king moves are legal in double+ check
        if num_checkers > 1 {
            return;
        }

        // mask of square a piece can capture on
        let mut capture_mask = BitBoard(0xFFFFFFFFFFFFFFFFu64);
        // mask of squares a piece can move to
        let mut push_mask = BitBoard(0xFFFFFFFFFFFFFFFFu64);
        // - Single Check
        if num_checkers == 1 {
            capture_mask = checkers;

            let checker_square = checkers.first_square();
            if self.piece_at(checker_square).is_slider() {
                // if the checking piece is a slider, we can push a piece to block it
                let slider_rays;
                if (king_square.rank()) == checker_square.rank()
                    || (king_square.file()) == checker_square.file()
                {
                    // orthogonal slider
                    slider_rays = lookup_rook(king_square, checker_square.bitboard());
                    push_mask = lookup_rook(checker_square, king_square.bitboard()) & slider_rays;
                } else {
                    // diagonal slider
                    slider_rays = lookup_bishop(king_square, checker_square.bitboard());
                    push_mask = lookup_bishop(checker_square, king_square.bitboard()) & slider_rays;
                }
            } else {
                // if the piece is not a slider, we can only capture
                push_mask = BitBoard::empty();
            }
        }
        // Pinned pieces
        let mut pinned_pieces = BitBoard::empty();

        let orthogonal_pin_rays = lookup_rook(king_square, self.color_masks[!color]);
        let pinning_orthogonals = (self.piece_masks[(!color, Rook)]
            | self.piece_masks[(!color, Queen)])
            & orthogonal_pin_rays;
        for pinner_square in pinning_orthogonals {
            let pin_ray = lookup_between(king_square, pinner_square);

            if (pin_ray & self.color_masks[color]).count_ones() == 1 {
                // there is only one piece on this ray so there is a pin
                // we only need to generate moves for rooks, queens and pawn pushes in this case

                // add any pinned piece to the mask
                pinned_pieces |= pin_ray & self.color_masks[color];

                let pinned_rook_or_queen =
                    pin_ray & (self.piece_masks[(color, Rook)] | self.piece_masks[(color, Queen)]);
                if pinned_rook_or_queen.is_not_empty() {
                    let rook_square = pinned_rook_or_queen.first_square();
                    let rook_moves = (pin_ray | pinner_square.bitboard())
                        & (push_mask | capture_mask)
                        & pinned_rook_or_queen.inverse();
                    for target in rook_moves {
                        let capture = target == pinner_square;
                        if !T::CAPTURE || (T::CAPTURE && capture) {
                            moves.push(Move::new(
                                rook_square,
                                target,
                                self.piece_at(rook_square),
                                NoPiece,
                                capture,
                                false,
                                false,
                                false,
                            ));
                        }
                    }
                }
                // no pawn pushes are captures
                if !T::CAPTURE {
                    let pinned_pawn = pin_ray & self.piece_masks[(color, Pawn)];
                    if pinned_pawn.is_not_empty() {
                        let pawn_square = pinned_pawn.first_square();
                        let mut pawn_moves = lookup_pawn_push(pawn_square, color)
                            & pin_ray
                            & push_mask
                            & self.combined.inverse();
                        if pawn_moves.is_not_empty()
                            && ((color == White
                                && pawn_square.rank() == 1
                                && (self.combined & pawn_square.offset(0, 2).bitboard())
                                    .is_empty())
                                || (color == Black
                                    && pawn_square.rank() == 6
                                    && (self.combined & pawn_square.offset(0, -2).bitboard())
                                        .is_empty()))
                        {
                            pawn_moves |= lookup_pawn_push(pawn_moves.first_square(), color)
                        }
                        for target in pawn_moves {
                            moves.push(Move::new(
                                pawn_square,
                                target,
                                Pawn,
                                NoPiece,
                                false,
                                // double pawn push
                                target.abs_diff(*pawn_square) == 16,
                                false,
                                false,
                            ));
                        }
                    }
                }
            }
        }
        let diagonal_pin_rays = lookup_bishop(king_square, self.color_masks[!color]);
        let pinning_diagonals = (self.piece_masks[(!color, Bishop)]
            | self.piece_masks[(!color, Queen)])
            & diagonal_pin_rays;
        for pinner_square in pinning_diagonals {
            let pin_ray = lookup_between(king_square, pinner_square);

            if (pin_ray & self.color_masks[color]).count_ones() == 1 {
                // there is only the king and one piece on this ray so there is a pin
                // we only need to generate moves for bishops, queens and pawn captures in this case

                // add any pinned piece to the mask
                pinned_pieces |= pin_ray & self.color_masks[color];

                let pinned_bishop_or_queen = pin_ray
                    & (self.piece_masks[(color, Bishop)] | self.piece_masks[(color, Queen)]);
                if pinned_bishop_or_queen.is_not_empty() {
                    let bishop_square = pinned_bishop_or_queen.first_square();
                    let bishop_moves = (pin_ray | pinner_square.bitboard())
                        & (push_mask | capture_mask)
                        & pinned_bishop_or_queen.inverse();
                    for target in bishop_moves {
                        let capture = target == pinner_square;
                        if !T::CAPTURE || (T::CAPTURE && capture) {
                            moves.push(Move::new(
                                bishop_square,
                                target,
                                self.piece_at(bishop_square),
                                NoPiece,
                                capture,
                                false,
                                false,
                                false,
                            ));
                        }
                    }
                }

                let pinned_pawn = pin_ray & self.piece_masks[(color, Pawn)];
                if pinned_pawn.is_not_empty() {
                    let pawn_square = pinned_pawn.first_square();
                    let pawn_moves = lookup_pawn_attack(pawn_square, color)
                        & pinner_square.bitboard()
                        & capture_mask
                        & (self.color_masks[!color] | self.en_passent_mask);
                    for target in pawn_moves {
                        let target: Square = target;
                        if target.rank() == !color as usize * 7 {
                            // pinned pawn capture promotions
                            moves.push(Move::pawn_capture_promotion(pawn_square, target, Knight));
                            moves.push(Move::pawn_capture_promotion(pawn_square, target, Bishop));
                            moves.push(Move::pawn_capture_promotion(pawn_square, target, Rook));
                            moves.push(Move::pawn_capture_promotion(pawn_square, target, Queen));
                        } else {
                            moves.push(Move::new(
                                pawn_square,
                                target,
                                Pawn,
                                NoPiece,
                                true,
                                false,
                                // en passent capture
                                match self.en_passent_mask {
                                    BitBoard(0) => false,
                                    _ => target == self.en_passent_mask.first_square(),
                                },
                                false,
                            ));
                        }
                    }
                }
            }
        }

        // Other moves
        // Castling if not in check and not generating captures
        if num_checkers == 0 && !T::CAPTURE {
            let king = self.piece_masks[(color, King)];
            if self.castling_rights[(color, Kingside)]
                && (self.combined & (king << 1 | king << 2)).is_empty()
                && (attacked_squares & (king << 1 | king << 2)).is_empty()
            {
                // generate castling kingside if rights remain, the way is clear and the squares aren't attacked
                let start = king.first_square();
                moves.push(Move::king_castle(start, start.offset(2, 0)));
            }
            if self.castling_rights[(color, Queenside)]
                && ((self.combined) & (king >> 1 | king >> 2 | king >> 3)).is_empty()
                && (attacked_squares & (king >> 1 | king >> 2)).is_empty()
            {
                // generate castling queenside if rights remain, the way is clear and the squares aren't attacked
                let start = king.first_square();
                moves.push(Move::king_castle(start, start.offset(-2, 0)));
            }
        }
        // Pawn moves
        let pawns = self.piece_masks[(color, Pawn)] & pinned_pieces.inverse();
        if color == White {
            // white pawn moves
            for pawn_square in pawns {
                let pawn_square: Square = pawn_square;
                let pawn = pawn_square.bitboard();

                if !T::CAPTURE {
                    // single pawn pushes
                    let pawn_push_one = (pawn << 8) & push_mask & (self.combined).inverse();
                    if pawn_push_one.is_not_empty() {
                        let target: Square = pawn_push_one.first_square();
                        // promotions
                        if target.rank() == 7 {
                            moves.push(Move::pawn_push_promotion(pawn_square, target, Knight));
                            moves.push(Move::pawn_push_promotion(pawn_square, target, Bishop));
                            moves.push(Move::pawn_push_promotion(pawn_square, target, Rook));
                            moves.push(Move::pawn_push_promotion(pawn_square, target, Queen));
                        } else {
                            // no promotion
                            moves.push(Move::pawn_push(pawn_square, target));
                        }
                    }
                    // double pawn pushes
                    let pawn_push_two = ((((pawn & SECOND_RANK) << 8) & (self.combined).inverse())
                        << 8)
                        & (self.combined).inverse()
                        & push_mask;

                    if pawn_push_two.is_not_empty() {
                        moves.push(Move::pawn_double_push(
                            pawn_square,
                            pawn_push_two.first_square(),
                        ));
                    }
                }
                // pawn captures
                let pawn_captures = (((pawn & NOT_A_FILE) << 7) | ((pawn & NOT_H_FILE) << 9))
                    // if a double-pushed pawn is giving check, mark it as takeable by en passent
                    & (capture_mask | (self.en_passent_mask & (capture_mask << 8)))
                    & (self.color_masks[!color] | self.en_passent_mask);
                for target in pawn_captures {
                    let target: Square = target;
                    if target.rank() == 7 {
                        // promotions
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Knight));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Bishop));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Rook));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Queen));
                    } else if target.bitboard() == self.en_passent_mask {
                        // en passent capture
                        if self.piece_masks[(color, King)].first_square().rank() == 4 {
                            let mut en_passent_pinned = false;
                            let blocking_mask = self.combined
                                & (pawn_square.bitboard() | (self.en_passent_mask >> 8)).inverse();
                            let attacking_rooks_or_queens = (self.piece_masks[(!color, Rook)]
                                | self.piece_masks[(!color, Queen)])
                                & FIFTH_RANK;
                            for rook_square in attacking_rooks_or_queens {
                                if (lookup_rook(rook_square, blocking_mask)
                                    & self.piece_masks[(color, King)])
                                    .is_not_empty()
                                {
                                    en_passent_pinned = true;
                                    break;
                                }
                            }
                            let attacking_queens = self.piece_masks[(!color, Queen)] & FOURTH_RANK;
                            for queen_square in attacking_queens {
                                if (lookup_queen(queen_square, blocking_mask)
                                    & self.piece_masks[(color, King)])
                                    .is_not_empty()
                                {
                                    en_passent_pinned = true;
                                    break;
                                }
                            }
                            if !en_passent_pinned {
                                moves.push(Move::pawn_enpassent_capture(pawn_square, target));
                            }
                        } else {
                            moves.push(Move::pawn_enpassent_capture(pawn_square, target));
                        }
                    } else {
                        // normal captures
                        moves.push(Move::pawn_capture(pawn_square, target));
                    }
                }
            }
        } else {
            // black pawn moves
            for pawn_square in pawns {
                let pawn_square: Square = pawn_square;
                let pawn = pawn_square.bitboard();

                if !T::CAPTURE {
                    // single pawn pushes
                    let pawn_push_one = pawn >> 8 & push_mask & (self.combined).inverse();
                    if pawn_push_one.is_not_empty() {
                        let target: Square = pawn_push_one.first_square();
                        // promotions
                        if target.rank() == 0 {
                            moves.push(Move::pawn_push_promotion(pawn_square, target, Knight));
                            moves.push(Move::pawn_push_promotion(pawn_square, target, Bishop));
                            moves.push(Move::pawn_push_promotion(pawn_square, target, Rook));
                            moves.push(Move::pawn_push_promotion(pawn_square, target, Queen));
                        } else {
                            // no promotion
                            moves.push(Move::pawn_push(pawn_square, target));
                        }
                    }
                    // double pawn pushes
                    let pawn_push_two =
                        ((((pawn & SEVENTH_RANK) >> 8) & (self.combined).inverse()) >> 8)
                            & (self.combined).inverse()
                            & push_mask;
                    if pawn_push_two.is_not_empty() {
                        moves.push(Move::pawn_double_push(
                            pawn_square,
                            pawn_push_two.first_square(),
                        ));
                    }
                }
                // pawn captures
                let pawn_captures = (((pawn & NOT_A_FILE) >> 9) | ((pawn & NOT_H_FILE) >> 7))
                    // if a double-pushed pawn is giving check, mark it as takeable by en passent
                    & (capture_mask | (self.en_passent_mask & (capture_mask >> 8)))
                    & (self.color_masks[!color] | self.en_passent_mask);
                for target in pawn_captures {
                    let target: Square = target;
                    if target.rank() == 0 {
                        // promotions
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Knight));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Bishop));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Rook));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Queen));
                    } else if target.bitboard() == self.en_passent_mask {
                        // en passent capture
                        if self.piece_masks[(color, King)].first_square().rank() == 3 {
                            let mut en_passent_pinned = false;
                            let blocking_mask = (self.combined)
                                & (pawn_square.bitboard() | self.en_passent_mask << 8).inverse();
                            let attacking_rooks_or_queens = (self.piece_masks[(!color, Rook)]
                                | self.piece_masks[(!color, Queen)])
                                & FOURTH_RANK;
                            for rook_square in attacking_rooks_or_queens {
                                if (lookup_rook(rook_square, blocking_mask)
                                    & self.piece_masks[(color, King)])
                                    .is_not_empty()
                                {
                                    en_passent_pinned = true;
                                    break;
                                }
                            }
                            if !en_passent_pinned {
                                moves.push(Move::pawn_enpassent_capture(pawn_square, target));
                            }
                        } else {
                            moves.push(Move::pawn_enpassent_capture(pawn_square, target));
                        }
                    } else {
                        // normal captures
                        moves.push(Move::pawn_capture(pawn_square, target));
                    }
                }
            }
        }

        // Knight moves
        let knights = self.piece_masks[(color, Knight)] & pinned_pieces.inverse();
        for knight_square in knights {
            let attacks = lookup_knight(knight_square)
                & self.color_masks[color].inverse()
                & (push_mask | capture_mask);
            for target in attacks {
                let capture = (self.color_masks[!color] & target.bitboard()).is_not_empty();
                if !T::CAPTURE || (T::CAPTURE && capture) {
                    moves.push(Move::knight_move(knight_square, target, capture));
                }
            }
        }

        // Bishop moves
        let bishops = self.piece_masks[(color, Bishop)] & pinned_pieces.inverse();
        for bishop_square in bishops {
            let attacks = lookup_bishop(bishop_square, self.combined)
                & self.color_masks[color].inverse()
                & (push_mask | capture_mask);
            for target in attacks {
                let capture = (self.color_masks[!color] & target.bitboard()).is_not_empty();
                if !T::CAPTURE || (T::CAPTURE && capture) {
                    moves.push(Move::bishop_move(bishop_square, target, capture));
                }
            }
        }

        // Rook moves
        let rooks = self.piece_masks[(color, Rook)] & pinned_pieces.inverse();
        for rook_square in rooks {
            let attacks = lookup_rook(rook_square, self.combined)
                & self.color_masks[color].inverse()
                & (push_mask | capture_mask);
            for target in attacks {
                let capture = (self.color_masks[!color] & target.bitboard()).is_not_empty();
                if !T::CAPTURE || (T::CAPTURE && capture) {
                    moves.push(Move::rook_move(rook_square, target, capture));
                }
            }
        }

        // queen moves
        let queens = self.piece_masks[(color, Queen)] & pinned_pieces.inverse();
        for queen_square in queens {
            let attacks = lookup_queen(queen_square, self.combined)
                & self.color_masks[color].inverse()
                & (push_mask | capture_mask);
            for target in attacks {
                let capture = (self.color_masks[!color] & target.bitboard()).is_not_empty();
                if !T::CAPTURE || (T::CAPTURE && capture) {
                    moves.push(Move::queen_move(queen_square, target, capture));
                }
            }
        }
    }
}
