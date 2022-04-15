use rand::prelude::*;

use crate::{types::{CastlingRights, ColorIndex, PieceIndex}, bitboard::BitBoard};

static mut ZOBRIST_NUMBERS: Vec<u64> = Vec::new();

pub fn initialise_zobrist_numbers() {
    let mut rng = StdRng::seed_from_u64(0x11A5117AB1E0);
    let mut numbers = vec![0; 64 * 6 * 2 + 1 + 16 + 8];

    rng.fill(&mut numbers[..]);
    // for the case when the en passent mask is zero and not changing, x^0 = x
    // let last = numbers.len() - 1;
    // numbers[last] = 0;

    unsafe { ZOBRIST_NUMBERS = numbers };
}

pub fn zobrist_piece(piece: PieceIndex, color: ColorIndex, square: usize) -> u64 {
    unsafe {
        *ZOBRIST_NUMBERS.get_unchecked(64 * 2 * (piece as usize) + 64 * (color as usize) + square)
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
    unsafe {
        *ZOBRIST_NUMBERS.get_unchecked(64 * 6 * 2 + 1 + 16 + mask.lsb_index() as usize % 8)
    }
}
