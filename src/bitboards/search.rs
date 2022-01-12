use super::*;
use evaluate::*;

impl BitBoards {
    pub fn search(&self) -> (i32, Move) {
        self.negamax(i32::MIN + 1, i32::MAX - 1, 5, Move::null())
    }

    fn negamax(&self, mut alpha: i32, beta: i32, depth: usize, last_move: Move) -> (i32, Move) {
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

        let mut moves: Vec<(Move, i32)> = moves
            .iter()
            .map(|m| {
                (
                    *m,
                    if m.capture() {
                        let mut score = 0i32;
                        // try recaptures first
                        if last_move.capture() && m.target() == last_move.target() {
                            score += 1000;
                        }
                        // order captures before quiet moves, MVV-LVA
                        if !m.en_passent() {
                            score += PIECE_VALUES[self.piece_at(m.target() as usize)]
                                - PIECE_VALUES[m.piece()] / 10;
                        } else {
                            score += 90;
                        }
                        score
                    } else {
                        0i32
                    },
                )
            })
            .collect();
        moves.sort_unstable_by_key(|m| std::cmp::Reverse(m.1));

        let mut best_move = moves.first().unwrap().0;
        for (move_, _) in moves {
            let mut copy = self.clone();
            copy.make_move(move_);
            let score = -copy.negamax(-beta, -alpha, depth - 1, move_).0;
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
