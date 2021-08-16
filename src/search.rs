use std::hash;

use crate::{bitboard::BitBoards, evaluate::consts::*, transposition_table::NodeType::*, types::*};

impl BitBoards {
    pub fn toplevel_search(&mut self, mut alpha: i32, beta: i32, depth: usize) -> (i32, Move) {
        // avoid illegal moves
        if !self.king_not_in_check(!self.current_player) {
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

        let mut moves = Vec::with_capacity(50);
        self.generate_pseudolegal_moves(&mut moves);

        let mut best_move = Move::null();
        let mut any_legal_move = false;
        for move_ in &moves {
            self.make_move(move_);
            let score = -self.search(-beta, -alpha, depth - 1);
            self.unmake_move();

            if score >= beta {
                self.transposition_table
                    .set(&self, *move_, depth as u8, beta, LowerBound);
                return (beta, *move_);
            }
            if score > alpha {
                alpha = score;
                best_move = *move_;
            }

            if score != -ILLEGAL_MOVE_SCORE {
                any_legal_move = true;
            }
        }

        // no legal moves, check how the game ends
        if !any_legal_move {
            if self.king_not_in_check(self.current_player) {
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
        self.transposition_table
            .set(&self, best_move, depth as u8, alpha, UpperBound);
        (alpha, best_move)
    }

    pub fn search(&mut self, mut alpha: i32, beta: i32, depth: usize) -> i32 {
        // avoid illegal moves
        if !self.king_not_in_check(!self.current_player) {
            return ILLEGAL_MOVE_SCORE;
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
                return DRAW_SCORE;
            }
        }
        let mut moves = Vec::with_capacity(50);
        self.generate_pseudolegal_moves(&mut moves);
        if let Some(((start, end), hash_depth, promotion, score, node_type)) =
            self.transposition_table.get(&self)
        {
            if hash_depth >= depth as u8 {
                // the transposition table result came from an equal or better search!
                if node_type != UpperBound {
                    // exact or lower bound, return like a beta cutoff
                    return score;
                } else {
                    if score > alpha {
                        // improve the min score of this position before search
                        alpha = score;
                    }
                }
            } else {
                // the transposition table result came from a worse search, use for move ordering
                // only use the move if it is pseudolegal, an illegal move from a transpostion table indicates a hash collision
                let hash_move = Move::new(start, end, promotion);
                if moves.contains(&hash_move) {
                    moves.push(hash_move);
                }
            }
        }

        if depth == 0 {
            let score = self.quiesce(alpha, beta);
            self.transposition_table
                .set(&self, Move::null(), depth as u8, score, Exact);
            return score;
        }

        // self.generate_pseudolegal_moves(&mut moves);
        let mut any_legal_move = false;
        let mut best_move = Move::null();
        for move_ in &moves {
            self.make_move(move_);
            let score = -self.search(-beta, -alpha, depth - 1);
            self.unmake_move();

            if score >= beta {
                self.transposition_table
                    .set(&self, *move_, depth as u8, beta, LowerBound);
                return beta;
            }
            if score > alpha {
                alpha = score;
                best_move = *move_;
            }

            if score != -ILLEGAL_MOVE_SCORE {
                any_legal_move = true;
            }
        }

        // no legal moves, check how the game ends
        if !any_legal_move {
            if self.king_not_in_check(self.current_player) {
                // stalemate
                self.transposition_table
                    .set(&self, Move::null(), depth as u8, DRAW_SCORE, Exact);
                return DRAW_SCORE;
            } else {
                // checkmate
                self.transposition_table.set(
                    &self,
                    Move::null(),
                    depth as u8,
                    CHECKMATE_SCORE,
                    Exact,
                );
                return CHECKMATE_SCORE;
            }
        }
        self.transposition_table
            .set(&self, best_move, depth as u8, alpha, UpperBound);
        alpha
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

    fn quiesce(&mut self, mut alpha: i32, beta: i32) -> i32 {
        use crate::piece_tables::GamePhase::*;

        // avoid illegal moves
        if !self.king_not_in_check(!self.current_player) {
            return ILLEGAL_MOVE_SCORE;
        }

        let stand_pat_score = self.evaluate(self.current_player);
        if stand_pat_score >= beta {
            return beta;
        }
        if stand_pat_score > alpha {
            alpha = stand_pat_score;
        }

        let mut captures = self.generate_captures();
        // sort by descending material difference (i.e search PxQ first)
        captures.sort_unstable_by(|a, b| a.material_difference().cmp(&b.material_difference()));

        for capture in &captures {
            // delta pruning, if the captured piece doesn't restore material balance enough then prune the tree
            // e.g. if a rook down, don't bother searching pawn captures
            // disable during late endgame to avoid bias away from captures for draws/wins
            if self.game_phase() > 200
                && self.material_count(self.current_player, EndGame)
                    - self.material_count(!self.current_player, EndGame)
                    // if there is no piece on the target square it is (hopefully) an en passent capture
                    + PIECE_VALUES[(EndGame, self.piece_list[capture.target as usize].unwrap_or((Pawn, White)).0)]
                    + 200
                    < 0
            {
                continue;
            }

            self.make_move(&capture.to_move());
            let score = -self.quiesce(-beta, -alpha);
            self.unmake_move();

            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }
        alpha
    }
}
