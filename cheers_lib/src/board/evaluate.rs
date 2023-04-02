use crate::{hash_tables::PawnHashTable, lookup_tables::*, types::*};
use cheers_bitboards::*;
use Piece::*;

use super::Board;
pub use super::{eval_params::*, eval_types::*};

pub struct EvalContext<'search, T> {
    game: &'search Board,
    pawn_hash_table: &'search mut PawnHashTable,
    trace: &'search mut T,
}

impl<'search, T: TraceTarget + Default> EvalContext<'search, T> {
    #[inline]
    pub fn evaluate<W: TypeColor>(&mut self) -> i16 {
        let color = W::INDEX;
        self.trace.term(|t| t.turn = color as i16);

        let pawn_cache = if !T::TRACING {
            self.pawn_hash_table.get::<W>(self.game.pawn_hash)
        } else {
            None
        };

        let phase = self.game.game_phase();

        let white_king_square = self.game.white_king.first_square();
        let black_king_square = self.game.black_king.first_square();

        let white_king_attacks = lookup_king(white_king_square);
        let black_king_attacks = lookup_king(black_king_square);

        let (white_passers, black_passers) = if let Some(cache) = pawn_cache {
            (
                cache.1 & self.game.white_pawns,
                cache.1 & self.game.black_pawns,
            )
        } else {
            let front_spans_black = Board::pawn_front_spans::<Black>(self.game.black_pawns);
            let all_front_spans_black = front_spans_black
                | (front_spans_black & NOT_H_FILE) << 1
                | (front_spans_black & NOT_A_FILE) >> 1;
            let rear_spans_black = Board::pawn_push_spans::<White>(self.game.black_pawns);

            let front_spans_white = Board::pawn_front_spans::<White>(self.game.white_pawns);
            let all_front_spans_white = front_spans_white
                | (front_spans_white & NOT_H_FILE) << 1
                | (front_spans_white & NOT_A_FILE) >> 1;
            let rear_spans_white = Board::pawn_push_spans::<Black>(self.game.white_pawns);

            let white_passers = self.game.white_pawns
                & all_front_spans_black.inverse()
                & rear_spans_white.inverse();
            let black_passers = self.game.black_pawns
                & all_front_spans_white.inverse()
                & rear_spans_black.inverse();

            (white_passers, black_passers)
        };

        // initialise eval info
        let mut info = EvalInfo {
            mobility_area: [
                self.game.mobility_area::<White>(),
                self.game.mobility_area::<Black>(),
            ],
            behind_pawns: [self.game.white_pawns >> 8, self.game.black_pawns << 8],
            outposts: [
                self.game.pawn_attack_spans::<Black>().inverse(),
                self.game.pawn_attack_spans::<White>().inverse(),
            ],
            seventh_rank: [SEVENTH_RANK, SECOND_RANK],
            king_square: [white_king_square, black_king_square],
            king_area: [
                white_king_attacks | (white_king_attacks << 8),
                black_king_attacks | (black_king_attacks >> 8),
            ],
            passed_pawns: [white_passers, black_passers],
        };

        let mut eval = EvalScore::zero();
        if !T::TRACING {
            match pawn_cache {
                None => {
                    let score = self.evaluate_pawns_only::<W>(&mut info)
                        - self.evaluate_pawns_only::<W::Other>(&mut info);
                    self.pawn_hash_table.set::<W>(
                        self.game.pawn_hash,
                        score,
                        white_passers | black_passers,
                    );
                    eval += score;
                }
                Some((val, _)) => {
                    eval += val;
                }
            }
        } else {
            eval += self.evaluate_pawns_only::<W>(&mut info)
                - self.evaluate_pawns_only::<W::Other>(&mut info);
        }

        eval += self.evaluate_passed_pawn_extras::<W>(&info)
            - self.evaluate_passed_pawn_extras::<W::Other>(&info);

        eval += self.evaluate_knights::<W>(&info) - self.evaluate_knights::<W::Other>(&info);

        eval += self.evaluate_bishops::<W>(&info) - self.evaluate_bishops::<W::Other>(&info);

        eval += self.evaluate_rooks::<W>(&info) - self.evaluate_rooks::<W::Other>(&info);

        eval += self.evaluate_queens::<W>(&info) - self.evaluate_queens::<W::Other>(&info);

        eval += self.evaluate_king::<W>(&info) - self.evaluate_king::<W::Other>(&info);

        if (W::WHITE && self.game.current_player() == Color::White)
            || (!W::WHITE && self.game.current_player() == Color::Black)
        {
            eval += EVAL_PARAMS.tempo;
            // tempo doesn't texel tune at all since it compares to WDL
            // self.trace.term(|t| t.tempo[W::INDEX] = 1);
        }

        // scale down evals for material draws
        if self.game.material_draw() {
            eval.div_by(32);
        }

        (((eval.mg() as i32 * (256 - phase)) + (eval.eg() as i32 * phase)) / 256) as i16
    }

    #[inline]
    pub fn evaluate_knights<W: TypeColor>(&mut self, info: &EvalInfo) -> EvalScore {
        let mut eval = EvalScore::zero();

        let knights = if W::WHITE {
            self.game.white_knights
        } else {
            self.game.black_knights
        };

        let color = W::INDEX;

        // material value
        let count = knights.count_ones() as i16;
        eval += EVAL_PARAMS.piece_values[Knight] * count;
        self.trace.term(|t| t.knight_count[color as usize] = count);

        // knights behind pawns
        let knights_behind_pawns =
            (knights & info.behind_pawns[color as usize]).count_ones() as i16;
        eval += EVAL_PARAMS.knight_behind_pawn * knights_behind_pawns;
        self.trace
            .term(|t| t.knights_behind_pawns[color] = knights_behind_pawns);

        for knight in knights {
            let relative_knight = relative_board_index::<W>(knight);
            // placement
            eval += EVAL_PARAMS.piece_tables[(Knight, relative_knight)];
            self.trace
                .term(|t| t.knight_placement[relative_knight][color] += 1);

            // king distance
            let king = info.king_square[color];
            let distance = king
                .file()
                .abs_diff(knight.file())
                .max(king.rank().abs_diff(knight.rank())) as usize;
            if distance >= 4 {
                eval += EVAL_PARAMS.knight_king_distance[distance - 4];
                self.trace
                    .term(|t| t.knight_king_distance[distance - 4][color] += 1);
            }

            // outposts
            if (knight.bitboard() & info.outposts[color]).is_not_empty() {
                let pawns = if W::WHITE {
                    self.game.white_pawns
                } else {
                    self.game.black_pawns
                };
                let defended =
                    (self.game.pawn_attack::<W::Other>(knight) & pawns).is_not_empty() as usize;
                eval += EVAL_PARAMS.knight_outpost[defended];
                self.trace.term(|t| t.knight_outposts[defended][color] += 1);
            }

            // mobility
            let attacks = lookup_knight(knight);
            let mobility = (attacks & info.mobility_area[color]).count_ones() as usize;
            eval += EVAL_PARAMS.knight_mobility[mobility];
            self.trace.term(|t| t.knight_mobility[mobility][color] += 1);
        }
        eval
    }

    #[inline]
    pub fn evaluate_bishops<W: TypeColor>(&mut self, info: &EvalInfo) -> EvalScore {
        let mut eval = EvalScore::zero();

        let bishops = if W::WHITE {
            self.game.white_bishops
        } else {
            self.game.black_bishops
        };

        let color = W::INDEX;

        // material value
        let count = bishops.count_ones() as i16;
        eval += EVAL_PARAMS.piece_values[Bishop] * count;
        self.trace.term(|t| t.bishop_count[color] = count);

        // bishops behind pawns
        let bishops_behind_pawns = (bishops & info.behind_pawns[color]).count_ones() as i16;
        eval += EVAL_PARAMS.bishop_behind_pawn * bishops_behind_pawns;
        self.trace
            .term(|t| t.bishops_behind_pawns[color] = bishops_behind_pawns);

        // bishop pair
        if (bishops & LIGHT_SQUARES).count_ones() >= 1 && (bishops & DARK_SQUARES).count_ones() >= 1
        {
            eval += EVAL_PARAMS.bishop_pair;
            self.trace.term(|t| t.bishop_pair[color] += 1);
        }

        // long diagonals
        let bishop_long_diagonals = (bishops & LONG_DIAGONALS).count_ones() as i16;
        eval += EVAL_PARAMS.bishop_long_diagonal * bishop_long_diagonals;
        self.trace
            .term(|t| t.bishop_long_diagonals[color] = bishop_long_diagonals);

        for bishop in bishops {
            // placement
            let relative_bishop = relative_board_index::<W>(bishop);
            eval += EVAL_PARAMS.piece_tables[(Bishop, relative_bishop)];
            self.trace
                .term(|t| t.bishop_placement[relative_bishop][color] += 1);

            // king distance
            let king = info.king_square[color];
            let distance = king
                .file()
                .abs_diff(bishop.file())
                .max(king.rank().abs_diff(bishop.rank())) as usize;
            if distance >= 4 {
                eval += EVAL_PARAMS.bishop_king_distance[distance - 4];
                self.trace
                    .term(|t| t.bishop_king_distance[distance - 4][color] += 1);
            }

            // outposts
            if (bishop.bitboard() & info.outposts[color]).is_not_empty() {
                let pawns = if W::WHITE {
                    self.game.white_pawns
                } else {
                    self.game.black_pawns
                };
                let defended =
                    (self.game.pawn_attack::<W::Other>(bishop) & pawns).is_not_empty() as usize;
                eval += EVAL_PARAMS.bishop_outpost[defended];
                self.trace.term(|t| t.bishop_outposts[defended][color] += 1);
            }

            // mobility
            let attacks = lookup_bishop(bishop, self.game.occupied);
            let mobility = (attacks & info.mobility_area[color]).count_ones() as usize;
            eval += EVAL_PARAMS.bishop_mobility[mobility];
            self.trace.term(|t| t.bishop_mobility[mobility][color] += 1);
        }
        eval
    }

    #[inline]
    pub fn evaluate_rooks<W: TypeColor>(&mut self, info: &EvalInfo) -> EvalScore {
        let mut eval = EvalScore::zero();

        let rooks = if W::WHITE {
            self.game.white_rooks
        } else {
            self.game.black_rooks
        };

        let color = W::INDEX;

        // material value
        let count = rooks.count_ones() as i16;
        eval += EVAL_PARAMS.piece_values[Rook] * count;
        self.trace.term(|t| t.rook_count[color as usize] = count);

        // rooks on seventh
        let seventh = if W::WHITE { SEVENTH_RANK } else { SECOND_RANK };
        let rooks_on_seventh = (rooks & seventh).count_ones() as i16;
        eval += EVAL_PARAMS.rook_on_seventh * rooks_on_seventh;
        self.trace
            .term(|t| t.rooks_on_seventh[color as usize] = rooks_on_seventh);

        for rook in rooks {
            // placement
            let relative_rook = relative_board_index::<W>(rook);
            eval += EVAL_PARAMS.piece_tables[(Rook, relative_rook)];
            self.trace
                .term(|t| t.rook_placement[relative_rook][color] += 1);

            // open file
            let (pawns, enemy_pawns) = if W::WHITE {
                (self.game.white_pawns, self.game.black_pawns)
            } else {
                (self.game.black_pawns, self.game.white_pawns)
            };

            if (pawns & FILES[rook.file()]).is_empty() {
                let open = (enemy_pawns & FILES[rook.file()]).is_empty() as usize;
                eval += EVAL_PARAMS.rook_open_file[open];
                self.trace.term(|t| t.rook_open_files[open][color] += 1);
            }

            // mobility
            let attacks = lookup_rook(rook, self.game.occupied);
            let mobility = (attacks & info.mobility_area[color]).count_ones() as usize;
            eval += EVAL_PARAMS.rook_mobility[mobility];
            self.trace.term(|t| t.rook_mobility[mobility][color] += 1);

            // trapped by king
            if mobility <= 3 {
                let king_file = info.king_square[color].file();
                if (king_file < 5) == (rook.file() < king_file) {
                    let can_castle = self.game.castling_rights[color]
                        .iter()
                        .any(|&c| c.is_not_empty()) as usize;
                    eval += EVAL_PARAMS.rook_trapped[can_castle];
                    self.trace.term(|t| t.rook_trapped[can_castle][color] += 1)
                }
            }
        }
        eval
    }

    #[inline]
    pub fn evaluate_queens<W: TypeColor>(&mut self, info: &EvalInfo) -> EvalScore {
        let mut eval = EvalScore::zero();

        let queens = if W::WHITE {
            self.game.white_queens
        } else {
            self.game.black_queens
        };
        let color = W::INDEX;
        // material value
        let count = queens.count_ones() as i16;
        eval += EVAL_PARAMS.piece_values[Queen] * count;
        self.trace.term(|t| t.queen_count[color] = count);

        for queen in queens {
            // placement
            let relative_queen = relative_board_index::<W>(queen);
            eval += EVAL_PARAMS.piece_tables[(Queen, relative_queen)];
            self.trace
                .term(|t| t.queen_placement[relative_queen][color] += 1);

            // discovery risk
            if self.game.discovered_attacks::<W>(queen).is_not_empty() {
                eval += EVAL_PARAMS.queen_discovery_risk;
                self.trace.term(|t| t.queen_discovery_risks[color] += 1);
            }

            // mobility
            let attacks = lookup_queen(queen, self.game.occupied);
            let mobility = (attacks & info.mobility_area[color]).count_ones() as usize;
            eval += EVAL_PARAMS.queen_mobility[mobility];
            self.trace.term(|t| t.queen_mobility[mobility][color] += 1);
        }
        eval
    }

    #[inline]
    pub fn evaluate_king<W: TypeColor>(&mut self, info: &EvalInfo) -> EvalScore {
        let mut eval = EvalScore::zero();

        let color = W::INDEX;

        // placement
        let relative_king = relative_board_index::<W>(info.king_square[color]);
        eval += EVAL_PARAMS.piece_tables[(King, relative_king)];
        self.trace
            .term(|t| t.king_placement[relative_king][color] += 1);

        // pawn and minor piece defenders
        let minors = if W::WHITE {
            self.game.white_pawns | self.game.white_knights | self.game.white_bishops
        } else {
            self.game.black_pawns | self.game.black_knights | self.game.black_bishops
        };
        let defenders = (info.king_area[color] & minors).count_ones() as usize;
        eval+= EVAL_PARAMS.king_defenders[defenders];
        self.trace.term(|t| t.king_defenders[defenders][color] += 1);

        // (half-) open files
        let (pawns, enemy_pawns) = if W::WHITE {
            (self.game.white_pawns, self.game.black_pawns)
        } else {
            (self.game.black_pawns, self.game.white_pawns)
        };
        if (pawns & FILES[info.king_square[color].file()]).is_empty() {
            let open = (enemy_pawns & FILES[info.king_square[color].file()]).is_empty() as usize;
            eval+= EVAL_PARAMS.king_open_file[open];
            self.trace.term(|t| t.king_open_file[open][color] += 1);
        }

        // no enemy queen
        let enemy_queens = if W::WHITE {
            self.game.black_queens
        } else {
            self.game.white_queens
        };
        if enemy_queens.is_empty() {
            eval+= EVAL_PARAMS.no_enemy_queen;
            self.trace.term(|t| t.no_enemy_queen[color] += 1);
        }

        // mobility
        let attacks = lookup_king(info.king_square[color]);
        let mobility = (attacks & info.mobility_area[color]).count_ones() as usize;
        eval += EVAL_PARAMS.king_mobility[mobility];
        self.trace.term(|t| t.king_mobility[mobility][color] += 1);

        eval
    }

    #[inline]
    pub fn evaluate_pawns_only<W: TypeColor>(&mut self, info: &mut EvalInfo) -> EvalScore {
        let mut eval = EvalScore::zero();

        let pawns = if W::WHITE {
            self.game.white_pawns
        } else {
            self.game.black_pawns
        };

        let color = W::INDEX;

        // material value
        let count = pawns.count_ones() as i16;
        eval+= EVAL_PARAMS.piece_values[Pawn] * count;
        self.trace.term(|t| t.pawn_count[color] = count);

        for pawn in pawns.clone() {
            // placement
            let relative_pawn = relative_board_index::<W>(pawn);
            eval+= EVAL_PARAMS.piece_tables[(Pawn, relative_pawn)];
            self.trace
                .term(|t| t.pawn_placement[relative_pawn][color] += 1);

            let file = pawn.file();
            let relative_rank = relative_pawn.rank();
            let board = pawn.bitboard();
            let attacks = self.game.pawn_attack::<W>(pawn);
            let threats = attacks
                & if W::WHITE {
                    self.game.black_pawns
                } else {
                    self.game.white_pawns
                };
            let neighbors = pawns & adjacent_files(file);
            let supporters = self.game.pawn_attack::<W::Other>(pawn) & pawns;

            // passed pawns
            if (board & info.passed_pawns[color]).is_not_empty() {
                info.passed_pawns[color] |= board;
                eval+= EVAL_PARAMS.passed_pawn[file];
                self.trace.term(|t| t.passed_pawn[file][color] += 1);

                eval+= EVAL_PARAMS.passed_pawn_advanced[relative_rank - 1];
                self.trace
                    .term(|t| t.passed_pawn_advanced[relative_rank - 1][color] += 1);

                if supporters.is_not_empty() {
                    eval+= EVAL_PARAMS.passed_pawn_connected;
                    self.trace.term(|t| t.passed_pawn_connected[color] += 1);
                }
            }

            // connected pawns
            if supporters.is_not_empty() {
                eval+= EVAL_PARAMS.connected_pawn[file];
                self.trace.term(|t| t.connected_pawn[file][color] += 1);
            }

            // double pawns
            let behind = if W::WHITE { board >> 8 } else { board << 8 };
            if supporters.is_empty() && (pawns & behind).is_not_empty() {
                eval+= EVAL_PARAMS.double_pawn[file];
                self.trace.term(|t| t.double_pawn[file][color] += 1);
            }

            // isolated pawns
            if threats.is_empty() && neighbors.is_empty() {
                eval+= EVAL_PARAMS.isolated_pawn[pawn.file()];
                self.trace
                    .term(|t| t.isolated_pawn[pawn.file()][color] += 1);
            }
        }

        eval
    }

    pub fn evaluate_passed_pawn_extras<W: TypeColor>(&mut self, info: &EvalInfo) -> EvalScore {
        let mut eval = EvalScore::zero();
        let color = W::INDEX;

        for pawn in info.passed_pawns[color] {
            let relative_rank = relative_board_index::<W>(pawn).rank();
            let file = pawn.file();
            let front_span = Board::pawn_push_span::<W>(pawn);
            let rear_span = Board::pawn_push_span::<W::Other>(pawn);

            let unblocked =
                (self.game.forward::<W>(pawn.bitboard()) & self.game.occupied).is_empty();
            if unblocked {
                eval+= EVAL_PARAMS.passed_pawn_unblocked;
                self.trace.term(|t| t.passed_pawn_unblocked[color] += 1);
            }
            let rooks = if W::WHITE {
                self.game.white_rooks
            } else {
                self.game.black_rooks
            };

            if (rear_span & rooks).is_not_empty() {
                eval+= EVAL_PARAMS.passed_pawn_friendly_rook;
                self.trace.term(|t| t.passed_pawn_friendly_rook[color] += 1);
            }

            let other_color = !(color != 0) as usize;
            let king_file_distance = info.king_square[other_color].file().abs_diff(file);
            let enemy_king_relative_rank =
                relative_board_index::<W>(info.king_square[other_color]).rank();
            if enemy_king_relative_rank < relative_rank
                || king_file_distance > front_span.count_ones() as usize
            {
                eval+= EVAL_PARAMS.passed_pawn_enemy_king_too_far;
                self.trace
                    .term(|t| t.passed_pawn_enemy_king_too_far[color] += 1);
            }
        }

        eval
    }
}

impl Board {
    #[inline]
    pub fn mobility_area<W: TypeColor>(&self) -> BitBoard {
        let (blocked_pawns, king) = if W::WHITE {
            (self.white_pawns & (self.black_pawns >> 8), self.white_king)
        } else {
            (self.black_pawns & (self.white_pawns << 8), self.black_king)
        };

        // exclude squares attacked by enemy pawns, our blocked pawns and our king
        (self.pawn_attacks::<W::Other>() | blocked_pawns | king).inverse()
    }

    #[inline]
    pub fn evaluate(&self, pawn_hash_table: &mut PawnHashTable) -> i16 {
        self.evaluate_impl::<()>(pawn_hash_table).0
    }

    #[inline]
    pub fn evaluate_impl<T: TraceTarget + Default>(
        &self,
        pawn_hash_table: &mut PawnHashTable,
    ) -> (i16, T) {
        let mut trace = T::default();
        let mut eval = EvalContext {
            game: self,
            pawn_hash_table,
            trace: &mut trace,
        };
        let score = if self.black_to_move {
            eval.evaluate::<Black>()
        } else {
            eval.evaluate::<White>()
        };

        (score, trace)
    }

    #[inline]
    pub fn game_phase(&self) -> i32 {
        let knight_phase = 1;
        let bishop_phase = 1;
        let rook_phase = 2;
        let queen_phase = 4;

        let total_phase = knight_phase * 4 + bishop_phase * 4 + rook_phase * 4 + queen_phase * 2;

        let mut phase: i32 = 0;

        phase += (self.white_knights | self.black_knights).count_ones() as i32 * knight_phase;
        phase += (self.white_bishops | self.black_bishops).count_ones() as i32 * bishop_phase;
        phase += (self.white_rooks | self.black_rooks).count_ones() as i32 * rook_phase;
        phase += (self.white_queens | self.black_queens).count_ones() as i32 * queen_phase;

        (256 * (total_phase - phase)) / total_phase
    }
}

#[inline(always)]
pub fn relative_board_index<W: TypeColor>(i: Square) -> Square {
    if W::WHITE {
        i
    } else {
        (*i ^ 56).into()
    }
}
