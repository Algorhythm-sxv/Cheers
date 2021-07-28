use crate::{
    bitboard::BitBoards,
    piece_tables::tables::*,
    types::{ColorIndex, PieceIndex},
    utils::flip_square,
};
use ColorIndex::*;
use PieceIndex::*;

struct PieceValues([i32; 5]);
impl std::ops::Index<PieceIndex> for PieceValues {
    type Output = i32;

    fn index(&self, index: PieceIndex) -> &Self::Output {
        &self.0[index as usize]
    }
}
// piece values shamelessly stolen from stockfish midgame numbers
const PIECE_VALUES: PieceValues = PieceValues([
    198,  // pawn
    817,  // knight
    836,  // bishop
    1270, // rook
    2521, // queen
]);

impl BitBoards {
    /// Static evaluation of the board position, positive value for player advantage, negative for opponent advantage
    pub fn evaluate(&self, color: ColorIndex) -> i32 {
        let mut result = 0;

        result += self.material_count(color) - self.material_count(!color);

        let placement = self.piece_placement();

        result += placement[color as usize] - placement[!color as usize];

        result
    }

    fn material_count(&self, color: ColorIndex) -> i32 {
        let mut result = 0;

        result += (self.piece_masks[Pawn] & self.color_masks[color]).count_ones() as i32
            * PIECE_VALUES[Pawn];
        result += (self.piece_masks[Knight] & self.color_masks[color]).count_ones() as i32
            * PIECE_VALUES[Knight];
        result += (self.piece_masks[Bishop] & self.color_masks[color]).count_ones() as i32
            * PIECE_VALUES[Bishop];
        result += (self.piece_masks[Rook] & self.color_masks[color]).count_ones() as i32
            * PIECE_VALUES[Rook];
        result += (self.piece_masks[Queen] & self.color_masks[color]).count_ones() as i32
            * PIECE_VALUES[Queen];

        result
    }

    fn piece_placement(&self) -> [i32; 2] {
        let mut result = [0, 0];
        self.piece_list
            .iter()
            .enumerate()
            .for_each(|(square, piece_opt)| {
                if let Some((piece, color)) = piece_opt {
                    result[*color as usize] += PIECE_TABLES[*piece][if *color == White {
                        square
                    } else {
                        flip_square(square)
                    }]
                }
            });

        result
    }
}
