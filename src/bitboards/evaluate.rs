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

impl BitBoards {
    pub fn evaluate(&self) -> i32 {
        let mut eval = 0i32;
        let mut midgame = 0i32;
        let mut endgame = 0i32;

        eval += self.material_difference();

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
            let mut player_pieces = self.piece_masks[(color, piece)];

            while player_pieces != 0 {
                let square = player_pieces.trailing_zeros() as usize;
                let player_index = if color == White { square ^ 56 } else { square };

                sum += PIECE_TABLES[(phase, piece)][player_index];

                player_pieces ^= 1 << square;
            }
            let mut opponent_pieces = self.piece_masks[(!color, piece)];

            while opponent_pieces != 0 {
                let square = opponent_pieces.trailing_zeros() as usize;
                let opponent_index = if !color == White { square ^ 56 } else { square };

                sum -= PIECE_TABLES[(phase, piece)][opponent_index];

                opponent_pieces ^= 1 << square;
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

        if self.piece_masks[(color, King)].trailing_zeros() % 8 <= 2 {
            // castled queenside
            let shield_pawns = self.piece_masks[(color, Pawn)] & (A_FILE | B_FILE | C_FILE);
            sum += (shield_pawns & rank_1).count_ones() as i32 * PAWN_SHIELD_1
                + (shield_pawns & rank_2).count_ones() as i32 * PAWN_SHIELD_2;
        } else if self.piece_masks[(color, King)].trailing_zeros() % 8 >= 5 {
            // castled kingside
            let shield_pawns = self.piece_masks[(color, Pawn)] & (F_FILE | G_FILE | H_FILE);
            sum += (shield_pawns & rank_1).count_ones() as i32 * PAWN_SHIELD_1
                + (shield_pawns & rank_2).count_ones() as i32 * PAWN_SHIELD_2;
        }
        sum
    }
}
