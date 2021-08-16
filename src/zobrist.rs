use once_cell::sync::OnceCell;
use rand::prelude::*;

use crate::{
    bitboard::BitBoards,
    types::{CastlingRights, ColorIndex, PieceIndex},
};

pub static ZOBRIST_NUMBERS: OnceCell<Vec<u64>> = OnceCell::new();

pub fn zobrist_numbers() -> &'static Vec<u64> {
    ZOBRIST_NUMBERS.get_or_init(|| initialise_zobrist_numbers())
}
fn initialise_zobrist_numbers() -> Vec<u64> {
    let mut rng = StdRng::seed_from_u64(0x11A5117AB1E0);
    let mut numbers = vec![0; 64 * 6 * 2 + 1 + 16 + 8 + 1];

    rng.fill(&mut numbers[..]);
    // for the case when the en passent mask is zero and not changing, x^0 = x
    let last = numbers.len() - 1;
    numbers[last] = 0;

    numbers
}

pub fn zobrist_hash(bitboards: &BitBoards) -> u64 {
    let mut hash =
        bitboards
            .piece_list
            .iter()
            .enumerate()
            .fold(0u64, |hash, (square, square_opt)| {
                if let Some((piece, color)) = square_opt {
                    hash ^ zobrist_numbers()[zobrist_piece_index(*piece, *color, square)]
                } else {
                    hash
                }
            });

    hash ^= bitboards.current_player as u64 * zobrist_numbers()[zobrist_player_index()];
    hash ^= zobrist_numbers()[zobrist_castling_index(bitboards.castling_rights)];
    if bitboards.en_passent_mask != 0 {
        hash ^= zobrist_numbers()[zobrist_en_passent_index(bitboards.en_passent_mask)];
    }

    hash
}

pub fn zobrist_piece_index(piece: PieceIndex, color: ColorIndex, square: usize) -> usize {
    (square * 6 * 2) + (color as usize * 6) + piece as usize
}

pub fn zobrist_player_index() -> usize {
    64 * 6 * 2
}

pub fn zobrist_castling_index(castling_rights: CastlingRights) -> usize {
    let offset = (castling_rights.0[0][0] as usize)
        + ((castling_rights.0[0][1] as usize) << 1)
        + ((castling_rights.0[1][0] as usize) << 2)
        + ((castling_rights.0[1][1] as usize) << 3);

    64 * 6 * 2 + 1 + offset
}

pub fn zobrist_en_passent_index(mask: u64) -> usize {
    64 * 6 * 2 + 1 + 16 + mask.trailing_zeros() as usize % 8
}

pub trait ZobristHash {
    fn update_piece(&mut self, piece: PieceIndex, color: ColorIndex, square: usize);
    fn update_player(&mut self);
    fn update_en_passent(&mut self, mask: u64);
    fn update_castling(&mut self, castling_rights: CastlingRights);
}

impl ZobristHash for u64 {
    fn update_piece(&mut self, piece: PieceIndex, color: ColorIndex, square: usize) {
        *self ^= zobrist_numbers()[zobrist_piece_index(piece, color, square)];
    }
    fn update_player(&mut self) {
        *self ^= zobrist_numbers()[zobrist_player_index()];
    }
    fn update_en_passent(&mut self, mask: u64) {
        *self ^= zobrist_numbers()[zobrist_en_passent_index(mask)];
    }
    fn update_castling(&mut self, castling_rights: CastlingRights) {
        *self ^= zobrist_numbers()[zobrist_castling_index(castling_rights)];
    }
}
