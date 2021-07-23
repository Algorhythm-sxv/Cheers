use crate::types::{ColorIndex, ColorIndex::*};
use once_cell::sync::OnceCell;
use rand::prelude::*;

pub static LOOKUP_TABLES: OnceCell<LookupTables> = OnceCell::new();

pub fn lookup_tables() -> &'static LookupTables {
    LOOKUP_TABLES.get().expect("Lookup tables uninitialised")
}

#[derive(Clone, Default)]
pub struct LookupTables {
    knight_table: Vec<u64>,
    king_table: Vec<u64>,
    pawn_push_one_tables: [Vec<u64>; 2],
    pawn_attack_tables: [Vec<u64>; 2],
    sliding_attack_table: Vec<u64>,
    rook_magics: Vec<MagicSquare>,
    bishop_magics: Vec<MagicSquare>,
}

impl LookupTables {
    pub fn generate_all() -> &'static Self {
        let mut sliding_attack_table = Vec::with_capacity(10000);

        let rook_magics = generate_rook_magics(&mut sliding_attack_table);
        let bishop_magics = generate_bishop_magics(&mut &mut sliding_attack_table);

        LOOKUP_TABLES.get_or_init(|| Self {
            knight_table: generate_knight_table(),
            king_table: generate_king_table(),
            pawn_push_one_tables: generate_pawn_push_tables(),
            pawn_attack_tables: generate_pawn_attack_tables(),
            sliding_attack_table,
            rook_magics,
            bishop_magics,
        })
    }

    #[inline(always)]
    fn bishop_attack_index(&self, square: usize, blocking_mask: u64) -> usize {
        let magic_square = self.bishop_magics[square];
        magic_square.index
            + magic_hash(
                magic_square.mask & blocking_mask,
                magic_square.magic,
                magic_square.shift,
            )
    }

    #[inline(always)]
    fn rook_attack_index(&self, square: usize, blocking_mask: u64) -> usize {
        let magic_square = self.rook_magics[square];
        magic_square.index
            + magic_hash(
                magic_square.mask & blocking_mask,
                magic_square.magic,
                magic_square.shift,
            )
    }

    pub fn lookup_knight(&self, square: usize) -> u64 {
        self.knight_table[square]
    }

    pub fn lookup_king(&self, square: usize) -> u64 {
        self.king_table[square]
    }

    pub fn lookup_pawn_push(&self, square: usize, color: ColorIndex) -> u64 {
        self.pawn_push_one_tables[color as usize][square]
    }

    pub fn lookup_pawn_attack(&self, square: usize, color: ColorIndex) -> u64 {
        self.pawn_attack_tables[color as usize][square]
    }

    pub fn lookup_bishop(&self, square: usize, blocking_mask: u64) -> u64 {
        self.sliding_attack_table[self.bishop_attack_index(square, blocking_mask)]
    }

    pub fn lookup_rook(&self, square: usize, blocking_mask: u64) -> u64 {
        self.sliding_attack_table[self.rook_attack_index(square, blocking_mask)]
    }

    pub fn lookup_queen(&self, square: usize, blocking_mask: u64) -> u64 {
        self.lookup_bishop(square, blocking_mask) | self.lookup_rook(square, blocking_mask)
    }
}

// masks to prevent A-H file wrapping
pub const NOT_A_FILE: u64 = !0x0101010101010101;
pub const NOT_A_B_FILES: u64 = !0x0303030303030303;
pub const NOT_H_FILE: u64 = !0x8080808080808080;
pub const NOT_G_H_FILES: u64 = !0xC0C0C0C0C0C0C0C0;

// masks for ranks/files
pub const A_FILE: u64 = 0x0101010101010101;
pub const B_FILE: u64 = 0x0202020202020202;
pub const C_FILE: u64 = 0x0404040404040404;
pub const D_FILE: u64 = 0x0808080808080808;
pub const E_FILE: u64 = 0x1010101010101010;
pub const F_FILE: u64 = 0x2020202020202020;
pub const G_FILE: u64 = 0x4040404040404040;
pub const H_FILE: u64 = 0x8080808080808080;

pub const FIRST_RANK: u64 = 0x00000000000000FF;
pub const SECOND_RANK: u64 = 0x000000000000FF00;
pub const THIRD_RANK: u64 = 0x0000000000FF0000;
pub const FOURTH_RANK: u64 = 0x00000000FF000000;
pub const FIFTH_RANK: u64 = 0x000000FF00000000;
pub const SIXTH_RANK: u64 = 0x0000FF0000000000;
pub const SEVENTH_RANK: u64 = 0x00FF000000000000;
pub const EIGHTH_RANK: u64 = 0xFF00000000000000;

/// Generates a table mapping an input square to a mask of all squares a knight attacks from there
fn generate_knight_table() -> Vec<u64> {
    let mut table = Vec::with_capacity(64);

    for square in 0..64 {
        let knight = 1 << square;

        let moves = ((knight << 6) & NOT_G_H_FILES)
            | ((knight << 10) & NOT_A_B_FILES)
            | ((knight << 15) & NOT_H_FILE)
            | ((knight << 17) & NOT_A_FILE)
            | ((knight >> 6) & NOT_A_B_FILES)
            | ((knight >> 10) & NOT_G_H_FILES)
            | ((knight >> 15) & NOT_A_FILE)
            | ((knight >> 17) & NOT_H_FILE);

        table.push(moves);
    }
    table
}

/// Generates a table mapping an input square to a mask of all squares a king attacks from there
fn generate_king_table() -> Vec<u64> {
    let mut table = Vec::with_capacity(64);

    for square in 0..64 {
        let mut king = 1 << square;

        let mut moves = ((king << 1) & NOT_A_FILE) | ((king >> 1) & NOT_H_FILE);

        king |= moves;

        moves |= (king << 8) | (king >> 8);

        table.push(moves);
    }

    table
}

/// Generates a table mapping an input square to a mask of squares a pawn can push to from there
fn generate_pawn_push_tables() -> [Vec<u64>; 2] {
    let mut tables = [vec![0; 64], vec![0; 64]];

    for square in 8..56 {
        tables[White as usize][square as usize] = (1 << square) << 8;
        tables[Black as usize][square as usize] = (1 << square) >> 8;
    }

    tables
}

fn generate_pawn_attack_tables() -> [Vec<u64>; 2] {
    let mut tables = [vec![0; 64], vec![0; 64]];

    for square in 8..56 {
        tables[White as usize][square as usize] =
            ((1 << square << 7) & NOT_H_FILE) | ((1 << square << 9) & NOT_A_FILE);
        tables[Black as usize][square as usize] =
            ((1 << square >> 7) & NOT_A_FILE) | ((1 << square >> 9) & NOT_H_FILE);
    }

    tables
}

#[derive(Copy, Clone)]
pub struct MagicSquare {
    pub index: usize,
    pub mask: u64,
    pub magic: u64,
    pub shift: u8,
}

/// Generates magic numbers/shifts to look up rook attacks from each square
fn generate_rook_magics(attack_table: &mut Vec<u64>) -> Vec<MagicSquare> {
    let mut rook_magic = Vec::with_capacity(64);

    for square in 0..64 {
        rook_magic.push(find_magic(square, false, attack_table).unwrap());
    }
    rook_magic
}

/// Generates magic numbers/shifts to look up bishop attacks from each square
fn generate_bishop_magics(attack_table: &mut Vec<u64>) -> Vec<MagicSquare> {
    let mut bishop_magic = Vec::with_capacity(64);

    for square in 0..64 {
        bishop_magic.push(find_magic(square, true, attack_table).unwrap());
    }
    bishop_magic
}

fn find_magic(
    square: usize,
    bishop: bool,
    attack_table: &mut Vec<u64>,
) -> Result<MagicSquare, String> {
    let mask = if bishop {
        bishop_mask(square)
    } else {
        rook_mask(square)
    };

    let n = mask.count_ones() as u8;
    let mut blocking_masks = Vec::with_capacity(1 << n);
    let mut attack_masks = Vec::with_capacity(1 << n);

    // populate the arrays of attacking masks for this square
    for i in 0..(1 << n) {
        blocking_masks.push(index_to_blocking_mask(i, n, mask));
        attack_masks.push(if bishop {
            bishop_attacks(square, blocking_masks[i])
        } else {
            rook_attacks(square, blocking_masks[i])
        });
    }

    let index = attack_table.len();

    let mut used = vec![0; 1 << n];

    for _ in 0..100000000 {
        let magic = random_sparse_u64();

        // reset the vec for the next attempt
        for x in used.iter_mut() {
            *x = 0
        }
        let mut failed = false;
        for i in 0..(1 << n) {
            let index = magic_hash(blocking_masks[i], magic, n);
            if used[index] == 0 {
                used[index] = attack_masks[i];
            } else if used[index] != attack_masks[i] {
                failed = true;
                break;
            }
        }
        if !failed {
            let result = Ok(MagicSquare {
                index,
                mask,
                magic,
                shift: n,
            });

            // allocate more elements
            attack_table.extend(used);

            return result;
        }
    }

    Err(format!(
        "Failed to find magic number for square index {}",
        square
    ))
}

fn random_sparse_u64() -> u64 {
    let mut rng = thread_rng();
    rng.gen::<u64>() & rng.gen::<u64>() & rng.gen::<u64>()
}

fn magic_hash(blocking_mask: u64, magic: u64, shift: u8) -> usize {
    ((blocking_mask.wrapping_mul(magic)) >> (64 - shift)) as usize
}

fn index_to_blocking_mask(index: usize, num_blockers: u8, mut mask: u64) -> u64 {
    let mut result = 0;
    for i in 0..num_blockers {
        // find the bit-index of the first blocker and clear that bit in the mask
        let first_blocker = mask.trailing_zeros();
        mask ^= 1 << first_blocker;

        if index & (1 << i) != 0 {
            result |= 1 << first_blocker
        }
    }
    result
}

fn rook_mask(square: usize) -> u64 {
    let rank = (square / 8) as isize;
    let file = (square % 8) as isize;

    let mut result = 0;

    for y in (rank + 1)..7 {
        result |= 1 << (file + y * 8);
    }
    for y in 1..rank {
        result |= 1 << (file + y * 8);
    }

    for x in (file + 1)..7 {
        result |= 1 << (x + rank * 8)
    }
    for x in 1..file {
        result |= 1 << (x + rank * 8)
    }

    result
}
fn bishop_mask(square: usize) -> u64 {
    let rank = (square / 8) as isize;
    let file = (square % 8) as isize;

    let mut result = 0;

    let mut x = file + 1;
    let mut y = rank + 1;
    while x < 7 && y < 7 {
        result |= 1 << (x + y * 8);
        x += 1;
        y += 1;
    }

    x = file - 1;
    y = rank + 1;
    while x > 0 && y < 7 {
        result |= 1 << (x + y * 8);
        x -= 1;
        y += 1;
    }

    x = file - 1;
    y = rank - 1;
    while x > 0 && y > 0 {
        result |= 1 << (x + y * 8);
        x -= 1;
        y -= 1;
    }

    x = file + 1;
    y = rank - 1;
    while x < 7 && y > 0 {
        result |= 1 << (x + y * 8);
        x += 1;
        y -= 1;
    }
    result
}

fn rook_attacks(square: usize, blocking_mask: u64) -> u64 {
    let rank = (square / 8) as isize;
    let file = (square % 8) as isize;

    let mut result = 0;

    for y in (rank + 1)..8 {
        result |= 1 << (file + y * 8);
        if blocking_mask & (1 << (file + y * 8)) != 0 {
            break;
        }
    }
    for y in (0..rank).rev() {
        result |= 1 << (file + y * 8);
        if blocking_mask & (1 << (file + y * 8)) != 0 {
            break;
        }
    }
    for x in (file + 1)..8 {
        result |= 1 << (x + rank * 8);
        if blocking_mask & (1 << (x + rank * 8)) != 0 {
            break;
        }
    }
    for x in (0..file).rev() {
        result |= 1 << (x + rank * 8);
        if blocking_mask & (1 << (x + rank * 8)) != 0 {
            break;
        }
    }

    result
}

fn bishop_attacks(square: usize, blocking_mask: u64) -> u64 {
    let rank = (square / 8) as isize;
    let file = (square % 8) as isize;

    let mut result = 0;

    let mut x = file + 1;
    let mut y = rank + 1;
    while x <= 7 && y <= 7 {
        result |= 1 << (x + y * 8);
        if blocking_mask & (1 << (x + y * 8)) != 0 {
            break;
        }
        x += 1;
        y += 1;
    }

    x = file - 1;
    y = rank + 1;
    while x >= 0 && y <= 7 {
        result |= 1 << (x + y * 8);
        if blocking_mask & (1 << (x + y * 8)) != 0 {
            break;
        }
        x -= 1;
        y += 1;
    }

    x = file - 1;
    y = rank - 1;
    while x >= 0 && y >= 0 {
        result |= 1 << (x + y * 8);
        if blocking_mask & (1 << (x + y * 8)) != 0 {
            break;
        }
        x -= 1;
        y -= 1;
    }

    x = file + 1;
    y = rank - 1;
    while x <= 7 && y >= 0 {
        result |= 1 << (x + y * 8);
        if blocking_mask & (1 << (x + y * 8)) != 0 {
            break;
        }
        x += 1;
        y -= 1;
    }

    result
}
