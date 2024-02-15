use crate::lookup_tables::*;
use crate::moves::*;
use crate::types::*;

use cheers_bitboards::BitBoard;
use Piece::*;

use super::Board;

pub const SEE_WINNING_SCORE: i16 = 10000;
pub const SEE_PIECE_VALUES: [i16; 6] = [100, 300, 300, 500, 900, 20000];
pub const MVV_LVA: [[i16; 6]; 6] = [
    // pawn captured
    [15, 14, 13, 12, 11, 10],
    // knight captured
    [25, 24, 23, 22, 21, 20],
    // bishop captured
    [35, 34, 33, 32, 31, 30],
    // rook captured
    [45, 44, 43, 42, 41, 40],
    // queen captured
    [55, 54, 53, 52, 51, 50],
    // king captured (never happens)
    [0, 0, 0, 0, 0, 0],
];

impl Board {
    pub fn see(&self, mv: Move) -> i16 {
        let target = mv.to();
        let mut swap_list = [0i16; 32];

        let mut current_attacker = mv.piece();
        let mut attacker_mask = mv.from().bitboard();

        let bishops =
            self.white_bishops | self.black_bishops | self.white_queens | self.black_queens;

        let rooks = self.white_rooks | self.black_rooks | self.white_queens | self.black_queens;

        // simulate the first capture
        swap_list[0] = match self.piece_on(target) {
            Some(p) => SEE_PIECE_VALUES[p],
            None => 0,
        };

        // simulate the first promotion
        if mv.promotion() != Pawn {
            current_attacker = mv.promotion();
            swap_list[0] += SEE_PIECE_VALUES[mv.promotion()] - SEE_PIECE_VALUES[Pawn];
        }

        let mut occupied = self.occupied;
        let mut color = !self.black_to_move;

        // correct for en passent capture
        if mv.piece() == Pawn && mv.to().bitboard() == self.ep_mask {
            // shift the pawn back to the normal square for en passent
            occupied ^= self.ep_mask | (self.ep_mask >> 8 << (16 * (self.black_to_move as u8)));
            swap_list[0] = SEE_PIECE_VALUES[Pawn];
        }

        let mut attackers = self.all_attacks_on(target, occupied);

        let mut i = 0;
        for _ in 1..32 {
            i += 1;
            let promotion = current_attacker == Pawn && (matches!(target.rank(), 0 | 7));
            let piece_value = if promotion {
                // correct the value of the last piece
                swap_list[i - 1] += SEE_PIECE_VALUES[Queen] - SEE_PIECE_VALUES[Pawn];
                SEE_PIECE_VALUES[Queen]
            } else {
                SEE_PIECE_VALUES[current_attacker]
            };
            swap_list[i] = piece_value - swap_list[i - 1];
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
            attacker_mask = BitBoard::empty();
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

    pub fn see_beats_threshold(&self, mv: Move, threshold: i16) -> bool {
        // correct for ep capture
        let mut value = if mv.piece() == Pawn && mv.to().bitboard() == self.ep_mask {
            SEE_PIECE_VALUES[Pawn] - threshold
        } else {
            // account for initial capture and promotion
            self.piece_on(mv.to())
                .map(|p| SEE_PIECE_VALUES[p])
                .unwrap_or(0)
                + SEE_PIECE_VALUES[mv.promotion()]
                - SEE_PIECE_VALUES[Pawn]
                - threshold
        };

        // if the initial capture doesn't beat the threshold then we fail early,
        // the opponent can simply not recapture
        if value < 0 {
            return false;
        }

        value -= SEE_PIECE_VALUES[mv.piece()];

        // if we still beat the threshold after the first recapture we succeed early
        if value >= 0 {
            return true;
        }

        let mut occupied = self.occupied ^ mv.from().bitboard();
        // remove ep pawn
        if mv.piece() == Pawn && mv.to().bitboard() == self.ep_mask {
            occupied &= ((self.ep_mask << 8) | (self.ep_mask >> 8)).inverse();
        }
        let mut attackers = self.all_attacks_on(mv.to(), occupied);

        let bishops =
            self.white_bishops | self.black_bishops | self.white_queens | self.black_queens;

        let rooks = self.white_rooks | self.black_rooks | self.white_queens | self.black_queens;

        let mut color = !self.black_to_move;

        loop {
            // remove used pieces from attackers
            attackers &= occupied;

            let (current_pieces, other_pieces) = if color {
                (self.black_pieces, self.white_pieces)
            } else {
                (self.white_pieces, self.black_pieces)
            };
            let current_attackers = attackers & current_pieces;

            // current color has no attackers left
            if current_attackers.is_empty() {
                break;
            }

            // find the least valuable piece to take with
            let piece = *PIECES
                .iter()
                .find(|p| {
                    let pieces = self.piece_mask(color, **p);
                    (pieces & current_attackers).is_not_empty()
                })
                .expect("SEE: no current piece found!");

            let piece_mask = self.piece_mask(color, piece);

            color = !color;

            // negamax the score
            value = -value - SEE_PIECE_VALUES[piece] - 1;

            if value >= 0 {
                // if the last capture was with king and it would be in check then fail instead of pass
                // from this color's perspective
                if piece == King && (attackers & other_pieces).is_not_empty() {
                    color = !color;
                }
                break;
            }

            // remove the last used piece from occupied
            occupied ^= (current_attackers & piece_mask).first_square().bitboard();

            // add discovered attacks from behind sliders
            if matches!(piece, Pawn | Bishop | Queen) {
                attackers |= lookup_bishop(mv.to(), occupied) & bishops;
            }

            if matches!(piece, Rook | Queen) {
                attackers |= lookup_rook(mv.to(), occupied) & rooks;
            }
        }

        // pass if opponent ran out of attackers or not recapturing wins material
        // fail if current player ran out of attackers or opponent has won material
        color != self.black_to_move
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
            ("1k1rr3/8/8/8/8/8/3Q4/K2R4 w - - 0 1", "d2d8", 100),
            (
                "rnbqk1nr/pppp1ppp/8/2b1p3/2B1P3/8/PPPP1PPP/RNBQK1NR w KQkq - 0 1",
                "c4f7",
                -200,
            ),
            ("6k1/8/8/RrRrP3/8/8/8/6K1 b - - 0 1", "d5e5", -400),
            (
                "6k1/1pp4p/p1pb4/6q1/3P1pRr/2P4P/PP1Br1P1/5RKN w - - 0 1",
                "f1f4",
                -100,
            ),
            (
                "5rk1/1pp2q1p/p1pb4/8/3P1NP1/2P5/1P1BQ1P1/5RK1 b - - 0 1",
                "d6f4",
                0,
            ),
            (
                "4R3/2r3p1/5bk1/1p1r3p/p2PR1P1/P1BK1P2/1P6/8 b - - 0 1",
                "h5g4",
                0,
            ),
            (
                "4R3/2r3p1/5bk1/1p1r1p1p/p2PR1P1/P1BK1P2/1P6/8 b - - 0 1",
                "h5g4",
                0,
            ),
            (
                "4r1k1/5pp1/nbp4p/1p2p2q/1P2P1b1/1BP2N1P/1B2QPPK/3R4 b - - 0 1",
                "g4f3",
                0,
            ),
            (
                "2r1r1k1/pp1bppbp/3p1np1/q3P3/2P2P2/1P2B3/P1N1B1PP/2RQ1RK1 b - - 0 1",
                "d6e5",
                100,
            ),
            (
                "7r/5qpk/p1Qp1b1p/3r3n/BB3p2/5p2/P1P2P2/4RK1R w - - 0 1",
                "e1e8",
                0,
            ),
            (
                "6rr/6pk/p1Qp1b1p/2n5/1B3p2/5p2/P1P2P2/4RK1R w - - 0 1",
                "e1e8",
                -500,
            ),
            (
                "7r/5qpk/2Qp1b1p/1N1r3n/BB3p2/5p2/P1P2P2/4RK1R w - - 0 1",
                "e1e8",
                -500,
            ),
            ("6RR/4bP2/8/8/5r2/3K4/5p2/4k3 w - - 0 1", "f7f8q", 200),
            ("6RR/4bP2/8/8/5r2/3K4/5p2/4k3 w - - 0 1", "f7f8n", 200),
            ("7R/5P2/8/8/6r1/3K4/5p2/4k3 w - - 0 1", "f7f8q", 800),
            ("7R/5P2/8/8/6r1/3K4/5p2/4k3 w - - 0 1", "f7f8b", 200),
            ("7R/4bP2/8/8/1q6/3K4/5p2/4k3 w - - 0 1", "f7f8r", -100),
            (
                "8/4kp2/2npp3/1Nn5/1p2PQP1/7q/1PP1B3/4KR1r b - - 0 1",
                "h1f1",
                0,
            ),
            (
                "8/4kp2/2npp3/1Nn5/1p2P1P1/7q/1PP1B3/4KR1r b - - 0 1",
                "h1f1",
                0,
            ),
            (
                "2r2r1k/6bp/p7/2q2p1Q/3PpP2/1B6/P5PP/2RR3K b - - 0 1",
                "c5c1",
                100,
            ),
            (
                "r2qk1nr/pp2ppbp/2b3p1/2p1p3/8/2N2N2/PPPP1PPP/R1BQR1K1 w kq - 0 1",
                "f3e5",
                100,
            ),
            (
                "6r1/4kq2/b2p1p2/p1pPb3/p1P2B1Q/2P4P/2B1R1P1/6K1 w - - 0 1",
                "f4e5",
                0,
            ),
            (
                "3q2nk/pb1r1p2/np6/3P2Pp/2p1P3/2R4B/PQ3P1P/3R2K1 w - h6 0 1",
                "g5h6",
                0,
            ),
            (
                "3q2nk/pb1r1p2/np6/3P2Pp/2p1P3/2R1B2B/PQ3P1P/3R2K1 w - h6 0 1",
                "g5h6",
                100,
            ),
            (
                "2r4r/1P4pk/p2p1b1p/7n/BB3p2/2R2p2/P1P2P2/4RK2 w - - 0 1",
                "c3c8",
                500,
            ),
            (
                "2r5/1P4pk/p2p1b1p/5b1n/BB3p2/2R2p2/P1P2P2/4RK2 w - - 0 1",
                "c3c8",
                500,
            ),
            (
                "2r4k/2r4p/p7/2b2p1b/4pP2/1BR5/P1R3PP/2Q4K w - - 0 1",
                "c3c5",
                300,
            ),
            (
                "8/pp6/2pkp3/4bp2/2R3b1/2P5/PP4B1/1K6 w - - 0 1",
                "g2c6",
                -200,
            ),
            (
                "4q3/1p1pr1k1/1B2rp2/6p1/p3PP2/P3R1P1/1P2R1K1/4Q3 b - - 0 1",
                "e6e4",
                -400,
            ),
            (
                "4q3/1p1pr1kb/1B2rp2/6p1/p3PP2/P3R1P1/1P2R1K1/4Q3 b - - 0 1",
                "h7e4",
                100,
            ),
            (
                "3r3k/3r4/2n1n3/8/3p4/2PR4/1B1Q4/3R3K w - - 0 1",
                "d3d4",
                -100,
            ),
            (
                "1k1r4/1ppn3p/p4b2/4n3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1",
                "d3e5",
                100,
            ),
            (
                "1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - - 0 1",
                "d3e5",
                -200,
            ),
            (
                "rnb2b1r/ppp2kpp/5n2/4P3/q2P3B/5R2/PPP2PPP/RN1QKB2 w Q - 0 1",
                "h4f6",
                100,
            ),
            (
                "r2q1rk1/2p1bppp/p2p1n2/1p2P3/4P1b1/1nP1BN2/PP3PPP/RN1QR1K1 b - - 0 1",
                "g4f3",
                0,
            ),
            (
                "r1bqkb1r/2pp1ppp/p1n5/1p2p3/3Pn3/1B3N2/PPP2PPP/RNBQ1RK1 b kq - 0 1",
                "c6d4",
                0,
            ),
            (
                "r1bq1r2/pp1ppkbp/4N1p1/n3P1B1/8/2N5/PPP2PPP/R2QK2R w KQ - 0 1",
                "e6g7",
                0,
            ),
            (
                "r1bq1r2/pp1ppkbp/4N1pB/n3P3/8/2N5/PPP2PPP/R2QK2R w KQ - 0 1",
                "e6g7",
                300,
            ),
            (
                "rnq1k2r/1b3ppp/p2bpn2/1p1p4/3N4/1BN1P3/PPP2PPP/R1BQR1K1 b kq - 0 1",
                "d6h2",
                -200,
            ),
            (
                "rn2k2r/1bq2ppp/p2bpn2/1p1p4/3N4/1BN1P3/PPP2PPP/R1BQR1K1 b kq - 0 1",
                "d6h2",
                100,
            ),
            (
                "r2qkbn1/ppp1pp1p/3p1rp1/3Pn3/4P1b1/2N2N2/PPP2PPP/R1BQKB1R b KQq - 0 1",
                "g4f3",
                100,
            ),
            (
                "rnbq1rk1/pppp1ppp/4pn2/8/1bPP4/P1N5/1PQ1PPPP/R1B1KBNR b KQ - 0 1",
                "b4c3",
                0,
            ),
            (
                "r4rk1/3nppbp/bq1p1np1/2pP4/8/2N2NPP/PP2PPB1/R1BQR1K1 b - - 0 1",
                "b6b2",
                -800,
            ),
            (
                "r4rk1/1q1nppbp/b2p1np1/2pP4/8/2N2NPP/PP2PPB1/R1BQR1K1 b - - 0 1",
                "f6d5",
                -200,
            ),
            (
                "1r3r2/5p2/4p2p/2k1n1P1/2PN1nP1/1P3P2/8/2KR1B1R b - - 0 1",
                "b8b3",
                -400,
            ),
            (
                "1r3r2/5p2/4p2p/4n1P1/kPPN1nP1/5P2/8/2KR1B1R b - - 0 1",
                "b8b4",
                100,
            ),
            (
                "2r2rk1/5pp1/pp5p/q2p4/P3n3/1Q3NP1/1P2PP1P/2RR2K1 b - - 0 1",
                "c8c1",
                0,
            ),
            (
                "5rk1/5pp1/2r4p/5b2/2R5/6Q1/R1P1qPP1/5NK1 b - - 0 1",
                "f5c2",
                -100,
            ),
            (
                "1r3r1k/p4pp1/2p1p2p/qpQP3P/2P5/3R4/PP3PP1/1K1R4 b - - 0 1",
                "a5a2",
                -800,
            ),
            (
                "1r5k/p4pp1/2p1p2p/qpQP3P/2P2P2/1P1R4/P4rP1/1K1R4 b - - 0 1",
                "a5a2",
                100,
            ),
            (
                "r2q1rk1/1b2bppp/p2p1n2/1ppNp3/3nP3/P2P1N1P/BPP2PP1/R1BQR1K1 w - - 0 1",
                "d5e7",
                0,
            ),
            (
                "rnbqrbn1/pp3ppp/3p4/2p2k2/4p3/3B1K2/PPP2PPP/RNB1Q1NR w - - 0 1",
                "d3e4",
                100,
            ),
            (
                "rnb1k2r/p3p1pp/1p3p1b/7n/1N2N3/3P1PB1/PPP1P1PP/R2QKB1R w KQkq - 0 1",
                "e4d6",
                -200,
            ),
            (
                "r1b1k2r/p4npp/1pp2p1b/7n/1N2N3/3P1PB1/PPP1P1PP/R2QKB1R w KQkq - 0 1",
                "e4d6",
                0,
            ),
            (
                "2r1k2r/pb4pp/5p1b/2KB3n/4N3/2NP1PB1/PPP1P1PP/R2Q3R w k - 0 1",
                "d5c6",
                -300,
            ),
            (
                "2r1k2r/pb4pp/5p1b/2KB3n/1N2N3/3P1PB1/PPP1P1PP/R2Q3R w k - 0 1",
                "d5c6",
                0,
            ),
            (
                "2r1k3/pbr3pp/5p1b/2KB3n/1N2N3/3P1PB1/PPP1P1PP/R2Q3R w - - 0 1",
                "d5c6",
                -300,
            ),
            (
                "5k2/p2P2pp/8/1pb5/1Nn1P1n1/6Q1/PPP4P/R3K1NR w KQ - 0 1",
                "d7d8q",
                800,
            ),
            (
                "r4k2/p2P2pp/8/1pb5/1Nn1P1n1/6Q1/PPP4P/R3K1NR w KQ - 0 1",
                "d7d8q",
                -100,
            ),
            (
                "5k2/p2P2pp/1b6/1p6/1Nn1P1n1/8/PPP4P/R2QK1NR w KQ - 0 1",
                "d7d8q",
                200,
            ),
            (
                "4kbnr/p1P1pppp/b7/4q3/7n/8/PP1PPPPP/RNBQKBNR w KQk - 0 1",
                "c7c8q",
                -100,
            ),
            (
                "4kbnr/p1P1pppp/b7/4q3/7n/8/PPQPPPPP/RNB1KBNR w KQk - 0 1",
                "c7c8q",
                200,
            ),
            (
                "4kbnr/p1P1pppp/b7/4q3/7n/8/PPQPPPPP/RNB1KBNR w KQk - 0 1",
                "c7c8q",
                200,
            ),
            (
                "4kbnr/p1P4p/b1q5/5pP1/4n3/5Q2/PP1PPP1P/RNB1KBNR w KQk f6 0 1",
                "g5f6",
                0,
            ),
            (
                "4kbnr/p1P4p/b1q5/5pP1/4n3/5Q2/PP1PPP1P/RNB1KBNR w KQk f6 0 1",
                "g5f6",
                0,
            ),
            (
                "4kbnr/p1P4p/b1q5/5pP1/4n2Q/8/PP1PPP1P/RNB1KBNR w KQk f6 0 1",
                "g5f6",
                0,
            ),
            (
                "1n2kb1r/p1P4p/2qb4/5pP1/4n2Q/8/PP1PPP1P/RNB1KBNR w KQk - 0 1",
                "c7b8q",
                200,
            ),
            (
                "rnbqk2r/pp3ppp/2p1pn2/3p4/3P4/N1P1BN2/PPB1PPPb/R2Q1RK1 w kq - 0 1",
                "g1h2",
                300,
            ),
            ("3N4/2K5/2n5/1k6/8/8/8/8 b - - 0 1", "c6d8", 0),
            ("3N4/2P5/2n5/1k6/8/8/8/4K3 b - - 0 1", "c6d8", -800),
            ("3n3r/2P5/8/1k6/8/8/3Q4/4K3 w - - 0 1", "d2d8", 300),
            ("3n3r/2P5/8/1k6/8/8/3Q4/4K3 w - - 0 1", "c7d8q", 700),
            ("r2n3r/2P1P3/4N3/1k6/8/8/8/4K3 w - - 0 1", "e6d8", 300),
            ("8/8/8/1k6/6b1/4N3/2p3K1/3n4 w - - 0 1", "e3d1", -800),
            ("8/8/1k6/8/8/2N1N3/4p1K1/3n4 w - - 0 1", "c3d1", 100),
        ];
        for (fen, move_, score) in test_cases {
            let game = Board::from_fen(fen).unwrap();
            let see = game.see(Move::from_pair(&game, move_));
            if see != score {
                panic!("Expected SEE {score} for {fen}, move {move_}\ngot {see}");
            }
            // let beats_eq = game.see_beats_threshold(Move::from_pair(&game, move_), score);
            // if !beats_eq {
            //     panic!("Expected {fen}, move {move_} to pass threshold {score}");
            // }
            // let beats_lt = game.see_beats_threshold(Move::from_pair(&game, move_), score - 10);
            // if !beats_lt {
            //     panic!(
            //         "Expected {fen}, move {move_} to pass threshold {}",
            //         score - 10
            //     );
            // }
            // let beats_gt = game.see_beats_threshold(Move::from_pair(&game, move_), score + 10);
            // if beats_gt {
            //     panic!(
            //         "Expected {fen}, move {move_} to fail threshold {}",
            //         score + 10
            //     );
            // }
        }
        Ok(())
    }
}
