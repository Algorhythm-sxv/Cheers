use super::*;

impl BitBoards {
    pub fn search(&self) -> (i32, Move) {
        let moves = self.legal_moves();

        let mut best_score = i32::MIN;
        let mut best_move = Move::null();
        for move_ in moves {
            let mut copy = self.clone();
            println!("{}", move_);
            copy.make_move(move_);
            let score = -copy.evaluate();
            if score > best_score {
                best_score = score;
                best_move = move_;
            }
        }
        (best_score, best_move)
    }
}