use crate::{bitboard::BitBoards, evaluate::consts::*, types::*};

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
            return (self.evaluate(self.current_player), Move::null());
        }

        let moves = self.generate_pseudolegal_moves();

        let mut best_move = Move::null();
        let mut any_legal_move = false;
        for move_ in &moves {
            self.make_move(move_);
            let score = -self.search(-beta, -alpha, depth - 1);
            self.unmake_move();

            if score >= beta {
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
                return (DRAW_SCORE, Move::null());
            } else {
                // checkmate
                return (CHECKMATE_SCORE, Move::null());
            }
        }
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

        if depth == 0 {
            return self.evaluate(self.current_player);
        }

        let moves = self.generate_pseudolegal_moves();
        let mut any_legal_move = false;
        for move_ in &moves {
            self.make_move(move_);
            let score = -self.search(-beta, -alpha, depth - 1);
            self.unmake_move();

            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }

            if score != -ILLEGAL_MOVE_SCORE {
                any_legal_move = true;
            }
        }

        // no legal moves, check how the game ends
        if !any_legal_move {
            if self.king_not_in_check(self.current_player) {
                // stalemate
                return DRAW_SCORE;
            } else {
                // checkmate
                return CHECKMATE_SCORE;
            }
        }
        alpha
    }

    pub fn perft(fen: String, depth: usize) -> Result<usize, Box<dyn std::error::Error>> {
        let mut boards = Self::new();
        boards.set_from_fen(fen)?;

        Ok(boards._perft(depth))
    }

    fn _perft(&mut self, depth: usize) -> usize {
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

        let captures = self.generate_captures();

        for capture in &captures {
            // delta pruning, if the captured piece doesn't restore material balance enough then prune the tree
            // e.g. if a rook down, don't bother searching pawn captures
            // disable during late endgame to avoid bias away from captures for draws/wins
            if self.game_phase() < 200
                && self.material_count(self.current_player, EndGame)
                    - self.material_count(!self.current_player, EndGame)
                    + PIECE_VALUES[(EndGame, self.piece_list[capture.target as usize].unwrap().0)]
                    + 200
                    < 0
            {
                continue;
            }

            self.make_move(capture);
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
