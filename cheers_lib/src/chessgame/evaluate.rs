use crate::hash_tables::PawnHashTable;

pub use self::eval_params::*;
use self::GamePhase::*;

use super::{eval_types::*, *};

pub struct EvalContext<'search, T> {
    game: &'search ChessGame,
    pawn_hash_table: &'search mut PawnHashTable,
    trace: &'search mut T,
    params: &'search EvalParams,
}

impl<'search, T: TraceTarget + Default> EvalContext<'search, T> {
    #[inline]
    pub fn evaluate(&mut self) -> i32 {
        let mut eval = EvalScore::zero();

        let color = self.game.current_player();

        self.trace.term(|t| t.turn = color as i32);

        let phase = self.game.game_phase();

        let white_king_square = self.game.piece_masks()[(White, King)].first_square();
        let black_king_square = self.game.piece_masks()[(Black, King)].first_square();

        let white_king_attacks = lookup_king(white_king_square);
        let black_king_attacks = lookup_king(black_king_square);

        let front_spans_black = self
            .game
            .pawn_front_spans(Black, self.game.piece_masks()[(Black, Pawn)]);
        let all_front_spans_black = front_spans_black
            | (front_spans_black & NOT_H_FILE) << 1
            | (front_spans_black & NOT_A_FILE) >> 1;
        let rear_spans_black = self
            .game
            .pawn_push_spans(self.game.piece_masks()[(Black, Pawn)], White);

        let front_spans_white = self
            .game
            .pawn_front_spans(White, self.game.piece_masks()[(White, Pawn)]);

        let all_front_spans_white = front_spans_white
            | (front_spans_white & NOT_H_FILE) << 1
            | (front_spans_white & NOT_A_FILE) >> 1;
        let rear_spans_white = self
            .game
            .pawn_push_spans(self.game.piece_masks()[(White, Pawn)], Black);

        let white_passers = self.game.piece_masks()[(White, Pawn)]
            & all_front_spans_black.inverse()
            & rear_spans_white.inverse();
        let black_passers = self.game.piece_masks()[(Black, Pawn)]
            & all_front_spans_white.inverse()
            & rear_spans_black.inverse();

        // initialise eval info
        let mut info = EvalInfo {
            mobility_area: [
                self.game.mobility_area(White),
                self.game.mobility_area(Black),
            ],
            behind_pawns: [
                self.game.piece_masks()[(White, Pawn)] >> 8,
                self.game.piece_masks()[(Black, Pawn)] << 8,
            ],
            outposts: [
                self.game.pawn_attack_spans(Black).inverse(),
                self.game.pawn_attacks(White).inverse(),
            ],
            seventh_rank: [SEVENTH_RANK, SECOND_RANK],
            king_square: [white_king_square, black_king_square],
            king_area: [
                white_king_attacks | (white_king_attacks << 8),
                black_king_attacks | (black_king_attacks >> 8),
            ],
            passed_pawns: [white_passers, black_passers],
        };

        if !T::TRACING {
            match self
                .pawn_hash_table
                .get(self.game.zobrist_pawn_hash(), color)
            {
                None => {
                    let score = self.evaluate_pawns_only(color, &mut info, self.params)
                        - self.evaluate_pawns_only(!color, &mut info, self.params);
                    self.pawn_hash_table.set(
                        self.game.zobrist_pawn_hash(),
                        score.mg,
                        score.eg,
                        color,
                    );
                    eval += score;
                }
                Some(val) => {
                    // let score = self.evaluate_pawns_only(color, &mut info, self.params)
                    //     - self.evaluate_pawns_only(!color, &mut info, self.params);
                    // if val != score {
                    //     println!("{}", self.game.fen());
                    //     println!("{val:?}");
                    //     println!("{score:?}");
                    //     println!(
                    //         "{} => {}",
                    //         self.game.zobrist_pawn_hash(),
                    //         self.game.zobrist_pawn_hash() & (65536 - 1)
                    //     )
                    // }
                    eval += val;
                }
            }
        } else {
            eval += self.evaluate_pawns_only(color, &mut info, self.params)
                - self.evaluate_pawns_only(!color, &mut info, self.params);
        }

        eval += self.evaluate_passed_pawn_extras(color, &info, self.params)
            - self.evaluate_passed_pawn_extras(!color, &info, self.params);

        eval += self.evaluate_knights(color, &info, self.params)
            - self.evaluate_knights(!color, &info, self.params);

        eval += self.evaluate_bishops(color, &info, self.params)
            - self.evaluate_bishops(!color, &info, self.params);

        eval += self.evaluate_rooks(color, &info, self.params)
            - self.evaluate_rooks(!color, &info, self.params);

        eval += self.evaluate_queens(color, &info, self.params)
            - self.evaluate_queens(!color, &info, self.params);

        eval += self.evaluate_king(color, &info, self.params)
            - self.evaluate_king(!color, &info, self.params);

        ((eval.mg * (256 - phase)) + (eval.eg * phase)) / 256
    }

    #[inline]
    pub fn evaluate_knights(
        &mut self,
        color: ColorIndex,
        info: &EvalInfo,
        params: &EvalParams,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        // material value
        let count = self.game.piece_masks()[(color, Knight)].count_ones() as i32;
        eval.mg += params.piece_values[(Midgame, Knight)] * count;
        eval.eg += params.piece_values[(Endgame, Knight)] * count;
        self.trace.term(|t| t.knight_count[color as usize] = count);

        // knights behind pawns
        let knights_behind_pawns = (self.game.piece_masks()[(color, Knight)]
            & info.behind_pawns[color as usize])
            .count_ones() as i32;
        eval.mg += params.knight_behind_pawn[Midgame] * knights_behind_pawns;
        eval.eg += params.knight_behind_pawn[Endgame] * knights_behind_pawns;
        self.trace
            .term(|t| t.knights_behind_pawns[color] = knights_behind_pawns);

        for knight in self.game.piece_masks()[(color, Knight)] {
            let relative_knight = relative_board_index(knight, color);
            // placement
            eval.mg += params.piece_tables[(Midgame, Knight, relative_knight)];
            eval.eg += params.piece_tables[(Endgame, Knight, relative_knight)];
            self.trace
                .term(|t| t.knight_placement[relative_knight][color] += 1);

            // king distance
            let king = info.king_square[color];
            let distance = king
                .file()
                .abs_diff(knight.file())
                .max(king.rank().abs_diff(knight.rank())) as usize;
            if distance >= 4 {
                eval.mg += params.knight_king_distance[distance - 4][Midgame];
                eval.eg += params.knight_king_distance[distance - 4][Endgame];
                self.trace
                    .term(|t| t.knight_king_distance[distance - 4][color] += 1);
            }

            // outposts
            if (knight.bitboard() & info.outposts[color]).is_not_empty() {
                let defended = (lookup_pawn_attack(knight, !color)
                    & self.game.piece_masks()[(color, Pawn)])
                    .is_not_empty() as usize;
                eval.mg += params.knight_outpost[defended][Midgame];
                eval.eg += params.knight_outpost[defended][Endgame];
                self.trace.term(|t| t.knight_outposts[defended][color] += 1);
            }

            // mobility
            let attacks = lookup_knight(knight);
            let mobility = (attacks & info.mobility_area[color]).count_ones() as usize;
            eval.mg += params.knight_mobility[mobility][Midgame];
            eval.eg += params.knight_mobility[mobility][Endgame];
            self.trace.term(|t| t.knight_mobility[mobility][color] += 1);
        }
        eval
    }

    #[inline]
    pub fn evaluate_bishops(
        &mut self,
        color: ColorIndex,
        info: &EvalInfo,
        params: &EvalParams,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        // material value
        let count = self.game.piece_masks()[(color, Bishop)].count_ones() as i32;
        eval.mg += params.piece_values[(Midgame, Bishop)] * count;
        eval.eg += params.piece_values[(Endgame, Bishop)] * count;
        self.trace.term(|t| t.bishop_count[color] = count);

        // bishops behind pawns
        let bishops_behind_pawns = (self.game.piece_masks()[(color, Bishop)]
            & info.behind_pawns[color])
            .count_ones() as i32;
        eval.mg += params.bishop_behind_pawn[Midgame] * bishops_behind_pawns;
        eval.eg += params.bishop_behind_pawn[Endgame] * bishops_behind_pawns;
        self.trace
            .term(|t| t.bishops_behind_pawns[color] = bishops_behind_pawns);

        // bishop pair
        if (self.game.piece_masks()[(color, Bishop)] & LIGHT_SQUARES).count_ones() >= 1
            && (self.game.piece_masks()[(color, Bishop)] & DARK_SQUARES).count_ones() >= 1
        {
            eval.mg += params.bishop_pair[Midgame];
            eval.eg += params.bishop_pair[Endgame];
            self.trace.term(|t| t.bishop_pair[color] += 1);
        }

        // long diagonals
        let bishop_long_diagonals =
            (self.game.piece_masks()[(color, Bishop)] & LONG_DIAGONALS).count_ones() as i32;
        eval.mg += params.bishop_long_diagonal[Midgame] * bishop_long_diagonals;
        eval.eg += params.bishop_long_diagonal[Endgame] * bishop_long_diagonals;
        self.trace
            .term(|t| t.bishop_long_diagonals[color] = bishop_long_diagonals);

        for bishop in self.game.piece_masks()[(color, Bishop)] {
            // placement
            let relative_bishop = relative_board_index(bishop, color);
            eval.mg += params.piece_tables[(Midgame, Bishop, relative_bishop)];
            eval.eg += params.piece_tables[(Endgame, Bishop, relative_bishop)];
            self.trace
                .term(|t| t.bishop_placement[relative_bishop][color] += 1);

            // king distance
            let king = info.king_square[color];
            let distance = king
                .file()
                .abs_diff(bishop.file())
                .max(king.rank().abs_diff(bishop.rank())) as usize;
            if distance >= 4 {
                eval.mg += params.bishop_king_distance[distance - 4][Midgame];
                eval.eg += params.bishop_king_distance[distance - 4][Endgame];
                self.trace
                    .term(|t| t.bishop_king_distance[distance - 4][color] += 1);
            }

            // outposts
            if (bishop.bitboard() & info.outposts[color]).is_not_empty() {
                let defended = (lookup_pawn_attack(bishop, !color)
                    & self.game.piece_masks()[(color, Pawn)])
                    .is_not_empty() as usize;
                eval.mg += params.bishop_outpost[defended][Midgame];
                eval.eg += params.bishop_outpost[defended][Endgame];
                self.trace.term(|t| t.bishop_outposts[defended][color] += 1);
            }

            // mobility
            let attacks = lookup_bishop(bishop, self.game.combined());
            let mobility = (attacks & info.mobility_area[color]).count_ones() as usize;
            eval.mg += params.bishop_mobility[mobility][Midgame];
            eval.eg += params.bishop_mobility[mobility][Endgame];
            self.trace.term(|t| t.bishop_mobility[mobility][color] += 1);
        }
        eval
    }

    #[inline]
    pub fn evaluate_rooks(
        &mut self,
        color: ColorIndex,
        info: &EvalInfo,
        params: &EvalParams,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        // material value
        let count = self.game.piece_masks()[(color, Rook)].count_ones() as i32;
        eval.mg += params.piece_values[(Midgame, Rook)] * count;
        eval.eg += params.piece_values[(Endgame, Rook)] * count;
        self.trace.term(|t| t.rook_count[color as usize] = count);

        // rooks on seventh
        let rooks_on_seventh = (self.game.piece_masks()[(color, Rook)]
            & info.seventh_rank[color as usize])
            .count_ones() as i32;
        eval.mg += params.rook_on_seventh[Midgame as usize] * rooks_on_seventh;
        eval.mg += params.rook_on_seventh[Endgame as usize] * rooks_on_seventh;
        self.trace
            .term(|t| t.rooks_on_seventh[color as usize] = rooks_on_seventh);

        for rook in self.game.piece_masks()[(color, Rook)] {
            // placement
            let relative_rook = relative_board_index(rook, color);
            eval.mg += params.piece_tables[(Midgame, Rook, relative_rook)];
            eval.eg += params.piece_tables[(Endgame, Rook, relative_rook)];
            self.trace
                .term(|t| t.rook_placement[relative_rook][color] += 1);

            // open files
            if (self.game.piece_masks()[(color, Pawn)] & FILES[rook.file()]).is_empty() {
                let open = (self.game.piece_masks()[(!color, Pawn)] & FILES[rook.file()]).is_empty()
                    as usize;
                eval.mg += params.rook_open_file[open][Midgame];
                eval.eg += params.rook_open_file[open][Endgame];
                self.trace.term(|t| t.rook_open_files[open][color] += 1);
            }

            // mobility
            let attacks = lookup_rook(rook, self.game.combined());
            let mobility = (attacks & info.mobility_area[color]).count_ones() as usize;
            eval.mg += params.rook_mobility[mobility][Midgame];
            eval.eg += params.rook_mobility[mobility][Endgame];
            self.trace.term(|t| t.rook_mobility[mobility][color] += 1);

            // trapped by king
            if mobility <= 3 {
                let king_file = self.game.piece_masks[(color, King)].first_square().file();
                if (king_file < 5) == (rook.file() < king_file) {
                    let can_castle = self.game.castling_rights.0[color].iter().any(|&c| c) as usize;
                    eval.mg += params.rook_trapped[can_castle][Midgame];
                    eval.eg += params.rook_trapped[can_castle][Endgame];
                    self.trace.term(|t| t.rook_trapped[can_castle][color] += 1)
                }
            }
        }
        eval
    }

    #[inline]
    pub fn evaluate_queens(
        &mut self,
        color: ColorIndex,
        info: &EvalInfo,
        params: &EvalParams,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        // material value
        let count = self.game.piece_masks()[(color, Queen)].count_ones() as i32;
        eval.mg += params.piece_values[(Midgame, Queen)] * count;
        eval.eg += params.piece_values[(Endgame, Queen)] * count;
        self.trace.term(|t| t.queen_count[color] = count);

        for queen in self.game.piece_masks()[(color, Queen)] {
            // placement
            let relative_queen = relative_board_index(queen, color);
            eval.mg += params.piece_tables[(Midgame, Queen, relative_queen)];
            eval.eg += params.piece_tables[(Endgame, Queen, relative_queen)];
            self.trace
                .term(|t| t.queen_placement[relative_queen][color] += 1);

            // discovery risk
            if self.game.discovered_attacks(queen, color).is_not_empty() {
                eval.mg += params.queen_discovery_risk[Midgame];
                eval.eg += params.queen_discovery_risk[Endgame];
                self.trace.term(|t| t.queen_discovery_risks[color] += 1);
            }

            // mobility
            let attacks = lookup_queen(queen, self.game.combined());
            let mobility = (attacks & info.mobility_area[color]).count_ones() as usize;
            eval.mg += params.queen_mobility[mobility][Midgame];
            eval.eg += params.queen_mobility[mobility][Endgame];
            self.trace.term(|t| t.queen_mobility[mobility][color] += 1);
        }
        eval
    }

    #[inline]
    pub fn evaluate_king(
        &mut self,
        color: ColorIndex,
        info: &EvalInfo,
        params: &EvalParams,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        // placement
        let relative_king = relative_board_index(info.king_square[color], color);
        eval.mg += params.piece_tables[(Midgame, King, relative_king)];
        eval.eg += params.piece_tables[(Endgame, King, relative_king)];
        self.trace
            .term(|t| t.king_placement[relative_king][color] += 1);

        // pawn and minor piece defenders
        let defenders = (info.king_area[color]
            & (self.game.piece_masks()[(color, Pawn)]
                | self.game.piece_masks()[(color, Knight)]
                | self.game.piece_masks()[(color, Bishop)]))
            .count_ones() as usize;
        eval.mg += params.king_defenders[defenders][Midgame];
        eval.eg += params.king_defenders[defenders][Endgame];
        self.trace.term(|t| t.king_defenders[defenders][color] += 1);

        // (half-) open files
        if (self.game.piece_masks[(color, Pawn)] & FILES[info.king_square[color].file()]).is_empty()
        {
            let open = (self.game.piece_masks()[(!color, Pawn)]
                & FILES[info.king_square[color].file()])
            .is_empty() as usize;
            eval.mg += params.king_open_file[open][Midgame];
            eval.eg += params.king_open_file[open][Endgame];
            self.trace.term(|t| t.king_open_file[open][color] += 1);
        }

        // no enemy queen
        if self.game.piece_masks[(!color, Queen)].is_empty() {
            eval.mg += params.no_enemy_queen[Midgame];
            eval.eg += params.no_enemy_queen[Endgame];
            self.trace.term(|t| t.no_enemy_queen[color] += 1);
        }

        // mobility
        let attacks = lookup_king(info.king_square[color]);
        let mobility = (attacks & info.mobility_area[color]).count_ones() as usize;
        eval.mg += params.king_mobility[mobility][Midgame];
        eval.eg += params.king_mobility[mobility][Endgame];
        self.trace.term(|t| t.king_mobility[mobility][color] += 1);

        eval
    }

    #[inline]
    pub fn evaluate_pawns_only(
        &mut self,
        color: ColorIndex,
        info: &mut EvalInfo,
        params: &EvalParams,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        // material value
        let count = self.game.piece_masks()[(color, Pawn)].count_ones() as i32;
        eval.mg += params.piece_values[(Midgame, Pawn)] * count;
        eval.eg += params.piece_values[(Endgame, Pawn)] * count;
        self.trace.term(|t| t.pawn_count[color] = count);

        let pawns = self.game.piece_masks()[(color, Pawn)];

        for pawn in self.game.piece_masks()[(color, Pawn)] {
            // placement
            let relative_pawn = relative_board_index(pawn, color);
            eval.mg += params.piece_tables[(Midgame, Pawn, relative_pawn)];
            eval.eg += params.piece_tables[(Endgame, Pawn, relative_pawn)];
            self.trace
                .term(|t| t.pawn_placement[relative_pawn][color] += 1);

            let file = pawn.file();
            let relative_rank = relative_pawn.rank();
            let board = pawn.bitboard();
            let attacks = lookup_pawn_attack(pawn, color);
            let threats = attacks & self.game.piece_masks()[(!color, Pawn)];
            let neighbors = pawns & adjacent_files(file);
            let supporters = lookup_pawn_attack(pawn, !color) & pawns;

            // passed pawns
            if (board & info.passed_pawns[color]).is_not_empty() {
                info.passed_pawns[color] |= board;
                eval.mg += params.passed_pawn[file][Midgame];
                eval.eg += params.passed_pawn[file][Endgame];
                self.trace.term(|t| t.passed_pawn[file][color] += 1);

                eval.mg += params.passed_pawn_advanced[relative_rank - 1][Midgame];
                eval.eg += params.passed_pawn_advanced[relative_rank - 1][Endgame];
                self.trace
                    .term(|t| t.passed_pawn_advanced[relative_rank - 1][color] += 1);

                if supporters.is_not_empty() {
                    eval.mg += params.passed_pawn_connected[Midgame];
                    eval.eg += params.passed_pawn_connected[Endgame];
                    self.trace.term(|t| t.passed_pawn_connected[color] += 1);
                }
            }

            // connected pawns
            if supporters.is_not_empty() {
                eval.mg += params.connected_pawn[file][Midgame];
                eval.eg += params.connected_pawn[file][Endgame];
                self.trace.term(|t| t.connected_pawn[file][color] += 1);
            }

            // double pawns
            let behind = board >> 8 << 16 * color as u8;
            if supporters.is_empty()
                && (self.game.piece_masks[(color, Pawn)] & behind).is_not_empty()
            {
                eval.mg += params.double_pawn[file][Midgame];
                eval.eg += params.double_pawn[file][Endgame];
                self.trace.term(|t| t.double_pawn[file][color] += 1);
            }

            // isolated pawns
            if threats.is_empty() && neighbors.is_empty() {
                eval.mg += params.isolated_pawn[pawn.file()][Midgame];
                eval.eg += params.isolated_pawn[pawn.file()][Endgame];
                self.trace
                    .term(|t| t.isolated_pawn[pawn.file()][color] += 1);
            }
        }

        eval
    }

    pub fn evaluate_passed_pawn_extras(
        &mut self,
        color: ColorIndex,
        info: &EvalInfo,
        params: &EvalParams,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        for pawn in info.passed_pawns[color] {
            let relative_rank = relative_board_index(pawn, color).rank();
            let file = pawn.file();
            let front_span = self.game.pawn_push_span(pawn, color);
            let rear_span = self.game.pawn_push_span(pawn, !color);

            let unblocked = (lookup_pawn_push(pawn, color) & self.game.combined()).is_empty();
            if unblocked {
                eval.mg += params.passed_pawn_unblocked[Midgame];
                eval.eg += params.passed_pawn_unblocked[Endgame];
                self.trace.term(|t| t.passed_pawn_unblocked[color] += 1);
            }

            if (rear_span & self.game.piece_masks()[(color, Rook)]).is_not_empty() {
                eval.mg += params.passed_pawn_friendly_rook[Midgame];
                eval.eg += params.passed_pawn_friendly_rook[Endgame];
                self.trace.term(|t| t.passed_pawn_friendly_rook[color] += 1);
            }

            let king_file_distance = info.king_square[!color].file().abs_diff(file);
            let enemy_king_relative_rank =
                relative_board_index(info.king_square[!color], color).rank();
            if enemy_king_relative_rank < relative_rank
                || king_file_distance > front_span.count_ones() as usize
            {
                eval.mg += params.passed_pawn_enemy_king_too_far[Midgame];
                eval.eg += params.passed_pawn_enemy_king_too_far[Endgame];
                self.trace
                    .term(|t| t.passed_pawn_enemy_king_too_far[color] += 1);
            }
        }

        eval
    }
}

impl ChessGame {
    #[inline]
    pub fn mobility_area(&self, color: ColorIndex) -> BitBoard {
        let blocked_pawns = match color {
            White => self.piece_masks[(White, Pawn)] & (self.piece_masks[(Black, Pawn)] >> 8),
            Black => self.piece_masks[(Black, Pawn)] & (self.piece_masks[(White, Pawn)] << 8),
        };

        // exclude squares attacked by enemy pawns, our blocked pawns and our king
        (self.pawn_attacks(!color) | blocked_pawns | self.piece_masks[(color, King)]).inverse()
    }

    #[inline]
    pub fn evaluate(&self, pawn_hash_table: &mut PawnHashTable) -> i32 {
        self.evaluate_impl::<()>(pawn_hash_table).0
    }

    #[inline]
    pub fn evaluate_impl<T: TraceTarget + Default>(
        &self,
        pawn_hash_table: &mut PawnHashTable,
    ) -> (i32, T) {
        let mut trace = T::default();
        let mut eval = EvalContext {
            game: self,
            pawn_hash_table,
            trace: &mut trace,
            params: &EVAL_PARAMS,
        };
        let score = eval.evaluate();
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

        phase += (self.piece_masks[(White, Knight)] | self.piece_masks[(Black, Knight)])
            .count_ones() as i32
            * knight_phase;
        phase += (self.piece_masks[(White, Bishop)] | self.piece_masks[(Black, Bishop)])
            .count_ones() as i32
            * bishop_phase;
        phase += (self.piece_masks[(White, Rook)] | self.piece_masks[(Black, Rook)]).count_ones()
            as i32
            * rook_phase;
        phase += (self.piece_masks[(White, Queen)] | self.piece_masks[(Black, Queen)]).count_ones()
            as i32
            * queen_phase;

        (256 * (total_phase - phase)) / total_phase
    }
}

#[inline]
pub fn relative_board_index(i: Square, color: ColorIndex) -> Square {
    (*i as usize ^ (56 * color as usize)).into()
}
