use crate::lookup_tables::*;

pub fn print_bitboard(board: u64) {
    let bits = format!("{:064b}", board);
    for row in 0..8 {
        let line = &bits[8 * row..(8 * row + 8)];
        for square in line.chars().rev() {
            match square {
                '0' => print!(". "),
                '1' => print!("1 "),
                _ => unreachable!(),
            }
        }
        print!("\n");
    }
    print!("\n");
}

#[derive(Copy, Clone, Debug)]
#[repr(usize)]
enum PieceIndex {
    Pawn = 0,
    Bishop = 1,
    Knight = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

#[derive(Copy, Clone, Debug)]
#[repr(usize)]
enum ColorIndex {
    White = 0,
    Black = 1,
}

use ColorIndex::*;
use PieceIndex::*;

struct ColorMasks([u64; 2]);

impl std::ops::Index<ColorIndex> for ColorMasks {
    type Output = u64;

    fn index(&self, index: ColorIndex) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl std::ops::IndexMut<ColorIndex> for ColorMasks {
    fn index_mut(&mut self, index: ColorIndex) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}
struct PieceMasks([u64; 6]);
impl std::ops::Index<PieceIndex> for PieceMasks {
    type Output = u64;

    fn index(&self, index: PieceIndex) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl std::ops::IndexMut<PieceIndex> for PieceMasks {
    fn index_mut(&mut self, index: PieceIndex) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

pub struct BitBoards {
    color_masks: ColorMasks,
    piece_masks: PieceMasks,
    lookup_tables: LookupTables,
}

impl BitBoards {
    /// Creates a new set of bitboards in the starting position
    pub fn new(lookup_tables: LookupTables) -> Self {
        let black_mask = 0xFFFF000000000000;
        let white_mask = 0x000000000000FFFF;

        let pawn_mask = 0x00FF00000000FF00;
        let bishop_mask = 0x2400000000000024;
        let knight_mask = 0x4200000000000042;
        let rook_mask = 0x8100000000000081;

        let queen_mask = 0x0800000000000008;
        let king_mask = 0x100000000000010;

        Self {
            color_masks: ColorMasks([white_mask, black_mask]),
            piece_masks: PieceMasks([
                pawn_mask,
                bishop_mask,
                knight_mask,
                rook_mask,
                queen_mask,
                king_mask,
            ]),
            lookup_tables,
        }
    }

    pub fn knight_attacks(&self, color: ColorIndex) -> u64 {
        let mut knights = self.piece_masks[Knight] & self.color_masks[color];

        let mut result = 0;
        while knights != 0 {
            let i = knights.trailing_zeros() as usize;
            result |= self.lookup_tables.lookup_knight(i);
            knights ^= 1 << i;
        }
        result
    }

    pub fn bishop_attacks(&self, color: ColorIndex) -> u64 {
        let mut bishops = self.piece_masks[Bishop] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] & self.color_masks[Black];

        let mut result = 0;
        while bishops != 0 {
            let i = bishops.trailing_zeros() as usize;
            result |= self.lookup_tables.lookup_bishop(i, blocking_mask);
            bishops ^= 1 << i;
        }
        result
    }

    pub fn rook_attacks(&self, color: ColorIndex) -> u64 {
        let mut rooks = self.piece_masks[Rook] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] & self.color_masks[Black];

        let mut result = 0;
        while rooks != 0 {
            let i = rooks.trailing_zeros() as usize;
            result |= self.lookup_tables.lookup_rook(i, blocking_mask);
            rooks ^= 1 << i;
        }
        result
    }

    pub fn queen_attacks(&self, color: ColorIndex) -> u64 {
        let mut queens = self.piece_masks[Queen] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] & self.color_masks[Black];

        let mut result = 0;
        while queens != 0 {
            let i = queens.trailing_zeros() as usize;
            result |= self.lookup_tables.lookup_bishop(i, blocking_mask)
                | self.lookup_tables.lookup_rook(i, blocking_mask);
            queens ^= 1 << i;
        }
        result
    }

    pub fn king_attacks(&self, color: ColorIndex) -> u64 {
        let king = self.piece_masks[King] & self.color_masks[color];

        self.lookup_tables
            .lookup_king(king.trailing_zeros() as usize)
    }

    pub fn pawn_attacks(&self, color: ColorIndex) -> u64 {
        match color {
            White => {
                let pawns = self.piece_masks[Pawn] & self.color_masks[White];
                let west_attacks = (pawns << 7) & NOT_H_FILE;
                let east_attacks = (pawns << 9) & NOT_A_FILE;

                west_attacks | east_attacks
            }
            Black => {
                let pawns = self.piece_masks[Pawn] & self.color_masks[Black];
                let west_attacks = (pawns >> 9) & NOT_H_FILE;
                let east_attacks = (pawns >> 7) & NOT_A_FILE;

                west_attacks | east_attacks
            }
        }
    }
}

#[inline(always)]
/// Returns an integer with only the lowest '1' bit of the input set
fn lowest_set_bit(n: u64) -> u64 {
    n & (1 << n.trailing_zeros())
}
