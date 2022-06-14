use std::ops::{Add, AddAssign, Sub};

use crate::bitboard::relative_board_index;

pub use self::eval_params::*;
use self::GamePhase::*;

use super::*;

pub struct EvalInfo {
    mobility_area: [BitBoard; 2],
}

#[derive(Copy, Clone)]
pub struct EvalScore {
    pub mg: i32,
    pub eg: i32,
}

impl EvalScore {
    pub fn zero() -> Self {
        Self { mg: 0, eg: 0 }
    }
}

impl Add<EvalScore> for EvalScore {
    type Output = Self;

    fn add(self, rhs: EvalScore) -> Self::Output {
        Self {
            mg: self.mg + rhs.mg,
            eg: self.eg + rhs.eg,
        }
    }
}

impl AddAssign<EvalScore> for EvalScore {
    fn add_assign(&mut self, rhs: EvalScore) {
        self.mg += rhs.mg;
        self.eg += rhs.eg;
    }
}

impl Sub<EvalScore> for EvalScore {
    type Output = Self;

    fn sub(self, rhs: EvalScore) -> Self::Output {
        Self {
            mg: self.mg - rhs.mg,
            eg: self.eg - rhs.eg,
        }
    }
}

pub trait TracingType {
    const TRACING: bool;
}
pub struct Tracing;
pub struct NoTracing;

impl TracingType for Tracing {
    const TRACING: bool = true;
}
impl TracingType for NoTracing {
    const TRACING: bool = false;
}

impl ChessGame {
    pub fn mobility_area(&self, color: ColorIndex) -> BitBoard {
        let blocked_pawns = match color {
            White => self.piece_masks[(White, Pawn)] & (self.piece_masks[(Black, Pawn)] >> 8),
            Black => self.piece_masks[(Black, Pawn)] & (self.piece_masks[(White, Pawn)] << 8),
        };

        // exclude squares attacked by enemy pawns, our blocked pawns and our king
        (self.pawn_attacks(!color) | blocked_pawns | self.piece_masks[(color, King)]).inverse()
    }

    pub fn evaluate(&self) -> i32 {
        self._evaluate::<NoTracing>(&EVAL_PARAMS).0
    }

    pub fn _evaluate<T: TracingType>(&self, params: &EvalParams) -> (i32, EvalTrace) {
        let mut eval = EvalScore::zero();
        let mut trace = EvalTrace::new();

        if T::TRACING {
            trace.turn = self.current_player as i32;
        }

        let phase = self.game_phase();

        // initialise eval info
        let info = EvalInfo {
            mobility_area: [self.mobility_area(White), self.mobility_area(Black)],
        };

        eval += self.evaluate_knights::<T>(self.current_player, &info, params, &mut trace)
            - self.evaluate_knights::<T>(!self.current_player, &info, params, &mut trace);

        eval += self.evaluate_bishops::<T>(self.current_player, &info, params, &mut trace)
            - self.evaluate_bishops::<T>(!self.current_player, &info, params, &mut trace);

        eval += self.evaluate_rooks::<T>(self.current_player, &info, params, &mut trace)
            - self.evaluate_rooks::<T>(!self.current_player, &info, params, &mut trace);

        eval += self.evaluate_queens::<T>(self.current_player, &info, params, &mut trace)
            - self.evaluate_queens::<T>(!self.current_player, &info, params, &mut trace);

        eval += self.evaluate_pawns::<T>(self.current_player, &info, params, &mut trace)
            - self.evaluate_pawns::<T>(!self.current_player, &info, params, &mut trace);

        eval += self.evaluate_king::<T>(self.current_player, &info, params, &mut trace)
            - self.evaluate_king::<T>(!self.current_player, &info, params, &mut trace);

        let score = ((eval.mg * (256 - phase)) + (eval.eg * phase)) / 256;
        (score, trace)
    }

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

    pub fn evaluate_knights<T: TracingType>(
        &self,
        color: ColorIndex,
        info: &EvalInfo,
        params: &EvalParams,
        trace: &mut EvalTrace,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        // material value
        let count = self.piece_masks[(color, Knight)].count_ones() as i32;
        eval.mg += params.piece_values[(Midgame, Knight)] * count;
        eval.eg += params.piece_values[(Endgame, Knight)] * count;
        if T::TRACING {
            trace.knight_count[color as usize] = count;
        }

        for knight in self.piece_masks[(color, Knight)] {
            let knight = relative_board_index(knight, color);
            // placement
            eval.mg += params.piece_tables[(Midgame, Knight, knight as u8)];
            eval.eg += params.piece_tables[(Endgame, Knight, knight as u8)];

            if T::TRACING {
                trace.knight_placement[knight as usize][color as usize] += 1;
            }

            // mobility
            let attacks = lookup_knight(knight.into());
            let mobility = (attacks & info.mobility_area[color as usize]).count_ones() as usize;
            eval.mg += params.knight_mobility[mobility][Midgame as usize];
            eval.eg += params.knight_mobility[mobility][Endgame as usize];
            if T::TRACING {
                trace.knight_mobility[mobility][color as usize] += 1;
            }
        }
        eval
    }

    pub fn evaluate_bishops<T: TracingType>(
        &self,
        color: ColorIndex,
        info: &EvalInfo,
        params: &EvalParams,
        trace: &mut EvalTrace,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        // material value
        let count = self.piece_masks[(color, Bishop)].count_ones() as i32;
        eval.mg += params.piece_values[(Midgame, Bishop)] * count;
        eval.eg += params.piece_values[(Endgame, Bishop)] * count;

        if T::TRACING {
            trace.bishop_count[color as usize] = count;
        }

        for bishop in self.piece_masks[(color, Bishop)] {
            // placement
            let bishop = relative_board_index(bishop, color);
            eval.mg += params.piece_tables[(Midgame, Bishop, bishop as u8)];
            eval.eg += params.piece_tables[(Endgame, Bishop, bishop as u8)];

            if T::TRACING {
                trace.bishop_placement[bishop as usize][color as usize] += 1;
            }

            // mobility
            let attacks = lookup_bishop(bishop as usize, self.combined);
            let mobility = (attacks & info.mobility_area[color as usize]).count_ones() as usize;
            eval.mg += params.bishop_mobility[mobility][Midgame as usize];
            eval.eg += params.bishop_mobility[mobility][Endgame as usize];
            if T::TRACING {
                trace.bishop_mobility[mobility][color as usize] += 1;
            }
        }
        eval
    }

    pub fn evaluate_rooks<T: TracingType>(
        &self,
        color: ColorIndex,
        info: &EvalInfo,
        params: &EvalParams,
        trace: &mut EvalTrace,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        // material value
        let count = self.piece_masks[(color, Rook)].count_ones() as i32;
        eval.mg += params.piece_values[(Midgame, Rook)] * count;
        eval.eg += params.piece_values[(Endgame, Rook)] * count;

        if T::TRACING {
            trace.rook_count[color as usize] = count;
        }

        for rook in self.piece_masks[(color, Rook)] {
            // placement
            let rook = relative_board_index(rook, color);
            eval.mg += params.piece_tables[(Midgame, Rook, rook as u8)];
            eval.eg += params.piece_tables[(Endgame, Rook, rook as u8)];

            if T::TRACING {
                trace.rook_placement[rook as usize][color as usize] += 1;
            }

            // mobility
            let attacks = lookup_rook(rook as usize, self.combined);
            let mobility = (attacks & info.mobility_area[color as usize]).count_ones() as usize;
            eval.mg += params.rook_mobility[mobility][Midgame as usize];
            eval.eg += params.rook_mobility[mobility][Endgame as usize];
            if T::TRACING {
                trace.rook_mobility[mobility][color as usize] += 1;
            }
        }
        eval
    }

    pub fn evaluate_queens<T: TracingType>(
        &self,
        color: ColorIndex,
        info: &EvalInfo,
        params: &EvalParams,
        trace: &mut EvalTrace,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        // material value
        let count = self.piece_masks[(color, Queen)].count_ones() as i32;
        eval.mg += params.piece_values[(Midgame, Queen)] * count;
        eval.eg += params.piece_values[(Endgame, Queen)] * count;
        if T::TRACING {
            trace.queen_count[color as usize] = count;
        }

        for queen in self.piece_masks[(color, Queen)] {
            // placement
            let queen = relative_board_index(queen, color);
            eval.mg += params.piece_tables[(Midgame, Queen, queen as u8)];
            eval.eg += params.piece_tables[(Endgame, Queen, queen as u8)];
            if T::TRACING {
                trace.queen_placement[queen as usize][color as usize] += 1;
            }

            // mobility
            let attacks = lookup_queen(queen as usize, self.combined);
            let mobility = (attacks & info.mobility_area[color as usize]).count_ones() as usize;
            eval.mg += params.queen_mobility[mobility][Midgame as usize];
            eval.eg += params.queen_mobility[mobility][Endgame as usize];
            if T::TRACING {
                trace.queen_mobility[mobility as usize][color as usize] += 1;
            }
        }
        eval
    }
    pub fn evaluate_king<T: TracingType>(
        &self,
        color: ColorIndex,
        _info: &EvalInfo,
        params: &EvalParams,
        trace: &mut EvalTrace,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        // placement
        let king = relative_board_index(self.piece_masks[(color, King)].lsb_index() as u8, color);
        eval.mg += params.piece_tables[(Midgame, King, king as u8)];
        eval.eg += params.piece_tables[(Endgame, King, king as u8)];
        if T::TRACING {
            trace.king_placement[king as usize][color as usize] += 1;
        }

        eval
    }

    pub fn evaluate_pawns<T: TracingType>(
        &self,
        color: ColorIndex,
        _info: &EvalInfo,
        params: &EvalParams,
        trace: &mut EvalTrace,
    ) -> EvalScore {
        let mut eval = EvalScore::zero();

        // material value
        let count = self.piece_masks[(color, Pawn)].count_ones() as i32;
        eval.mg += params.piece_values[(Midgame, Pawn)] * count;
        eval.eg += params.piece_values[(Endgame, Pawn)] * count;
        if T::TRACING {
            trace.pawn_count[color as usize] = count;
        }

        // placement
        for pawn in self.piece_masks[(color, Pawn)] {
            let pawn = relative_board_index(pawn, color);
            eval.mg += params.piece_tables[(Midgame, Pawn, pawn as u8)];
            eval.eg += params.piece_tables[(Endgame, Pawn, pawn as u8)];
            if T::TRACING {
                trace.pawn_placement[pawn as usize][color as usize] += 1;
            }
        }

        // passed pawns
        let front_spans = self.pawn_front_spans(!color);
        let all_front_spans =
            front_spans | (front_spans & NOT_H_FILE) << 1 | (front_spans & NOT_A_FILE) >> 1;
        let passers =
            (self.piece_masks[(color, Pawn)] & all_front_spans.inverse()).count_ones() as i32;
        eval.mg += params.passed_pawn_bonus[Midgame as usize] * passers;
        eval.eg += params.passed_pawn_bonus[Endgame as usize] * passers;
        if T::TRACING {
            trace.passed_pawns[color as usize] = passers;
        }

        // unsupported double pawns
        let pawns = self.piece_masks[(color, Pawn)];
        let shifted = if color == White {
            pawns >> 8
        } else {
            pawns << 8
        };
        let double_pawns =
            (pawns & shifted & ((pawns & NOT_H_FILE) << 1 | (pawns & NOT_A_FILE >> 1)).inverse())
                .count_ones() as i32;
        eval.mg += params.double_pawn_penalty[Midgame as usize] * double_pawns;
        eval.eg += params.double_pawn_penalty[Endgame as usize] * double_pawns;
        if T::TRACING {
            trace.double_pawns[color as usize] = double_pawns;
        }

        // backward pawns: -10?
        // isolated pawns: -20?

        eval
    }
}
