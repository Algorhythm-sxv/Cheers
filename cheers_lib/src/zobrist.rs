use crate::types::*;
use cheers_bitboards::*;
use cheers_pregen::*;

#[inline(always)]
pub fn zobrist_piece<T: TypeColor>(piece: Piece, square: Square) -> u64 {
    let color_offset = if T::WHITE { 0 } else { 64 };
    #[cfg(debug_assertions)]
    {
        ZOBRIST_NUMBERS[64 * 2 * piece as usize + color_offset + *square as usize]
    }
    #[cfg(not(debug_assertions))]
    unsafe {
        *ZOBRIST_NUMBERS.get_unchecked(64 * 2 * piece as usize + color_offset + *square as usize)
    }
}

#[inline(always)]
pub fn zobrist_player() -> u64 {
    #[cfg(debug_assertions)]
    {
        ZOBRIST_NUMBERS[64 * 6 * 2]
    }
    #[cfg(not(debug_assertions))]
    unsafe {
        *ZOBRIST_NUMBERS.get_unchecked(64 * 6 * 2)
    }
}

#[inline(always)]
pub fn zobrist_castling(rights: [[BitBoard; 2]; 2]) -> u64 {
    let index: usize = rights
        .iter()
        .flatten()
        .enumerate()
        .map(|(i, b)| (b.is_not_empty() as usize) << i)
        .sum();
    #[cfg(debug_assertions)]
    {
        ZOBRIST_NUMBERS[64 * 6 * 2 + 1 + index]
    }
    #[cfg(not(debug_assertions))]
    unsafe {
        *ZOBRIST_NUMBERS.get_unchecked(64 * 6 * 2 + 1 + index)
    }
}

#[inline(always)]
pub fn zobrist_ep(mask: BitBoard) -> u64 {
    #[cfg(debug_assertions)]
    {
        ZOBRIST_NUMBERS[64 * 6 * 2 + 1 + 16 + mask.first_square().file()]
    }
    #[cfg(not(debug_assertions))]
    unsafe {
        *ZOBRIST_NUMBERS.get_unchecked(64 * 6 * 2 + 1 + 16 + mask.first_square().file())
    }
}
