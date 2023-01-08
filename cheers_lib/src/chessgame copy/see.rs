use crate::{
    lookup_tables::{lookup_bishop, lookup_rook},
    moves::Move,
    types::{ColorIndex::*, Piece::*, PIECES},
};

use super::ChessGame;

pub const SEE_PIECE_VALUES: [i32; 7] = [100, 300, 300, 500, 900, 20000, 0];
impl ChessGame {
    pub fn see(&self, move_: Move) -> i32 {
        let target = move_.target();
        let mut swap_list = [0i32; 32];

        let mut current_attacker = move_.piece();
        let mut attacker_mask = move_.start().bitboard();

        let bishops = self.piece_masks[(White, Bishop)]
            | self.piece_masks[(Black, Bishop)]
            | self.piece_masks[(White, Queen)]
            | self.piece_masks[(Black, Queen)];

        let rooks = self.piece_masks[(White, Rook)]
            | self.piece_masks[(Black, Rook)]
            | self.piece_masks[(White, Queen)]
            | self.piece_masks[(Black, Queen)];

        // simulate the first capture
        swap_list[0] = SEE_PIECE_VALUES[self.piece_at(target)];
        let mut occupied = self.combined;
        let mut color = !self.current_player;

        // correct for en passent capture
        if move_.en_passent() {
            // shift the pawn back to the normal square for en passent
            occupied ^= self.en_passent_mask
                | (self.en_passent_mask >> 8 << 16 * (self.current_player as u8));
            swap_list[0] = SEE_PIECE_VALUES[Pawn];
        }

        let mut attackers = self.all_attacks_on(target, occupied);

        let mut i = 0;
        for _ in 1..32 {
            i += 1;
            swap_list[i] = SEE_PIECE_VALUES[current_attacker] - swap_list[i - 1];
            if swap_list[i].max(swap_list[i - 1]) < 0 {
                break;
            }

            // remove the attacker from the masks (perform the capture)
            occupied ^= attacker_mask;

            // consider diagonal x-rays
            if current_attacker == Pawn || current_attacker == Bishop || current_attacker == Queen {
                attackers |= lookup_bishop(target, occupied) & bishops;
            }
            // consider orthogonal x-rays
            if current_attacker == Rook || current_attacker == Queen {
                attackers |= lookup_rook(target, occupied) & rooks;
            }

            // remove used attacks
            attackers &= occupied;
            if attackers.is_empty() {
                break;
            }
            for p in PIECES {
                if (attackers & self.piece_masks[(color, p)]).is_not_empty() {
                    current_attacker = p;
                    attacker_mask = (attackers & self.piece_masks[(color, p)])
                        .first_square()
                        .bitboard();
                    break;
                }
            }
            if attacker_mask.is_empty() {
                break;
            }
            color = !color;
        }

        i -= 1;
        while i != 0 {
            swap_list[i - 1] = -(swap_list[i].max(-swap_list[i - 1]));
            i -= 1;
        }
        swap_list[0]
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::{chessgame::ChessGame, moves::Move};

    #[test]
    fn test_see() -> Result<(), Box<dyn Error>> {
        let test_cases = [
            (
                "1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1",
                "d3e5",
                -200,
            ),
            ("4k3/1n6/8/2n5/3P4/8/8/4K3 w - - 0 1", "d4c5", 200),
            (
                "q2n4/1b1p4/3k4/2pP4/4B3/5B2/6B1/2Q1K3 w - c6 0 1",
                "d5c6",
                0,
            ),
            ("2kr4/8/8/8/2pP4/8/3K4/5Q2 b - d3 0 1", "c4d3", 100),
            ("4k3/8/8/4r3/5P2/8/8/4K3 w - - 0 1", "f4e5", 500),
            ("4k3/8/3p4/4n3/8/4R3/8/4K3 w - - 0 1", "e3e5", -200),
            ("4k3/8/1p1p4/2p5/3P4/8/2R5/4K3 w - - 0 1", "d4c5", 0),
            ("4k3/8/1q1p4/2p5/3P4/8/2R5/4K3 w - - 0 1", "d4c5", 0),
            ("4k3/8/1q1p4/2p5/3P4/8/2R5/4K3 w - - 0 1", "c2c5", -400),
            ("4k3/8/1q1p4/2p5/3P4/8/2R5/2Q1K3 w - - 0 1", "d4c5", 100),
            ("4k3/8/1b1p4/2p5/3P4/4B3/5B2/4K3 w - - 0 1", "d4c5", 100),
            ("3k3/8/1b1p4/2p5/3P4/4Q3/5B2/4K3 w - - 0 1", "d4c5", 0),
            ("8/8/8/2pk4/3P4/4P3/8/4K3 b - - 0 1", "c5d4", 100),
        ];
        for (fen, move_, score) in test_cases {
            let mut game = ChessGame::new();
            game.set_from_fen(fen)?;
            assert_eq!(game.see(Move::from_pair(&game, move_)), score);
        }
        Ok(())
    }
}
