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
            king_area: [white_king_attacks, black_king_attacks],
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

        for knight in knights {
            let relative_knight = relative_board_index::<W>(knight);
            // placement
            eval += EVAL_PARAMS.piece_tables[(Knight, relative_knight)];
            self.trace
                .term(|t| t.knight_placement[relative_knight][color] += 1);

            // mobility
            let mobility =
                (lookup_knight(knight) & info.mobility_area[color]).count_ones() as usize;
            eval += EVAL_PARAMS.knight_mobility[mobility];
            self.trace.term(|t| t.knight_mobility[mobility][color] += 1);

            // outposts
            let pawns = if W::WHITE {
                self.game.white_pawns
            } else {
                self.game.black_pawns
            };
            let outpost =
                (self.game.pawn_attack_spans::<W::Other>() & knight.bitboard()).is_empty() as usize;
            let defended =
                (self.game.pawn_attack::<W::Other>(knight) & pawns).is_not_empty() as usize;
            // normal - 0, outpost - 1, defended outpost - 2
            let outpost_score = outpost + defended * outpost;
            eval += EVAL_PARAMS.knight_outpost[outpost_score];
            self.trace
                .term(|t| t.knight_outpost[outpost_score][color] += 1);
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

        for bishop in bishops {
            // placement
            let relative_bishop = relative_board_index::<W>(bishop);
            eval += EVAL_PARAMS.piece_tables[(Bishop, relative_bishop)];
            self.trace
                .term(|t| t.bishop_placement[relative_bishop][color] += 1);

            // mobility
            let mobility = (lookup_bishop(bishop, self.game.occupied) & info.mobility_area[color])
                .count_ones() as usize;
            eval += EVAL_PARAMS.bishop_mobility[mobility];
            self.trace.term(|t| t.bishop_mobility[mobility][color] += 1);

            // outposts
            let pawns = if W::WHITE {
                self.game.white_pawns
            } else {
                self.game.black_pawns
            };
            let outpost =
                (self.game.pawn_attack_spans::<W::Other>() & bishop.bitboard()).is_empty() as usize;
            let defended =
                (self.game.pawn_attack::<W::Other>(bishop) & pawns).is_not_empty() as usize;
            // normal - 0, outpost - 1, defended outpost - 2
            let outpost_score = outpost + defended * outpost;
            eval += EVAL_PARAMS.bishop_outpost[outpost_score];
            self.trace
                .term(|t| t.bishop_outpost[outpost_score][color] += 1);
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

        for rook in rooks {
            // placement
            let relative_rook = relative_board_index::<W>(rook);
            eval += EVAL_PARAMS.piece_tables[(Rook, relative_rook)];
            self.trace
                .term(|t| t.rook_placement[relative_rook][color] += 1);

            // mobility
            let mobility = (lookup_rook(rook, self.game.occupied) & info.mobility_area[color])
                .count_ones() as usize;
            eval += EVAL_PARAMS.rook_mobility[mobility];
            self.trace.term(|t| t.rook_mobility[mobility][color] += 1);

            // open files
            let (friendly_pawns, enemy_pawns) = if W::WHITE {
                (self.game.white_pawns, self.game.black_pawns)
            } else {
                (self.game.black_pawns, self.game.white_pawns)
            };

            let semi_open = (FILES[rook.file()] & friendly_pawns).is_empty() as usize;
            let open = (FILES[rook.file()] & enemy_pawns).is_empty() as usize;
            // normal - 0, semi-open - 1, open - 2
            let open_score = semi_open + semi_open * open;
            eval += EVAL_PARAMS.rook_on_open_file[open_score];
            self.trace
                .term(|t| t.rook_on_open_file[open_score][color] += 1);
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

            // mobility
            let mobility = (lookup_queen(queen, self.game.occupied) & info.mobility_area[color])
                .count_ones() as usize;
            eval += EVAL_PARAMS.queen_mobility[mobility];
            self.trace.term(|t| t.queen_mobility[mobility][color] += 1);

            // discovery risk
            let discoveries = self.game.discovered_attacks::<W>(queen).is_not_empty() as i16;
            eval += EVAL_PARAMS.queen_discovery_risk * discoveries;
            self.trace
                .term(|t| t.queen_discovery_risk[color] += discoveries);
        }
        eval
    }

    #[inline]
    pub fn evaluate_king<W: TypeColor>(&mut self, info: &EvalInfo) -> EvalScore {
        let mut eval = EvalScore::zero();

        let color = W::INDEX;
        let king = info.king_square[color];

        // placement
        let relative_king = relative_board_index::<W>(info.king_square[color]);
        eval += EVAL_PARAMS.piece_tables[(King, relative_king)];
        self.trace
            .term(|t| t.king_placement[relative_king][color] += 1);

        // open files
        let (pawns, enemy_pawns) = if W::WHITE {
            (self.game.white_pawns, self.game.black_pawns)
        } else {
            (self.game.black_pawns, self.game.white_pawns)
        };
        let semi_open = (FILES[king.file()] & pawns).is_empty() as usize;
        let open = (FILES[king.file()] & enemy_pawns).is_empty() as usize;

        eval += EVAL_PARAMS.king_on_open_file[semi_open + semi_open * open];
        self.trace
            .term(|t| t.king_on_open_file[semi_open + semi_open * open][color] += 1);

        // king ring attacks
        let king_ring_attacks = ((self.game.knight_attacks::<W::Other>() & info.king_area[color])
            .count_ones()
            + (self.game.diagonal_attacks::<W::Other>(self.game.occupied) & info.king_area[color])
                .count_ones()
            + (self.game.orthogonal_attacks::<W::Other>(self.game.occupied)
                & info.king_area[color])
                .count_ones())
        .min(15) as usize;
        eval += EVAL_PARAMS.king_ring_attacks[king_ring_attacks];
        self.trace
            .term(|t| t.king_ring_attacks[king_ring_attacks][color] += 1);

        // king virtual mobility
        let mobility = (lookup_queen(king, self.game.occupied) & info.mobility_area[color])
            .count_ones() as usize;
        eval += EVAL_PARAMS.king_virtual_mobility[mobility];
        self.trace
            .term(|t| t.king_virtual_mobility[mobility][color] += 1);

        // discovery risk
        let discoveries = self.game.discovered_attacks::<W>(king).is_not_empty() as i16;
        eval += EVAL_PARAMS.king_discovery_risk * discoveries;
        self.trace
            .term(|t| t.king_discovery_risk[color] += discoveries);

        eval
    }

    #[inline]
    pub fn evaluate_pawns_only<W: TypeColor>(&mut self, _info: &mut EvalInfo) -> EvalScore {
        let mut eval = EvalScore::zero();

        let (pawns, other_pawns) = if W::WHITE {
            (self.game.white_pawns, self.game.black_pawns)
        } else {
            (self.game.black_pawns, self.game.white_pawns)
        };

        let color = W::INDEX;

        // material value
        let count = pawns.count_ones() as i16;
        eval += EVAL_PARAMS.piece_values[Pawn] * count;
        self.trace.term(|t| t.pawn_count[color] = count);

        // doubled pawns per-file
        for file in FILES {
            let file_double_pawn_count = (pawns & file).count_ones().saturating_sub(1) as usize;
            eval += EVAL_PARAMS.pawn_doubled[file_double_pawn_count];
            self.trace
                .term(|t| t.pawn_doubled[file_double_pawn_count][color] += 1);
        }

        for pawn in pawns.clone() {
            // placement
            let relative_pawn = relative_board_index::<W>(pawn);
            eval += EVAL_PARAMS.piece_tables[(Pawn, relative_pawn)];
            self.trace
                .term(|t| t.pawn_placement[relative_pawn][color] += 1);

            // connected
            let connected_pawns =
                (self.game.pawn_attack::<W::Other>(pawn) & pawns).count_ones() as usize;
            eval += EVAL_PARAMS.pawn_connected[connected_pawns];
            self.trace
                .term(|t| t.pawn_connected[connected_pawns][color] += 1);

            // phalanx
            let phalanx_pawns = ((pawn.bitboard()
                | ((pawn.bitboard() & NOT_H_FILE) << 1)
                | ((pawn.bitboard() & NOT_A_FILE) >> 1))
                & pawns)
                .count_ones() as usize
                - 1; // the pawn in question will always be included
            eval += EVAL_PARAMS.pawn_phalanx[phalanx_pawns];
            self.trace
                .term(|t| t.pawn_phalanx[phalanx_pawns][color] += 1);

            // isolated
            let pawn_isolated = (pawns & adjacent_files(pawn.file())).is_empty() as usize;
            eval += EVAL_PARAMS.pawn_isolated[pawn_isolated];
            self.trace
                .term(|t| t.pawn_isolated[pawn_isolated][color] += 1);

            // backward
            let gatekeeper = if W::WHITE {
                self.game.pawn_attack::<W>(pawn) << 8
            } else {
                self.game.pawn_attack::<W>(pawn) >> 8
            } & other_pawns;
            let backward = (self.game.pawn_adjacent_rear_span::<W>(pawn) & pawns).is_empty()
                && gatekeeper.is_not_empty();
            eval += EVAL_PARAMS.pawn_backward[backward as usize];
            self.trace
                .term(|t| t.pawn_backward[backward as usize][color] += 1)
        }

        eval
    }

    pub fn evaluate_passed_pawn_extras<W: TypeColor>(&mut self, info: &EvalInfo) -> EvalScore {
        let mut eval = EvalScore::zero();

        let color = W::INDEX;

        let passers = info.passed_pawns[color];

        for passer in passers {
            // placement
            let relative_passer = relative_board_index::<W>(passer);
            eval += EVAL_PARAMS.passed_pawn_table[relative_passer];
            self.trace
                .term(|t| t.passed_pawn_placement[relative_passer][color] += 1);

            // friendly king distance
            let king = info.king_square[color];
            let friendly_distance = passer
                .rank()
                .abs_diff(king.rank())
                .max(passer.file().abs_diff(king.file()))
                .min(4)
                - 1;
            eval += EVAL_PARAMS.passed_pawn_friendly_king_distance[friendly_distance];
            self.trace
                .term(|t| t.passed_pawn_friendly_king_distance[friendly_distance][color] += 1);

            let other_king = info.king_square[W::Other::INDEX];
            let enemy_distance = passer
                .rank()
                .abs_diff(other_king.rank())
                .max(passer.file().abs_diff(other_king.file()))
                .min(4)
                - 1;
            eval += EVAL_PARAMS.passed_pawn_enemy_king_distance[enemy_distance];
            self.trace
                .term(|t| t.passed_pawn_enemy_king_distance[enemy_distance][color] += 1);
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

        (256 * (total_phase.saturating_sub(phase))) / total_phase
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
