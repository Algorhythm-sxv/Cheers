use crate::types::{ColorIndex, ColorIndex::*};
use once_cell::sync::OnceCell;
use rand::prelude::*;

pub static LOOKUP_TABLES: OnceCell<LookupTables> = OnceCell::new();

pub fn lookup_tables() -> &'static LookupTables {
    LOOKUP_TABLES.get_or_init(|| {
        let mut sliding_attack_table = Vec::with_capacity(10000);

        let rook_magics = generate_rook_magics(&mut sliding_attack_table, true);
        let bishop_magics = generate_bishop_magics(&mut sliding_attack_table, true);

        LookupTables {
            knight_table: generate_knight_table(),
            king_table: generate_king_table(),
            pawn_push_one_tables: generate_pawn_push_tables(),
            pawn_attack_tables: generate_pawn_attack_tables(),
            sliding_attack_table,
            rook_magics,
            bishop_magics,
        }
    })
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
        LOOKUP_TABLES.get_or_init(|| {
            let mut sliding_attack_table = Vec::with_capacity(10000);

            let rook_magics = generate_rook_magics(&mut sliding_attack_table, false);
            let bishop_magics = generate_bishop_magics(&mut sliding_attack_table, false);

            LookupTables {
                knight_table: generate_knight_table(),
                king_table: generate_king_table(),
                pawn_push_one_tables: generate_pawn_push_tables(),
                pawn_attack_tables: generate_pawn_attack_tables(),
                sliding_attack_table,
                rook_magics,
                bishop_magics,
            }
        })
    }

    fn bishop_attack_index(&self, square: usize, blocking_mask: u64) -> usize {
        let magic_square = self.bishop_magics[square];
        magic_square.index
            + magic_hash(
                magic_square.mask & blocking_mask,
                magic_square.magic,
                magic_square.shift,
            )
    }

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

    pub fn print_magics(&self) {
        println!("Rook magics:");
        for square in self.rook_magics.iter() {
            println!("{:#018X},", square.magic);
        }
        println!("Bishop magics:");
        for square in self.bishop_magics.iter() {
            println!("{:#018X},", square.magic);
        }
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
fn generate_rook_magics(attack_table: &mut Vec<u64>, use_pregen: bool) -> Vec<MagicSquare> {
    let mut rook_magic = Vec::with_capacity(64);

    for square in 0..64 {
        rook_magic.push(find_magic(square, false, attack_table, use_pregen).unwrap());
    }
    rook_magic
}

/// Generates magic numbers/shifts to look up bishop attacks from each square
fn generate_bishop_magics(attack_table: &mut Vec<u64>, use_pregen: bool) -> Vec<MagicSquare> {
    let mut bishop_magic = Vec::with_capacity(64);

    for square in 0..64 {
        bishop_magic.push(find_magic(square, true, attack_table, use_pregen).unwrap());
    }
    bishop_magic
}

fn find_magic(
    square: usize,
    bishop: bool,
    attack_table: &mut Vec<u64>,
    use_pregen: bool,
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
        let magic = if use_pregen {
            if bishop {
                BISHOP_MAGICS[square]
            } else {
                ROOK_MAGICS[square]
            }
        } else {
            random_sparse_u64()
        };

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

const ROOK_MAGICS: [u64; 64] = [
    0xB480081120804000,
    0x21001020C0010080,
    0x8480100020000880,
    0x8100041000890060,
    0x048004008008004A,
    0x1500022100940008,
    0x0900242082003100,
    0x0080018000442B00,
    0x4048800040006080,
    0x0001004005008021,
    0x8580802000100480,
    0x0010803000800800,
    0x4120808004008800,
    0x4100800400020080,
    0x0019000700048200,
    0x0009000084620100,
    0x2800208000C00088,
    0x5040008041600080,
    0x4001010010200040,
    0x010452002200C008,
    0x8000808008020400,
    0x0044004002010040,
    0x3010540010260108,
    0x08010A000100906C,
    0x21C2400080002095,
    0x2100810200412201,
    0x0460450100200032,
    0x8000180080500080,
    0x2480080080140080,
    0x0088040080800200,
    0x0001081400019002,
    0x0021009200094104,
    0x3080C00188800122,
    0x404100C001002080,
    0x0000200080801004,
    0x01020230420008A0,
    0x0800140080800800,
    0x0004000200800480,
    0x0040880B84000210,
    0x2024054082001401,
    0x2014A84000808000,
    0x0041006200820044,
    0x0001004020090014,
    0x100008D001030020,
    0x0413080111010004,
    0x068200900C320048,
    0x9043008200430004,
    0x000020A104420004,
    0x4100800040002080,
    0x80070128C0008100,
    0x5044802000100480,
    0x0000900080080080,
    0x0000800400080080,
    0x0002008026040080,
    0x1100420148102400,
    0x00100C0091004600,
    0x0002204980010015,
    0x02A1004000802019,
    0x0006201008408202,
    0x4460201200964046,
    0x21020008A004B0A2,
    0x0002005038150402,
    0x0880221021408804,
    0x0000004411208502,
];

const BISHOP_MAGICS: [u64; 64] = [
    0x2008060828070110,
    0x0008500082004201,
    0x00040122020000C0,
    0x2004410220001008,
    0x0824042000101060,
    0x001A0124A0000000,
    0x2321042202402008,
    0x1400A3080804022A,
    0x0804050802180200,
    0x000A111012890840,
    0x1000460806088008,
    0x0002020A02021400,
    0x0001420210081000,
    0x00000208120A1300,
    0x0008141A88080801,
    0x2008851188040285,
    0x4C10002002220860,
    0x082002908D021098,
    0x0012000408020488,
    0x0000902802004100,
    0x0444000080A00040,
    0x4000802410148800,
    0xA012100D48242418,
    0x3180221184042600,
    0x0008040008105041,
    0x00108A20300A0609,
    0xC004100281010022,
    0xA062006182008200,
    0x1A05010108104000,
    0x201A008002482000,
    0x0882440422440200,
    0x0085002001008810,
    0x0901105200400402,
    0x0500880400081000,
    0x2214042810140040,
    0x1280208120080200,
    0x0040C0C0400C0100,
    0x000500D602050100,
    0x4104810040020821,
    0x02220A044009A401,
    0xA014020210C04000,
    0x0014110808004220,
    0x20A20100A8000480,
    0x0000004204805808,
    0x0430880905001010,
    0x0006009001040A80,
    0x08A0040100488A00,
    0x50100C0490842462,
    0x0001880508200080,
    0x21220A1301180040,
    0x4210060044120004,
    0x0210000022880440,
    0x0004440810340041,
    0x0002041004084006,
    0x0040900942008000,
    0x802002261A08A028,
    0x1000402208200412,
    0x0003020644044404,
    0x0420C00201039803,
    0x0E10000020C20205,
    0x0418400040086600,
    0x030001400C888082,
    0x3C0040480802808A,
    0x0004040810510208,
];
