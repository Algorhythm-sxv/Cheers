use crate::lookup_tables::*;
use crate::moves::*;
use crate::types::*;

use Piece::*;

use super::Board;

pub const SEE_PIECE_VALUES: [i32; 6] = [100, 300, 300, 500, 900, 200000];
pub const MVV_LVA: [[i16; 6]; 6] = [
    // pawn captured
    [15, 14, 13, 12, 11, 10],
    // knight captured
    [25, 24, 23, 22, 21, 20],
    // bishop captured
    [35, 34, 33, 32, 31, 30],
    // rook captured
    [45, 44, 43, 42, 41, 40],
    // queen captures
    [55, 54, 53, 52, 51, 50],
    // king captured (never happens)
    [0, 0, 0, 0, 0, 0],
];

impl Board {
    pub fn see(&self, mv: Move) -> i32 {
        let target = mv.to;
        let mut swap_list = [0i32; 32];

        let mut current_attacker = mv.piece;
        let mut attacker_mask = mv.from.bitboard();

        let bishops =
            self.white_bishops | self.black_bishops | self.white_queens | self.black_queens;

        let rooks = self.white_rooks | self.black_rooks | self.white_queens | self.black_queens;

        // simulate the first capture
        swap_list[0] = match self.piece_on(target) {
            Some(p) => SEE_PIECE_VALUES[p],
            None => 0,
        };
        let mut occupied = self.occupied;
        let mut color = !self.black_to_move;

        // correct for en passent capture
        if mv.piece == Pawn && mv.to.bitboard() == self.ep_mask {
            // shift the pawn back to the normal square for en passent
            occupied ^= self.ep_mask | (self.ep_mask >> 8 << 16 * (self.black_to_move as u8));
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
                let mask = if color {
                    match p {
                        Pawn => self.black_pawns,
                        Knight => self.black_knights,
                        Bishop => self.black_bishops,
                        Rook => self.black_rooks,
                        Queen => self.black_queens,
                        King => self.black_king,
                    }
                } else {
                    match p {
                        Pawn => self.white_pawns,
                        Knight => self.white_knights,
                        Bishop => self.white_bishops,
                        Rook => self.white_rooks,
                        Queen => self.white_queens,
                        King => self.white_king,
                    }
                };
                if (attackers & mask).is_not_empty() {
                    current_attacker = p;
                    attacker_mask = (attackers & mask).first_square().bitboard();
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

    use crate::{board::Board, moves::Move};

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
            let game = Board::from_fen(fen).unwrap();
            assert_eq!(game.see(Move::from_pair(&game, move_)), score);
        }
        Ok(())
    }
}
