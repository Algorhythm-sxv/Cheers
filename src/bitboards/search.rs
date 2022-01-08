use super::*;

impl BitBoards {
    pub fn search(&self) -> (i32, Move) {
        self.negamax(5)
    }

    fn negamax(&self, depth: usize) -> (i32, Move) {
        if depth == 0 {
            return (self.evaluate(), Move::null())
        }

        let moves = self.legal_moves();

        let mut best_score = i32::MIN;
        let mut best_move = Move::null();
        for move_ in moves {
            let mut copy = self.clone();
            copy.make_move(move_);
            let score = -copy.negamax(depth - 1).0;
            if score > best_score {
                best_score = score;
                best_move = move_;
            }
        }
        (best_score, best_move)
    }   
}