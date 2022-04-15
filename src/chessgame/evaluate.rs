use crate::types::PieceIndex;
use piece_tables::*;
use GamePhase::*;

use super::*;

pub const PIECE_VALUES: PieceValues = PieceValues([
    100,   // pawn value
    320,   // knight value
    350,   // bishop value
    500,   // rook value
    900,   // queen value
    20000, // king value
]);

pub const CHECKMATE_SCORE: i32 = 20000;
pub const DRAW_SCORE: i32 = 0;
pub const PAWN_SHIELD_1: i32 = 50;
pub const PAWN_SHIELD_2: i32 = 35;

pub struct PieceValues([i32; 6]);

impl std::ops::Index<PieceIndex> for PieceValues {
    type Output = i32;
    fn index(&self, index: PieceIndex) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl ChessGame {
    pub fn evaluate(&self) -> i32 {
        let mut eval = 0i32;
        let mut midgame = 0i32;
        let mut endgame = 0i32;

        eval += self.material_difference();
        eval +=
            self.piece_mobility(self.current_player) - self.piece_mobility(!self.current_player);

        eval +=
            self.pawn_structure(self.current_player) - self.pawn_structure(!self.current_player);

        midgame += self.piece_placement(Midgame);
        midgame += self.pawn_shield(self.current_player) - self.pawn_shield(!self.current_player);

        endgame += self.piece_placement(Endgame);

        let phase = self.game_phase();

        eval += ((midgame * (256 - phase)) + (endgame * phase)) / 256;

        eval
    }

    pub fn game_phase(&self) -> i32 {
        let knight_phase = 1;
        let bishop_phase = 1;
        let rook_phase = 2;
        let queen_phase = 4;

        let total_phase = knight_phase * 4 + bishop_phase * 4 + rook_phase * 4 + queen_phase * 2;

        let mut phase: i32 = total_phase;

        phase -= (self.piece_masks[(White, Knight)] | self.piece_masks[(Black, Knight)])
            .count_ones() as i32
            * knight_phase;
        phase -= (self.piece_masks[(White, Bishop)] | self.piece_masks[(Black, Bishop)])
            .count_ones() as i32
            * bishop_phase;
        phase -= (self.piece_masks[(White, Rook)] | self.piece_masks[(Black, Rook)]).count_ones()
            as i32
            * rook_phase;
        phase -= (self.piece_masks[(White, Queen)] | self.piece_masks[(Black, Queen)]).count_ones()
            as i32
            * queen_phase;

        (phase * 256 + (total_phase / 2)) / total_phase
    }

    pub fn material_difference(&self) -> i32 {
        let mut sum = 0i32;
        let color = self.current_player;
        for piece in [Pawn, Knight, Bishop, Rook, Queen] {
            sum += (self.piece_masks[(color, piece)].count_ones() as i32
                - self.piece_masks[(!color, piece)].count_ones() as i32)
                * PIECE_VALUES[piece];
        }
        sum
    }

    pub fn piece_placement(&self, phase: GamePhase) -> i32 {
        let mut sum = 0i32;
        let color = self.current_player;
        for piece in [Pawn, Knight, Bishop, Rook, Queen, King] {
            let player_pieces = self.piece_masks[(color, piece)];

            for square in player_pieces {
                let player_index = if color == White { square ^ 56 } else { square } as usize;

                sum += PIECE_TABLES[(phase, piece)][player_index];
            }
            let opponent_pieces = self.piece_masks[(!color, piece)];

            for square in opponent_pieces {
                let opponent_index = if !color == White { square ^ 56 } else { square } as usize;

                sum -= PIECE_TABLES[(phase, piece)][opponent_index];
            }
        }
        sum
    }

    pub fn pawn_shield(&self, color: ColorIndex) -> i32 {
        let mut sum = 0;
        let rank_1 = if color == White {
            SECOND_RANK
        } else {
            SEVENTH_RANK
        };
        let rank_2 = if color == White {
            THIRD_RANK
        } else {
            SIXTH_RANK
        };

        if self.piece_masks[(color, King)].lsb_index() % 8 <= 2 {
            // castled queenside
            let shield_pawns = self.piece_masks[(color, Pawn)] & (A_FILE | B_FILE | C_FILE);
            sum += (shield_pawns & rank_1).count_ones() as i32 * PAWN_SHIELD_1
                + (shield_pawns & rank_2).count_ones() as i32 * PAWN_SHIELD_2;
        } else if self.piece_masks[(color, King)].lsb_index() % 8 >= 5 {
            // castled kingside
            let shield_pawns = self.piece_masks[(color, Pawn)] & (F_FILE | G_FILE | H_FILE);
            sum += (shield_pawns & rank_1).count_ones() as i32 * PAWN_SHIELD_1
                + (shield_pawns & rank_2).count_ones() as i32 * PAWN_SHIELD_2;
        }
        sum
    }

    pub fn piece_mobility(&self, color: ColorIndex) -> i32 {
        let mut mobility = 0i32;

        mobility += self.knight_attacks(color).count_ones() as i32;
        mobility += self
            .bishop_attacks(color, self.color_masks[White] | self.color_masks[Black])
            .count_ones() as i32;
        mobility += self
            .rook_attacks(color, self.color_masks[White] | self.color_masks[Black])
            .count_ones() as i32;
        mobility += self
            .queen_attacks(color, self.color_masks[White] | self.color_masks[Black])
            .count_ones() as i32;

        mobility
    }

    pub fn pawn_structure(&self, color: ColorIndex) -> i32 {
        let mut sum = 0i32;

        // passed pawns: +100
        let front_spans = self.pawn_front_spans(!color);
        let all_front_spans =
            front_spans | (front_spans & NOT_H_FILE) << 1 | (front_spans & NOT_A_FILE) >> 1;
        let passers = self.piece_masks[(color, Pawn)] & all_front_spans.inverse();
        sum += 100 * passers.count_ones() as i32;

        // double pawns: -40
        for file in FILES {
            sum -= 40
                * ((self.piece_masks[(color, Pawn)] & file)
                    .count_ones()
                    .saturating_sub(1) as i32);
        }

        // backward pawns: -10?
        // isolated pawns: -20?

        sum
    }
}
