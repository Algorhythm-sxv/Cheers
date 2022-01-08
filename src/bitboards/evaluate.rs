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

pub const CHECKMATE_SCORE: i32 = PIECE_VALUES[King];
pub const DRAW_SCORE: i32 = 0;

pub struct PieceValues([i32; 6]);

impl std::ops::Index<PieceIndex> for PieceValues {
    type Output = i32;
    fn index(&self, index: PieceIndex) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl BitBoards {
    pub fn evaluate(&self) -> i32 {
        let mut sum = 0i32;

        sum += self.material_difference();
        sum += self.piece_placement();

        sum
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

    pub fn piece_placement(&self) -> i32 {
        let mut sum = 0i32;
        let color = self.current_player;
        for piece in [Pawn, Knight, Bishop, Rook, Queen, King] {
            let mut player_pieces = self.piece_masks[(color, piece)];

            while player_pieces != 0 {
                let square = player_pieces.trailing_zeros() as usize;
                let player_index = if color == White { square ^ 56 } else { square };

                sum += PIECE_TABLES[(Midgame, piece)][player_index];

                player_pieces ^= 1 << square;
            }
            let mut opponent_pieces = self.piece_masks[(!color, piece)];

            while opponent_pieces != 0 {
                let square = opponent_pieces.trailing_zeros() as usize;
                let opponent_index = if !color == White { square ^ 56 } else { square };

                sum -= PIECE_TABLES[(Midgame, piece)][opponent_index];

                opponent_pieces ^= 1 << square;
            }
        }
        sum
    }
}
