use super::*;

use crate::moves::*;
use crate::types::*;
use Piece::*;

impl Board {
    pub fn legal_move_list(&self) -> Vec<Move> {
        let mut moves = Vec::new();
        self.generate_legal_moves(|mvs| {
            for mv in mvs {
                moves.push(mv);
            }
        });

        moves
    }

    pub fn generate_legal_moves_into(&self, list: &mut MoveList) {
        list.reset();
        self.generate_legal_moves(|mvs| {
            for mv in mvs {
                list.push(SortingMove::new(mv))
            }
        });
    }

    pub fn generate_legal_captures_into(&self, list: &mut MoveList) {
        list.reset();
        self.generate_legal_moves(|mvs| {
            for mv in mvs {
                if mv.promotion() != Pawn || self.is_capture(mv) {
                    list.push(SortingMove::new(mv))
                }
            }
        });
    }

    pub fn generate_legal_moves(&self, mut listener: impl FnMut(MoveMask)) {
        if self.black_to_move {
            self.legal_moves::<Black, _>(&mut listener)
        } else {
            self.legal_moves::<White, _>(&mut listener)
        }
    }

    fn legal_moves<T: TypeColor, F: FnMut(MoveMask)>(&self, listener: &mut F) {
        if self.check_mask == FULL_BOARD {
            // no check
            if self.ep_mask.is_not_empty() {
                self.legal_pawn_moves::<T, NotInCheck, Ep, F>(listener);
            } else {
                self.legal_pawn_moves::<T, NotInCheck, NoEp, F>(listener);
            }
            self.legal_knight_moves::<T, NotInCheck, F>(listener);
            self.legal_bishop_moves::<T, NotInCheck, F>(listener);
            self.legal_rook_moves::<T, NotInCheck, F>(listener);
            self.legal_queen_moves::<T, NotInCheck, F>(listener);
            let color = if T::WHITE { 0 } else { 1 };
            if self.castling_rights[color].iter().any(|b| b.is_not_empty()) {
                self.legal_king_moves::<T, NotInCheck, Castling, F>(listener);
            } else {
                self.legal_king_moves::<T, NotInCheck, NoCastling, F>(listener);
            }
        } else if self.check_mask.is_not_empty() {
            // single check
            if self.ep_mask.is_not_empty() {
                self.legal_pawn_moves::<T, InCheck, Ep, F>(listener);
            } else {
                self.legal_pawn_moves::<T, InCheck, NoEp, F>(listener);
            }
            self.legal_knight_moves::<T, InCheck, F>(listener);
            self.legal_bishop_moves::<T, InCheck, F>(listener);
            self.legal_rook_moves::<T, InCheck, F>(listener);
            self.legal_queen_moves::<T, InCheck, F>(listener);
            self.legal_king_moves::<T, InCheck, NoCastling, F>(listener);
        } else {
            // double check
            self.legal_king_moves::<T, InCheck, NoCastling, F>(listener);
        }
    }

    #[inline(always)]
    pub fn valid_move_targets<T: TypeColor, C: TypeCheck>(&self) -> BitBoard {
        let mut targets = if T::WHITE {
            self.white_pieces.inverse()
        } else {
            self.black_pieces.inverse()
        };
        if C::IN_CHECK {
            targets &= self.check_mask;
        }
        targets
    }

    #[inline(always)]
    pub fn legal_pawn_moves<T: TypeColor, C: TypeCheck, E: EpPossible, F: FnMut(MoveMask)>(
        &self,
        listener: &mut F,
    ) {
        let (pawns, enemy_pieces) = if T::WHITE {
            (self.white_pawns, self.black_pieces)
        } else {
            (self.black_pawns, self.white_pieces)
        };

        // unpinned pawns
        for pawn in pawns & (self.diagonal_pin_mask | self.orthogonal_pin_mask).inverse() {
            let pushes = self.pawn_pushes::<T>(pawn);
            let attacks = Self::pawn_attack::<T>(pawn);
            let captures = attacks & enemy_pieces;
            let ep = if E::EP_POSSIBLE {
                let mut ep = attacks & self.ep_mask;
                // ep is illegal if the captured pawn would be pinned
                let ep_target = self.ep_target::<T>();
                // TODO: I don't think any legal positions can have ep discover a diagonal check
                // when the friendly pawn is unpinned
                // ep &= self.forward::<T>(ep_target & (self.diagonal_pin_mask).inverse());

                // horizontal pin check
                let (king, enemy_orthogonals) = if T::WHITE {
                    (
                        self.white_king.first_square(),
                        self.black_rooks | self.black_queens,
                    )
                } else {
                    (
                        self.black_king.first_square(),
                        self.white_rooks | self.white_queens,
                    )
                };
                if (lookup_rook(
                    king,
                    self.occupied ^ (pawn.bitboard() | ep_target) | self.ep_mask,
                ) & enemy_orthogonals)
                    .is_not_empty()
                {
                    ep = BitBoard::empty();
                }

                ep
            } else {
                BitBoard::empty()
            };

            listener(MoveMask {
                piece: Pawn,
                start: pawn,
                moves: ((pushes | captures) & self.check_mask)
                    | (ep & (self.forward::<T>(self.check_mask))),
            })
        }

        // pinned pawns can only move when not in check
        if !C::IN_CHECK {
            // orthogonally pinned pawns cannot capture
            for pawn in pawns & (self.orthogonal_pin_mask) {
                let pushes = self.pawn_pushes::<T>(pawn) & self.orthogonal_pin_mask;
                listener(MoveMask {
                    piece: Pawn,
                    start: pawn,
                    moves: pushes,
                })
            }

            // diagonally pinned pawns can only capture along the line of the pin
            for pawn in pawns & self.diagonal_pin_mask {
                let attacks = Self::pawn_attack::<T>(pawn);
                let captures = attacks & enemy_pieces & self.diagonal_pin_mask;

                // diagonally pinned pawns can ep, but can't be horizontally pinned
                let ep = if E::EP_POSSIBLE {
                    let ep = attacks & self.ep_mask & self.diagonal_pin_mask;
                    // ep is illegal if the captured pawn would be pinned
                    let ep_target = self.ep_target::<T>();
                    ep & self.forward::<T>(ep_target & (self.diagonal_pin_mask).inverse())
                } else {
                    BitBoard::empty()
                };

                listener(MoveMask {
                    piece: Pawn,
                    start: pawn,
                    moves: captures | ep,
                })
            }
        }
    }

    #[inline(always)]
    pub fn legal_knight_moves<T: TypeColor, C: TypeCheck, F: FnMut(MoveMask)>(
        &self,
        listener: &mut F,
    ) {
        let knights = if T::WHITE {
            self.white_knights
        } else {
            self.black_knights
        };
        let valid_targets = self.valid_move_targets::<T, C>();

        // only unpinned knights can move
        for knight in knights & (self.diagonal_pin_mask | self.orthogonal_pin_mask).inverse() {
            let moves = lookup_knight(knight) & valid_targets;
            listener(MoveMask {
                piece: Knight,
                start: knight,
                moves,
            })
        }
    }

    #[inline(always)]
    pub fn legal_bishop_moves<T: TypeColor, C: TypeCheck, F: FnMut(MoveMask)>(
        &self,
        listener: &mut F,
    ) {
        let bishops = if T::WHITE {
            self.white_bishops
        } else {
            self.black_bishops
        };
        let mut valid_targets = self.valid_move_targets::<T, C>();

        // unpinned bishops
        for bishop in bishops & (self.diagonal_pin_mask | self.orthogonal_pin_mask).inverse() {
            let moves = lookup_bishop(bishop, self.occupied) & valid_targets;
            listener(MoveMask {
                piece: Bishop,
                start: bishop,
                moves,
            });
        }

        // pinned bishops can only move when not in check
        if !C::IN_CHECK {
            // pinned bishops can only move along the line of the pin
            valid_targets &= self.diagonal_pin_mask;
            // orthogonally pinned bishops can never move
            for bishop in bishops & self.diagonal_pin_mask {
                let moves = lookup_bishop(bishop, self.occupied) & valid_targets;
                listener(MoveMask {
                    piece: Bishop,
                    start: bishop,
                    moves,
                });
            }
        }
    }

    #[inline(always)]
    pub fn legal_rook_moves<T: TypeColor, C: TypeCheck, F: FnMut(MoveMask)>(
        &self,
        listener: &mut F,
    ) {
        let rooks = if T::WHITE {
            self.white_rooks
        } else {
            self.black_rooks
        };
        let mut valid_targets = self.valid_move_targets::<T, C>();

        // unpinned rooks
        for rook in rooks & (self.diagonal_pin_mask | self.orthogonal_pin_mask).inverse() {
            let moves = lookup_rook(rook, self.occupied) & valid_targets;
            listener(MoveMask {
                piece: Rook,
                start: rook,
                moves,
            });
        }

        // pinned rooks can only move when not in check
        if !C::IN_CHECK {
            // pinned rooks can only move along the line of the pin
            valid_targets &= self.orthogonal_pin_mask;
            // diagonally pinned rooks can never move
            for rook in rooks & self.orthogonal_pin_mask {
                let moves = lookup_rook(rook, self.occupied) & valid_targets;
                listener(MoveMask {
                    piece: Rook,
                    start: rook,
                    moves,
                })
            }
        }
    }

    #[inline(always)]
    pub fn legal_queen_moves<T: TypeColor, C: TypeCheck, F: FnMut(MoveMask)>(
        &self,
        listener: &mut F,
    ) {
        let queens = if T::WHITE {
            self.white_queens
        } else {
            self.black_queens
        };
        let valid_targets = self.valid_move_targets::<T, C>();

        // unpinned queens
        for queen in queens & (self.orthogonal_pin_mask | self.diagonal_pin_mask).inverse() {
            let moves = lookup_queen(queen, self.occupied) & valid_targets;
            listener(MoveMask {
                piece: Queen,
                start: queen,
                moves,
            })
        }

        // pinned queens can only move when not in check
        if !C::IN_CHECK {
            // diagonally pinned queens can only move along the line of the pin
            let diagonal_targets = valid_targets & self.diagonal_pin_mask;
            for queen in queens & self.diagonal_pin_mask {
                let moves = lookup_bishop(queen, self.occupied) & diagonal_targets;
                listener(MoveMask {
                    piece: Queen,
                    start: queen,
                    moves,
                });
            }

            // orthogonally pinned queens can only move along the line of the pin
            let orthogonal_targets = valid_targets & self.orthogonal_pin_mask;
            for queen in queens & self.orthogonal_pin_mask {
                let moves = lookup_rook(queen, self.occupied) & orthogonal_targets;
                listener(MoveMask {
                    piece: Queen,
                    start: queen,
                    moves,
                })
            }
        }
    }

    #[inline(always)]
    pub fn legal_king_moves<T: TypeColor, C: TypeCheck, P: CastlingPossible, F: FnMut(MoveMask)>(
        &self,
        listener: &mut F,
    ) {
        let king = if T::WHITE {
            self.white_king.first_square()
        } else {
            self.black_king.first_square()
        };
        if !C::IN_CHECK {
            let mut moves = lookup_king(king)
                & self.valid_move_targets::<T, NotInCheck>()
                & self.king_safe_squares::<T>();

            if P::CASTLING_POSSIBLE {
                // castling is only possible when not in check
                let rights = if T::WHITE {
                    self.castling_rights[0]
                } else {
                    self.castling_rights[1]
                };

                // short castling (chess960 rules)
                if rights[0].is_not_empty() && self.castling_legal::<T>(false) {
                    moves |= rights[0]
                }

                // long castling (chess960 rules)
                if rights[1].is_not_empty() && self.castling_legal::<T>(true) {
                    moves |= rights[1]
                }
            }

            listener(MoveMask {
                piece: King,
                start: king,
                moves,
            })
        } else {
            let moves = lookup_king(king)
                & self.valid_move_targets::<T, NotInCheck>()
                & self.king_safe_squares::<T>();
            listener(MoveMask {
                piece: King,
                start: king,
                moves,
            })
        }
    }

    #[inline(always)]
    pub fn castling_legal<T: TypeColor>(&self, queenside: bool) -> bool {
        let (king, rook, king_target, rook_target) = if T::WHITE {
            (
                self.white_king.first_square(),
                self.castling_rights[0][queenside as usize].first_square(),
                if queenside { Square::C1 } else { Square::G1 },
                if queenside { Square::D1 } else { Square::F1 },
            )
        } else {
            (
                self.black_king.first_square(),
                self.castling_rights[1][queenside as usize].first_square(),
                if queenside { Square::C8 } else { Square::G8 },
                if queenside { Square::D8 } else { Square::F8 },
            )
        };
        let must_be_safe = lookup_between(king, king_target) | king_target.bitboard();
        let must_be_empty = must_be_safe | lookup_between(king, rook) | rook_target.bitboard();
        let blockers = self.occupied ^ king.bitboard() ^ rook.bitboard();
        (rook.bitboard() & self.orthogonal_pin_mask).is_empty()
            && (blockers & must_be_empty).is_empty()
            && (must_be_safe & self.all_enemy_attacks::<T>(self.occupied)).is_empty()
    }
}
