use std::sync::atomic::*;

use super::*;
use crate::transposition_table::NodeType::*;
use evaluate::*;

pub static RUN_SEARCH: AtomicBool = AtomicBool::new(false);
pub static NODE_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static NPS_COUNT: AtomicUsize = AtomicUsize::new(0);

impl BitBoards {
    pub fn search(&self, max_depth: Option<usize>) -> (i32, Move) {
        let mut score = i32::MIN;
        let mut best_move = Move::null();
        for i in 0.. {
            let result = self.negamax(i32::MIN + 1, i32::MAX - 1, i, Move::null());
            if !RUN_SEARCH.load(Ordering::Relaxed) {
                // can't trust results from a partial search
                break;
            }
            score = result.0;
            best_move = result.1;
            let pv = best_move.coords();
            println!("info depth {i} score cp {score} pv {pv}");

            // terminate search at max depth
            if max_depth == Some(i) {
                RUN_SEARCH.store(false, Ordering::Relaxed);
                break;
            }
        }
        (score, best_move)
    }

    fn negamax(&self, mut alpha: i32, beta: i32, depth: usize, last_move: Move) -> (i32, Move) {
        NODE_COUNT.fetch_add(1, Ordering::Relaxed);
        NPS_COUNT.fetch_add(1, Ordering::Relaxed);

        // terminate search early
        if !RUN_SEARCH.load(Ordering::Relaxed) {
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
            return (DRAW_SCORE, Move::null());
        }

        // quiescence search at full depth
        if depth == 0 {
            let score = self.quiesce(alpha, beta);
            self.transposition_table
                .set(self.hash, Move::null(), depth as u8, score, Exact);
            return (score, Move::null());
        }

        // transposition table lookup
        let mut tt_move = Move::null();
        if let Some(tt_entry) = self.transposition_table.get(self.hash) {
            // prune on exact score/beta cutoff with pseudolegal move and equal/higher depth
            if tt_entry.depth as usize >= depth
                && (tt_entry.node_type == Exact || tt_entry.node_type == LowerBound) && tt_entry.score >= beta
                    && self.is_pseudolegal(tt_entry.move_start, tt_entry.move_target) {
                return (
                    beta,
                    self.move_from(
                        tt_entry.move_start,
                        tt_entry.move_target,
                        tt_entry.promotion,
                    ),
                );
            }
            // build a dummy move for move ordering
            tt_move = Move::new(
                tt_entry.move_start,
                tt_entry.move_target,
                NoPiece,
                tt_entry.promotion,
                false,
                false,
                false,
                false,
            );
        }
        let moves = self.legal_moves();

        if moves.is_empty() {
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

        let mut moves: Vec<(Move, i32)> = moves
            .iter()
            .map(|m| {
                (*m, {
                    let mut score = 0i32;
                    // try the transposition table move early
                    if m.start() == tt_move.start() && m.target() == tt_move.target() {
                        score += 1000;
                    }
                    if m.capture() {
                        // try recaptures first
                        if last_move.capture() && m.target() == last_move.target() {
                            score += 1001;
                        }
                        // order captures before quiet moves, MVV-LVA
                        if !m.en_passent() {
                            score += PIECE_VALUES[self.piece_at(m.target() as usize)]
                                - PIECE_VALUES[m.piece()] / 10;
                        } else {
                            score += 90;
                        }
                    }
                    score
                })
            })
            .collect();
        moves.sort_unstable_by_key(|m| std::cmp::Reverse(m.1));

        let mut best_move = moves.first().unwrap().0;
        for (move_, _) in moves {
            let mut copy = self.clone();
            copy.make_move(move_);
            let score = -copy.negamax(-beta, -alpha, depth - 1, move_).0;
            if score >= beta {
                self.transposition_table
                    .set(self.hash, move_, depth as u8, beta, LowerBound);
                return (beta, move_);
            }
            if score > alpha {
                alpha = score;
                best_move = move_;
            }
        }
        self.transposition_table
            .set(self.hash, best_move, depth as u8, alpha, UpperBound);
        (alpha, best_move)
    }

    pub fn quiesce(&self, mut alpha: i32, beta: i32) -> i32 {
        let stand_pat_score = self.evaluate();

        if stand_pat_score >= beta {
            return beta;
        }
        alpha = alpha.max(stand_pat_score);

        let moves = self.legal_moves();
        for move_ in moves.iter().filter(|m| m.capture()) {
            let mut boards = self.clone();
            boards.make_move(*move_);
            let score = -boards.quiesce(-beta, -alpha);
            if score >= beta {
                return beta;
            }
            alpha = alpha.max(score);
        }
        alpha
    }
}
