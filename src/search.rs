use crate::{bitboard::BitBoards, types::Move};

impl BitBoards {
    pub fn search(&mut self, depth: usize) -> (i32, Move) {
        if depth == 0 {
            return (self.evaluate(self.current_player), Move::null());
        }

        // 50-move rule
        if self.halfmove_clock == 50 {
            return (0, Move::null());
        }

        let moves = self.generate_legal_moves();

        if moves.len() == 0 {
            if self.king_not_in_check(self.current_player) {
                // stalemate
                return (0, Move::null());
            } else {
                // checkmate
                return (i32::MIN, Move::null());
            }
        }

        let mut best_score = i32::MIN;
        let mut best_move = Move::null();
        for move_ in &moves {
            self.make_move(move_);
            let (opp_score, _move) = self.search(depth-1);
            self.unmake_move();
            if -opp_score > best_score {
                best_score = -opp_score;
                best_move = *move_;
            }
        }

        if best_score == i32::MIN {
            best_move = moves[0]
        }

        (best_score, best_move)
    }
}
