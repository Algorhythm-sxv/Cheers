use crate::types::{CastlingRights, ColorIndex, PieceIndex};
use cheers_bitboards::{BitBoard, Square};

use cheers_pregen::ZOBRIST_NUMBERS;

pub fn zobrist_piece(piece: PieceIndex, color: ColorIndex, square: Square) -> u64 {
    unsafe {
        *ZOBRIST_NUMBERS
            .get_unchecked(64 * 2 * (piece as usize) + 64 * (color as usize) + *square as usize)
    }
}

pub fn zobrist_player() -> u64 {
    unsafe { *ZOBRIST_NUMBERS.get_unchecked(64 * 6 * 2) }
}

pub fn zobrist_castling(rights: CastlingRights) -> u64 {
    let index = (rights.0[0][0] as usize)
        + ((rights.0[0][1] as usize) << 1)
        + ((rights.0[1][0] as usize) << 2)
        + ((rights.0[1][1] as usize) << 3);
    unsafe { *ZOBRIST_NUMBERS.get_unchecked(64 * 6 * 2 + 1 + index) }
}

pub fn zobrist_enpassent(mask: BitBoard) -> u64 {
    unsafe { *ZOBRIST_NUMBERS.get_unchecked(64 * 6 * 2 + 1 + 16 + mask.first_square().file()) }
}
