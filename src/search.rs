use crate::{bitboard::BitBoards, types::*};

const CHECKMATE_SCORE: i32 = 10000;
const ILLEGAL_MOVE_SCORE: i32 = 100000;
const DRAW_SCORE: i32 = 0;

impl BitBoards {
    pub fn search(&mut self, depth: usize) -> (i32, Move) {
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

        let mut best_score = i32::MIN;
        let mut best_move = Move::null();
        let mut any_legal_move = false;
        for move_ in &moves {
            self.make_move(move_);
            let (opp_score, _move) = self.search(depth - 1);
            self.unmake_move();

            if opp_score != ILLEGAL_MOVE_SCORE {
                any_legal_move = true;
            }

            if -opp_score > best_score {
                best_score = -opp_score;
                best_move = *move_;
            }
        }

        // no legal moves, check how the game ends
        if !any_legal_move {
            if self.king_not_in_check(self.current_player) {
                // stalemate
                return (DRAW_SCORE, Move::null());
            } else {
                // checkmate
                return (-CHECKMATE_SCORE, Move::null());
            }
        }

        if best_score == i32::MIN {
            best_move = moves[0]
        }

        (best_score, best_move)
    }
}
