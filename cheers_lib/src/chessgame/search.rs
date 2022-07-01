use std::{fmt::Display, sync::atomic::*};

use super::{
    eval_types::{GamePhase, TraceTarget},
    *,
};
use crate::transposition_table::NodeType::*;
use GamePhase::*;

pub static RUN_SEARCH: AtomicBool = AtomicBool::new(false);
pub static NODE_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static NPS_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Copy, Clone, Default, Debug)]
pub struct PrincipalVariation {
    pub len: usize,
    pub moves: [Move; 16],
}

impl PrincipalVariation {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Display for PrincipalVariation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for m in self.moves.iter().take(self.len) {
            write!(f, "{} ", m.coords())?;
        }
        Ok(())
    }
}

impl ChessGame {
    pub fn search(&self, max_depth: Option<usize>, quiet: bool) -> (i32, Move) {
        RUN_SEARCH.store(true, Ordering::Relaxed);
        let mut score = i32::MIN;
        let mut best_move = Move::null();
        let mut boards = self.clone();
        for i in 0.. {
            let mut pv = PrincipalVariation::new();
            let result = boards.negamax(
                i32::MIN + 1,
                i32::MAX - 1,
                i as i32,
                0,
                Move::null(),
                &mut pv,
            );
            if !RUN_SEARCH.load(Ordering::Relaxed) && i > 1 {
                // can't trust results from a partial search
                break;
            }
            score = result.0;
            best_move = result.1;
            let pv_string = if !best_move.is_null() {
                // format!(" pv {}", best_move.coords())
                format!(" pv {pv}")
            } else {
                String::from("")
            };
            if !quiet {
                println!("info depth {i} score cp {score}{pv_string}")
            };

            // terminate search at max depth
            if max_depth == Some(i) {
                RUN_SEARCH.store(false, Ordering::Relaxed);
                break;
            }
        }
        (score, best_move)
    }

    fn negamax(
        &mut self,
        mut alpha: i32,
        beta: i32,
        depth: i32,
        ply: usize,
        last_move: Move,
        pv: &mut PrincipalVariation,
    ) -> (i32, Move) {
        NODE_COUNT.fetch_add(1, Ordering::Relaxed);
        NPS_COUNT.fetch_add(1, Ordering::Relaxed);

        // terminate search early
        if !RUN_SEARCH.load(Ordering::Relaxed) && depth > 1 {
            return (0, Move::null());
        }

        // check 50 move and repetition draws
        if self.halfmove_clock == 100
            || self
                .position_history
                .iter()
                .filter(|&&p| p == self.hash)
                .count()
                == 2
        {
            // exact score so we must reset the pv
            pv.len = 0;
            return (DRAW_SCORE, Move::null());
        }

        let mut line = PrincipalVariation::new();
        // quiescence search at full depth
        if depth == 0 {
            // exact score so we must reset the pv
            pv.len = 0;
            let score = self.quiesce(alpha, beta, -1, last_move, EVAL_PARAMS);
            self.transposition_table
                .set(self.hash, Move::null(), depth as i8, score, Exact);
            return (score, Move::null());
        }

        // transposition table lookup
        let mut tt_move = Move::null();
        if let Some(tt_entry) = self.transposition_table.get(self.hash) {
            // if the tt move is pseudolegal cross fingers we don't have a key collision
            if self.is_pseudolegal(tt_entry.move_start, tt_entry.move_target) {
                tt_move = Move::new(
                    tt_entry.move_start,
                    tt_entry.move_target,
                    self.piece_at(tt_entry.move_start as usize),
                    tt_entry.promotion,
                    tt_entry.en_passent_capture
                        || self.piece_at(tt_entry.move_target as usize) != NoPiece,
                    tt_entry.double_pawn_push,
                    tt_entry.en_passent_capture,
                    tt_entry.castling,
                );
                // prune on exact score/beta cutoff with pseudolegal move and equal/higher depth
                if tt_entry.depth as i32 >= depth
                    && (tt_entry.node_type == Exact || tt_entry.node_type == LowerBound)
                    && tt_entry.score >= beta
                {
                    // exact score (?) so we must reset the pv
                    pv.len = 0;
                    return (beta, tt_move);
                }
            }
        }

        // Null move pruning
        // don't search the null move when in check or only down to pawn/kings
        if depth >= 3
            && !self.in_check(self.current_player)
            && self.has_non_pawn_material(self.current_player)
        {
            self.make_null_move();
            let null_score = -self
                .negamax(
                    -beta,
                    -beta + 1,
                    depth - 3,
                    ply + 1,
                    Move::null(),
                    &mut line,
                )
                .0;
            self.unmake_null_move();

            if null_score >= beta {
                return (beta, Move::null());
            }
        }

        let moves = self.legal_moves();

        if moves.is_empty() {
            // exact score, so we must reset the pv
            pv.len = 0;
            if self.in_check(self.current_player) {
                // checkmate, preferring shorter mating sequences
                return (-CHECKMATE_SCORE - depth as i32, Move::null());
            } else {
                // stalemate
                return (DRAW_SCORE, Move::null());
            }
        }

        let depth = if moves.len() == 1 {
            // reduce depth when we only have 1 legal move
            depth.saturating_sub(3).max(1)
        } else {
            depth
        };

        let mut moves = moves
            .into_iter()
            .map(|m| {
                (m, {
                    let mut score = 0i32;
                    // try the transposition table move early
                    if m.start() == tt_move.start() && m.target() == tt_move.target() {
                        score += 100_000;
                    } else if m.capture() {
                        score += 2000;

                        // try recaptures first, least valuable piece first
                        if last_move.capture() && m.target() == last_move.target() {
                            score += 10_000 - EVAL_PARAMS.piece_values[(Midgame, m.piece())] / 10;
                        }
                        // order all captures before quiet moves, MVV-LVA
                        if !m.en_passent() {
                            score += EVAL_PARAMS.piece_values
                                [(Midgame, self.piece_at(m.target() as usize))]
                                - EVAL_PARAMS.piece_values[(Midgame, m.piece())];
                        }
                        // order queen and rook promotions ahead of quiet moves
                        else if m.promotion() == Queen || m.promotion() == Rook {
                            score += EVAL_PARAMS.piece_values[(Midgame, m.promotion())] + 100;
                        }
                    // quiet killer moves get sorted after captures but before other quiet moves
                    } else if self.killer_moves[ply].contains(&m) {
                        score += 500;
                    // quiet moves get ordered by their history heuristic
                    } else {
                        score += self.history_tables[self.current_player()][m.piece()]
                            [m.target() as usize];
                    }
                    score
                })
            })
            .collect::<Vec<(Move, i32)>>();
        // moves.sort_unstable_by_key(|m| std::cmp::Reverse(m.sort_score));
        // make sure the reported best move is at least legal
        let mut best_move = moves.first().unwrap().0;

        for i in 0..moves.len() {
            pick_move(&mut moves, i);
            let move_ = moves[i].0;
            // Late move reduction on non-captures and non-queen-promotions
            let mut score = if i >= 3
                && depth >= 3
                && !move_.capture()
                && move_.promotion() != Queen
                && !self.in_check(self.current_player())
            {
                self.make_move(move_);
                // search with a null window; we only care whether it fails low or not
                let score = -self
                    .negamax(-alpha - 1, -alpha, depth - 2, ply + 1, move_, &mut line)
                    .0;
                self.unmake_move();
                score
            } else {
                alpha + 1
            };

            // search at full depth, if a reduced move improves alpha it is searched again
            if score > alpha {
                self.make_move(move_);
                score = -self
                    .negamax(-beta, -alpha, depth - 1, ply + 1, move_, &mut line)
                    .0;
                self.unmake_move();
                if score >= beta {
                    self.transposition_table
                        .set(self.hash, move_, depth as i8, beta, LowerBound);
                    if !move_.capture() {
                        self.history_tables[self.current_player][move_.piece()]
                            [move_.target() as usize] += depth * depth;
                        if move_.promotion() == NoPiece {
                            self.killer_moves.push(move_, ply);
                        }
                    }
                    return (beta, move_);
                }
                if score > alpha {
                    // update PV
                    pv.moves[0] = move_;
                    pv.moves[1..(line.len + 1)].copy_from_slice(&line.moves[..line.len]);
                    pv.len = line.len + 1;

                    alpha = score;
                    best_move = move_;
                }
            }
        }
        self.transposition_table
            .set(self.hash, best_move, depth as i8, alpha, UpperBound);
        (alpha, best_move)
    }

    pub fn quiesce(
        &mut self,
        alpha: i32,
        beta: i32,
        depth: i32,
        last_move: Move,
        eval_params: EvalParams,
    ) -> i32 {
        self._quiesce::<()>(alpha, beta, depth, last_move, eval_params)
            .0
    }

    pub fn _quiesce<T: TraceTarget + Default>(
        &mut self,
        mut alpha: i32,
        beta: i32,
        depth: i32,
        last_move: Move,
        eval_params: EvalParams,
    ) -> (i32, T) {
        let (stand_pat_score, mut best_trace) = self.evaluate::<T>();

        if stand_pat_score >= beta {
            return (beta, best_trace);
        }
        alpha = alpha.max(stand_pat_score);

        // transposition table lookup
        let mut tt_move = Move::null();
        if !T::TRACING {
            if let Some(tt_entry) = self.transposition_table.get(self.hash) {
                if self.is_pseudolegal(tt_entry.move_start, tt_entry.move_target) {
                    tt_move = Move::new(
                        tt_entry.move_start,
                        tt_entry.move_target,
                        self.piece_at(tt_entry.move_start as usize),
                        tt_entry.promotion,
                        tt_entry.en_passent_capture
                            || self.piece_at(tt_entry.move_target as usize) != NoPiece,
                        tt_entry.double_pawn_push,
                        tt_entry.en_passent_capture,
                        tt_entry.castling,
                    );
                }
            }
        }
        let mut moves: Vec<(Move, i32)> = self
            .legal_moves()
            .into_iter()
            .filter(|m| m.capture())
            .map(|m| {
                (m, {
                    let mut score = 0i32;
                    // try the transposition table move early
                    if m.start() == tt_move.start()
                        && m.target() == tt_move.target()
                        && tt_move.capture()
                    {
                        score += 10_000;
                    }
                    // try recaptures first
                    if last_move.capture() && m.target() == last_move.target() {
                        score += 2000;
                    }
                    // order captures before quiet moves, MVV-LVA
                    if !m.en_passent() {
                        score += EVAL_PARAMS.piece_values
                            [(Midgame, self.piece_at(m.target() as usize))]
                            - EVAL_PARAMS.piece_values[(Midgame, m.piece())];
                    }
                    score
                })
            })
            .collect();
        moves.sort_unstable_by_key(|m| std::cmp::Reverse(m.1));

        let mut best_move = Move::null();
        for (move_, _) in moves.iter() {
            self.make_move(*move_);
            let (mut score, trace) =
                self._quiesce::<T>(-beta, -alpha, depth - 1, *move_, eval_params);
            score = -score;
            self.unmake_move();
            if score >= beta {
                if !T::TRACING {
                    self.transposition_table
                        .set(self.hash, *move_, depth as i8, beta, LowerBound);
                }
                return (beta, trace);
            }
            if score > alpha {
                alpha = score;
                best_trace = trace;
                best_move = *move_;
            }
        }
        if !T::TRACING {
            self.transposition_table
                .set(self.hash, best_move, depth as i8, alpha, UpperBound);
        }
        (alpha, best_trace)
    }
}
