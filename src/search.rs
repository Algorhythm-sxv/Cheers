use std::time::*;

use rayon::prelude::*;

use crate::{bitboard::BitBoards, evaluate::consts::*, transposition_table::NodeType::*, types::*};

impl BitBoards {
    pub fn toplevel_search(&mut self, alpha: i32, beta: i32, depth: usize) -> (i32, Move) {
        // avoid illegal moves
        if self.king_in_check(!self.current_player) {
            return (ILLEGAL_MOVE_SCORE, Move::null());
        }

        // weird draws
        if self.halfmove_clock >= 8 {
            // 50-move rule
            if self.halfmove_clock >= 100
            // threefold repetition
            || self
            .position_history
            .iter()
            .filter(|&&pos| pos == self.position_hash)
            .count()
            == 2
            {
                return (DRAW_SCORE, Move::null());
            }
        }

        if depth == 0 {
            // a toplevel search with depth 0 just returns the static score
            let score = self.quiesce(alpha, beta);
            self.transposition_table
                .set(&self, Move::null(), depth as u8, score, Exact);
            return (score, Move::null());
        }

        let start_time = Instant::now();

        // Iterative deepening with Lazy SMP
        let (tx, rx) = std::sync::mpsc::sync_channel(depth);
        let tx = tx.clone();
        (0..depth).into_par_iter().for_each(|i| {
            let (score, best_move) = self.clone().search(alpha, beta, i);
            if score == ILLEGAL_MOVE_SCORE {
                // position is illegal, opponent is checkmated
                tx.send((-CHECKMATE_SCORE, Move::null(), i))
                    .expect("Failed to send results to main thread!");
            }
            // send move and score back
            tx.send((score, best_move, i))
                .expect("Failed to send result to main thread!")
        });

        let mut best_score = i32::MIN;
        let mut best_move = Move::null();
        let mut top_depth = 0;
        while let Ok((score, move_, depth)) = rx.try_recv() {
            println!(
                "Best move at depth {}: {}, score {}",
                depth,
                move_.to_algebraic_notation(),
                score
            );
            if depth >= top_depth {
                best_score = score;
                best_move = move_;
                top_depth = depth;
            }
        }
        let end_time = Instant::now();
        println!(
            "Search completed in {}s",
            (end_time - start_time).as_millis() as f32 / 1000.0
        );

        return (best_score, best_move);
    }

    pub fn search(&mut self, mut alpha: i32, mut beta: i32, depth: usize) -> (i32, Move) {
        let alpha_old = alpha;
        // avoid illegal moves
        if self.king_in_check(!self.current_player) {
            return (ILLEGAL_MOVE_SCORE, Move::null());
        }

        // weird draws
        if self.halfmove_clock >= 8 {
            // 50-move rule
            if self.halfmove_clock >= 100
            // threefold repetition
            || self
            .position_history
            .iter()
            .filter(|&&pos| pos == self.position_hash)
            .count()
            == 2
            {
                return (DRAW_SCORE, Move::null());
            }
        }

        if depth == 0 {
            let score = self.quiesce(alpha, beta);
            self.transposition_table
                .set(&self, Move::null(), depth as u8, score, Exact);
            return (score, Move::null());
        }

        let mut hash_move = None;
        if let Some(((start, end), hash_depth, promotion, score, node_type)) =
            self.transposition_table.get(&self)
        {
            hash_move = Some(Move::new(start, end, promotion));
            if hash_depth >= depth as u8 {
                // the transposition table result came from an equal or better search!
                match node_type {
                    Exact => {
                        return (score, hash_move.unwrap());
                    }
                    LowerBound => alpha = alpha.max(score),
                    UpperBound => beta = beta.min(score),
                }
                if alpha >= beta {
                    return (score, hash_move.unwrap());
                }
            }
            // the transposition table result is not exact or came from a worse search, use for move ordering
        }
        let mut non_captures = self.generate_non_captures();
        non_captures.extend(self.generate_legal_castles());

        // order captures by Most Valuable Victim, Least Valuable Attacker
        let mut captures = self.generate_captures();
        captures.sort_unstable_by(|a, b| a.material_difference().cmp(&b.material_difference()));
        let captures: Vec<Move> = captures.into_iter().map(|c| c.to_move()).collect();

        // hash move is not pseudolegal
        if let Some(hash_move_inner) = hash_move {
            if !(captures.contains(&hash_move_inner) || non_captures.contains(&hash_move_inner)) {
                hash_move = None;
            }
        }

        let moves = hash_move
            .iter()
            .chain(captures.iter())
            .chain(non_captures.iter());

        let mut any_legal_move = false;
        let mut best_move = Move::null();
        let mut score = i32::MIN;
        for move_ in moves {
            self.make_move(move_);
            score = score.max(-self.search(-beta, -alpha, depth - 1).0);
            self.unmake_move();

            if score > alpha {
                alpha = score;
                best_move = *move_;
            }

            if score != -ILLEGAL_MOVE_SCORE {
                any_legal_move = true;
            }

            if alpha >= beta {
                break;
            }
        }

        // no legal moves, check how the game ends
        if !any_legal_move {
            if !self.king_in_check(self.current_player) {
                // stalemate
                self.transposition_table
                    .set(&self, Move::null(), depth as u8, DRAW_SCORE, Exact);
                return (DRAW_SCORE, Move::null());
            } else {
                // checkmate
                self.transposition_table.set(
                    &self,
                    Move::null(),
                    depth as u8,
                    CHECKMATE_SCORE,
                    Exact,
                );
                return (CHECKMATE_SCORE, Move::null());
            }
        }
        if score <= alpha_old {
            self.transposition_table
                .set(&self, best_move, depth as u8, score, UpperBound);
        } else if score >= beta {
            self.transposition_table
                .set(&self, best_move, depth as u8, score, LowerBound)
        } else {
            self.transposition_table
                .set(&self, best_move, depth as u8, score, Exact)
        }
        (score, best_move)
    }

    pub fn perft(fen: String, depth: usize) -> Result<usize, Box<dyn std::error::Error>> {
        let mut boards = Self::new(0);
        boards.set_from_fen(fen)?;

        Ok(boards._perft(depth))
    }

    pub fn _perft(&mut self, depth: usize) -> usize {
        if depth == 0 {
            return 1;
        }

        let moves = self.generate_legal_moves();
        let mut nodes = 0;

        for move_ in moves {
            self.make_move(&move_);
            nodes += self._perft(depth - 1);
            self.unmake_move();
        }
        nodes
    }

    fn quiesce(&mut self, mut alpha: i32, mut beta: i32) -> i32 {
        use crate::piece_tables::GamePhase::*;

        let alpha_old = alpha;

        // avoid illegal moves
        if self.king_in_check(!self.current_player) {
            return ILLEGAL_MOVE_SCORE;
        }

        let stand_pat_score = self.evaluate(self.current_player);
        if stand_pat_score >= beta {
            return beta;
        }
        alpha = alpha.max(stand_pat_score);

        let mut captures = self.generate_captures();
        // sort by descending material difference (i.e search PxQ first)
        captures.sort_unstable_by(|a, b| a.material_difference().cmp(&b.material_difference()));
        if let Some(((_, _), _, _, score, node_type)) = self.transposition_table.get(&self) {
            match node_type {
                Exact => return score,
                LowerBound => alpha = alpha.max(score),
                UpperBound => beta = beta.min(score),
            }
        }

        let mut score = i32::MIN;
        for capture in &captures {
            // delta pruning, if the captured piece doesn't restore material balance enough then prune the tree
            // e.g. if a rook down, don't bother searching pawn captures
            // disable during late endgame to avoid bias away from captures for draws/wins
            if self.game_phase() > 200
                && self.material_count(self.current_player, EndGame)
                    - self.material_count(!self.current_player, EndGame)
                    + PIECE_VALUES[(EndGame, capture.capture)]
                    + 200
                    < 0
            {
                continue;
            }

            self.make_move(&capture.to_move());
            score = -self.quiesce(-beta, -alpha);
            self.unmake_move();

            alpha = alpha.max(score);

            if alpha >= beta {
                break;
            }
        }

        if score <= alpha_old {
            self.transposition_table
                .set(&self, Move::null(), 0, score, UpperBound);
        } else if score >= beta {
            self.transposition_table
                .set(&self, Move::null(), 0, score, LowerBound)
        } else {
            self.transposition_table
                .set(&self, Move::null(), 0, score, Exact)
        }

        alpha
    }
}
