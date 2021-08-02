use crate::{bitboard::BitBoards, types::*, evaluate::consts::*};

impl BitBoards {
    pub fn search(&mut self, mut alpha: i32, beta: i32, depth: usize) -> (i32, Move) {
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
            let (opp_score, _move) = self.search(-beta, -alpha, depth - 1);
            self.unmake_move();

            if -opp_score >= beta {
                return (beta, *move_)
            }
            if -opp_score > alpha {
                alpha = -opp_score;
                best_move = *move_;
            }

            if opp_score != ILLEGAL_MOVE_SCORE {
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

        for move_ in moves{
            self.make_move(&move_);
            nodes += self._perft(depth-1);
            self.unmake_move();
        }
        nodes
    }

    fn quiesce(&mut self, mut alpha: i32, beta: i32) -> i32 {
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