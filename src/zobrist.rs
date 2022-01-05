use once_cell::sync::OnceCell;
use rand::prelude::*;

use crate::{
    bitboards::*,
    types::{CastlingRights, ColorIndex, PieceIndex},
};

pub static ZOBRIST_NUMBERS: OnceCell<Vec<u64>> = OnceCell::new();

pub fn zobrist_numbers() -> &'static Vec<u64> {
    ZOBRIST_NUMBERS.get_or_init(initialise_zobrist_numbers)
}
fn initialise_zobrist_numbers() -> Vec<u64> {
    let mut rng = StdRng::seed_from_u64(0x11A5117AB1E0);
    let mut numbers = vec![0; 64 * 6 * 2 + 1 + 16 + 8];

    rng.fill(&mut numbers[..]);
    // for the case when the en passent mask is zero and not changing, x^0 = x
    // let last = numbers.len() - 1;
    // numbers[last] = 0;

    numbers
}

pub fn zobrist_piece(piece: PieceIndex, color: ColorIndex, square: usize) -> u64 {
    zobrist_numbers()[64 * 2 * (piece as usize) + 64 * (color as usize) + square]
}

pub fn zobrist_player() -> u64 {
    zobrist_numbers()[64 * 6 * 2]
}

pub fn zobrist_castling(rights: CastlingRights) -> u64 {
    let index = (rights.0[0][0] as usize)
        + ((rights.0[0][1] as usize) << 1)
        + ((rights.0[1][0] as usize) << 2)
        + ((rights.0[1][1] as usize) << 3);
    zobrist_numbers()[64 * 6 * 2 + 1 + index]
}

pub fn zobrist_enpassent(mask: u64) -> u64 {
    zobrist_numbers()[64 * 6 * 2 + 1 + 16 + mask.trailing_zeros() as usize % 8]
}
