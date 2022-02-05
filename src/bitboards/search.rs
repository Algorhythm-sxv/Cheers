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
        let mut boards = self.clone();
        for i in 0.. {
            let result = boards.negamax(i32::MIN + 1, i32::MAX - 1, i, Move::null());
            if !RUN_SEARCH.load(Ordering::Relaxed) {
                // can't trust results from a partial search
                break;
            }
            score = result.0;
            best_move = result.1;
            let pv_string = if !best_move.is_null() {
                format!(" pv {}", best_move.coords())
            } else {
                String::from("")
            };
            println!("info depth {i} score cp {score}{pv_string}");

            // terminate search at max depth
            if max_depth == Some(i) {
                RUN_SEARCH.store(false, Ordering::Relaxed);
                break;
            }
        }
        (score, best_move)
    }

    fn negamax(&mut self, mut alpha: i32, beta: i32, depth: usize, last_move: Move) -> (i32, Move) {
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
            let score = self.quiesce(alpha, beta, last_move);
            self.transposition_table
                .set(self.hash, Move::null(), depth as u8, score, Exact);
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
                if tt_entry.depth as usize >= depth
                    && (tt_entry.node_type == Exact || tt_entry.node_type == LowerBound)
                    && tt_entry.score >= beta
                {
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
            let null_score = -self.negamax(-beta, -beta + 1, depth - 3, Move::null()).0;
            self.unmake_null_move();

            if null_score >= beta {
                return (beta, Move::null());
            }
        }

        let mut moves = self.legal_moves();

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

        moves.iter_mut().for_each(|m| {
            m.sort_score = {
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
            }
        });
        moves.sort_unstable_by_key(|m| std::cmp::Reverse(m.sort_score));
        let mut best_move = *moves.first().unwrap();
        // let moves = MoveSorter::new(moves);

        for (i, &move_) in moves.iter().enumerate() {
            // Late move reduction on non-captures and non-queen-promotions
            let mut score = if i >= 3
                && depth >= 3
                && !move_.capture()
                && move_.promotion() != Queen
                && !self.in_check(self.current_player())
            {
                self.make_move(move_);
                // search with a null window; we only care whether it fails low or not
                let score = -self.negamax(-alpha - 1, -alpha, depth - 2, move_).0;
                self.unmake_move();
                score
            } else {
                alpha + 1
            };

            // search at full depth, if a reduced move improves alpha it is searched again
            if score > alpha {
                self.make_move(move_);
                score = -self.negamax(-beta, -alpha, depth - 1, move_).0;
                self.unmake_move();
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
        }
        self.transposition_table
            .set(self.hash, best_move, depth as u8, alpha, UpperBound);
        (alpha, best_move)
    }

    pub fn quiesce(&mut self, mut alpha: i32, beta: i32, last_move: Move) -> i32 {
        let stand_pat_score = self.evaluate();

        if stand_pat_score >= beta {
            return beta;
        }
        alpha = alpha.max(stand_pat_score);

        let moves = self.legal_moves();
        let mut moves: Vec<(Move, i32)> = moves
            .iter()
            .filter_map(|m| {
                if m.capture() {
                    let mut score = 0i32;
                    // try recaptures first
                    if last_move.capture() && m.target() == last_move.target() {
                        score += 1001;
                    }
                    // order captures by MVV-LVA
                    if !m.en_passent() {
                        score += PIECE_VALUES[self.piece_at(m.target() as usize)]
                            - PIECE_VALUES[m.piece()] / 10;
                    } else {
                        score += 90;
                    }
                    Some((*m, score))
                } else {
                    None
                }
            })
            .collect();
        moves.sort_unstable_by_key(|m| std::cmp::Reverse(m.1));

        for (move_, _) in moves.iter() {
            self.make_move(*move_);
            let score = -self.quiesce(-beta, -alpha, *move_);
            self.unmake_move();
            if score >= beta {
                return beta;
            }
            alpha = alpha.max(score);
        }
        alpha
    }
}
