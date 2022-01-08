use super::*;

impl BitBoards {
    pub fn search(&self) -> (i32, Move) {
        self.negamax(i32::MIN + 1, i32::MAX - 1, 7)
    }

    fn negamax(&self, mut alpha: i32, beta: i32, depth: usize) -> (i32, Move) {
        if depth == 0 {
            return (self.quiesce(alpha, beta), Move::null());
        }

        let moves = self.legal_moves();

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
