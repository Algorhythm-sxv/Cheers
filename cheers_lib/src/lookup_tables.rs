use cheers_bitboards::*;
use cheers_pregen::*;

#[inline(always)]
pub fn lookup_knight(square: Square) -> BitBoard {
    #[cfg(debug_assertions)]
    {
        KNIGHT_TABLE[square]
    }
    #[cfg(not(debug_assertions))]
    unsafe {
        *KNIGHT_TABLE.get_unchecked(*square as usize)
    }
}

#[inline(always)]
pub fn lookup_bishop(square: Square, mask: BitBoard) -> BitBoard {
    #[cfg(debug_assertions)]
    {
        SLIDING_ATTACK_TABLE[bishop_attack_index(square, mask)]
    }
    #[cfg(not(debug_assertions))]
    unsafe {
        *SLIDING_ATTACK_TABLE.get_unchecked(bishop_attack_index(square, mask))
    }
}

#[inline(always)]
pub fn lookup_rook(square: Square, mask: BitBoard) -> BitBoard {
    #[cfg(debug_assertions)]
    {
        SLIDING_ATTACK_TABLE[rook_attack_index(square, mask)]
    }
    #[cfg(not(debug_assertions))]
    unsafe {
        *SLIDING_ATTACK_TABLE.get_unchecked(rook_attack_index(square, mask))
    }
}

#[inline(always)]
pub fn lookup_queen(square: Square, mask: BitBoard) -> BitBoard {
    #[cfg(debug_assertions)]
    {
        SLIDING_ATTACK_TABLE[rook_attack_index(square, mask)]
            | SLIDING_ATTACK_TABLE[bishop_attack_index(square, mask)]
    }
    #[cfg(not(debug_assertions))]
    unsafe {
        *SLIDING_ATTACK_TABLE.get_unchecked(rook_attack_index(square, mask))
            | *SLIDING_ATTACK_TABLE.get_unchecked(bishop_attack_index(square, mask))
    }
}

#[inline(always)]
pub fn lookup_king(square: Square) -> BitBoard {
    #[cfg(debug_assertions)]
    {
        KING_TABLE[square]
    }
    #[cfg(not(debug_assertions))]
    unsafe {
        *KING_TABLE.get_unchecked(*square as usize)
    }
}

#[inline(always)]
pub fn lookup_between(a: Square, b: Square) -> BitBoard {
    #[cfg(debug_assertions)]
    {
        BETWEEN[a][b]
    }
    #[cfg(not(debug_assertions))]
    unsafe {
        *BETWEEN
            .get_unchecked(*a as usize)
            .get_unchecked(*b as usize)
    }
}

pub fn adjacent_files(file: usize) -> BitBoard {
    match file {
        0 => B_FILE,
        1 => A_FILE | C_FILE,
        2 => B_FILE | D_FILE,
        3 => C_FILE | E_FILE,
        4 => D_FILE | F_FILE,
        5 => E_FILE | G_FILE,
        6 => F_FILE | H_FILE,
        7 => G_FILE,
        _ => unreachable!(),
    }
}
