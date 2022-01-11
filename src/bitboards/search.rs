use super::*;
use evaluate::*;

impl BitBoards {
    pub fn search(&self) -> (i32, Move) {
        self.negamax(i32::MIN + 1, i32::MAX - 1, 6)
    }

    fn negamax(&self, mut alpha: i32, beta: i32, depth: usize) -> (i32, Move) {
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
            return (self.quiesce(alpha, beta), Move::null());
        }

        let moves = self.legal_moves();

        if moves.is_empty() {
            if self.in_check(self.current_player) {
                // checkmate
                return (-CHECKMATE_SCORE, Move::null());
            } else {
                // stalemate
                return (DRAW_SCORE, Move::null());
            }
        }

        let mut best_move = Move::null();
        for move_ in moves {
            let mut copy = self.clone();
            copy.make_move(move_);
            let score = -copy.negamax(-beta, -alpha, depth - 1).0;
            if score >= beta {
                return (beta, move_);
            }
            if score > alpha {
                alpha = score;
                best_move = move_;
            }
        }
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
