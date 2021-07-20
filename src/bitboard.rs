use itertools::repeat_n;

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

pub fn square_to_coord(square: u8) -> String {
    let mut result = String::new();
    result.push(match square % 8 {
        0 => 'a',
        1 => 'b',
        2 => 'c',
        3 => 'd',
        4 => 'e',
        5 => 'f',
        6 => 'g',
        7 => 'h',
        _ => unreachable!(),
    });
    result.push(match square / 8 {
        0 => '1',
        1 => '2',
        2 => '3',
        3 => '4',
        4 => '5',
        5 => '6',
        6 => '7',
        7 => '8',
        _ => unreachable!(),
    });

    result
}

pub fn coord_to_square(coord: &str) -> u8 {
    let mut result = match coord.chars().nth(0).unwrap() {
        'a' => 0,
        'b' => 1,
        'c' => 2,
        'd' => 3,
        'e' => 4,
        'f' => 5,
        'g' => 6,
        'h' => 7,
        _ => unreachable!(),
    };
    result += match coord.chars().nth(1).unwrap() {
        '1' => 0 * 8,
        '2' => 1 * 8,
        '3' => 2 * 8,
        '4' => 3 * 8,
        '5' => 4 * 8,
        '6' => 5 * 8,
        '7' => 6 * 8,
        '8' => 7 * 8,
        _ => unreachable!(),
    };
    result
}

#[derive(Copy, Clone, Debug)]
pub enum PieceIndex {
    Pawn = 0,
    Bishop = 1,
    Knight = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

#[derive(Copy, Clone, Debug)]
#[repr(usize)]
pub enum ColorIndex {
    White = 0,
    Black = 1,
}

impl std::ops::Not for ColorIndex {
    type Output = ColorIndex;
    fn not(self) -> Self::Output {
        match self {
            White => Black,
            Black => White,
        }
    }
}

pub use ColorIndex::*;
pub use PieceIndex::*;

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
    piece_list: Vec<Option<(PieceIndex, ColorIndex)>>,
}

impl BitBoards {
    /// Creates a new set of bitboards in the starting position
    pub fn new(lookup_tables: LookupTables) -> Self {
        let mut piece_list = vec![None; 64];

        piece_list.splice(
            0..8,
            [
                Some((Rook, White)),
                Some((Knight, White)),
                Some((Bishop, White)),
                Some((Queen, White)),
                Some((King, White)),
                Some((Bishop, White)),
                Some((Knight, White)),
                Some((Rook, White)),
            ],
        );

        piece_list.splice(8..16, repeat_n(Some((Pawn, White)), 8));
        piece_list.splice(48..56, repeat_n(Some((Pawn, Black)), 8));

        piece_list.splice(
            56..64,
            [
                Some((Rook, Black)),
                Some((Knight, Black)),
                Some((Bishop, Black)),
                Some((Queen, Black)),
                Some((King, Black)),
                Some((Bishop, Black)),
                Some((Knight, Black)),
                Some((Rook, Black)),
            ],
        );

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
            piece_list,
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

    pub fn knight_moves(&self, color: ColorIndex) -> Vec<(u8, u8)> {
        let mut moves = Vec::new();

        let mut knights = self.piece_masks[Knight] & self.color_masks[color];
        while knights != 0 {
            let i = knights.trailing_zeros() as usize;
            let mut result = self.lookup_tables.lookup_knight(i) & !self.color_masks[color];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                moves.push((i as u8, target));

                result ^= 1 << target;
            }
            knights ^= 1 << i;
        }
        moves
    }

    pub fn bishop_attacks(&self, color: ColorIndex) -> u64 {
        let mut bishops = self.piece_masks[Bishop] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        let mut result = 0;
        while bishops != 0 {
            let i = bishops.trailing_zeros() as usize;
            result |= self.lookup_tables.lookup_bishop(i, blocking_mask);
            bishops ^= 1 << i;
        }
        result
    }

    pub fn bishop_moves(&self, color: ColorIndex) -> Vec<(u8, u8)> {
        let mut moves = Vec::new();

        let mut bishops = self.piece_masks[Bishop] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        while bishops != 0 {
            let i = bishops.trailing_zeros() as usize;
            let mut result =
                self.lookup_tables.lookup_bishop(i, blocking_mask) & !self.color_masks[color];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                moves.push((i as u8, target));

                result ^= 1 << target;
            }
            bishops ^= 1 << i;
        }
        moves
    }

    pub fn rook_attacks(&self, color: ColorIndex) -> u64 {
        let mut rooks = self.piece_masks[Rook] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        let mut result = 0;
        while rooks != 0 {
            let i = rooks.trailing_zeros() as usize;
            result |= self.lookup_tables.lookup_rook(i, blocking_mask);
            rooks ^= 1 << i;
        }
        result
    }

    pub fn rook_moves(&self, color: ColorIndex) -> Vec<(u8, u8)> {
        let mut moves = Vec::new();

        let mut rooks = self.piece_masks[Rook] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        while rooks != 0 {
            let i = rooks.trailing_zeros() as usize;
            let mut result =
                self.lookup_tables.lookup_rook(i, blocking_mask) & !self.color_masks[color];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                moves.push((i as u8, target));

                result ^= 1 << target;
            }
            rooks ^= 1 << i;
        }
        moves
    }

    pub fn queen_attacks(&self, color: ColorIndex) -> u64 {
        let mut queens = self.piece_masks[Queen] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        let mut result = 0;
        while queens != 0 {
            let i = queens.trailing_zeros() as usize;
            result |= self.lookup_tables.lookup_bishop(i, blocking_mask)
                | self.lookup_tables.lookup_rook(i, blocking_mask);
            queens ^= 1 << i;
        }
        result
    }

    pub fn queen_moves(&self, color: ColorIndex) -> Vec<(u8, u8)> {
        let mut moves = Vec::new();

        let mut queens = self.piece_masks[Queen] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        while queens != 0 {
            let i = queens.trailing_zeros() as usize;
            let mut result =
                self.lookup_tables.lookup_queen(i, blocking_mask) & !self.color_masks[color];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                moves.push((i as u8, target));

                result ^= 1 << target;
            }
            queens ^= 1 << i;
        }
        moves
    }

    pub fn king_attacks(&self, color: ColorIndex) -> u64 {
        let king = self.piece_masks[King] & self.color_masks[color];

        self.lookup_tables
            .lookup_king(king.trailing_zeros() as usize)
    }

    pub fn king_moves(&self, color: ColorIndex) -> Vec<(u8, u8)> {
        let mut moves = Vec::new();

        let king = self.piece_masks[King] & self.color_masks[color];
        let square = king.trailing_zeros() as usize;

        let mut result = self.lookup_tables.lookup_king(square) & !self.color_masks[color];

        while result != 0 {
            let target = result.trailing_zeros() as u8;
            moves.push((square as u8, target));

            result ^= 1 << target;
        }
        moves
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

    pub fn white_pawn_moves(&self) -> Vec<(u8, u8)> {
        let mut moves = Vec::new();
        let mut pawns = self.piece_masks[Pawn] & self.color_masks[White];

        while pawns != 0 {
            let i = pawns.trailing_zeros() as usize;
            let mut result = self.lookup_tables.lookup_pawn_push(i, White);

            let empty = !(self.color_masks[White] | self.color_masks[Black]);

            // add double pushes to relevant unblocked single pushes
            result |= (result & THIRD_RANK & empty) << 8;

            // remove blocked double pushes
            result &= empty;

            print_bitboard(result);

            result |= self.lookup_tables.lookup_pawn_attack(i, White) & self.color_masks[Black];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                moves.push((i as u8, target));

                result ^= 1 << target;
            }
            pawns ^= 1 << i;
        }
        moves
    }

    pub fn black_pawn_moves(&self) -> Vec<(u8, u8)> {
        let mut moves = Vec::new();
        let mut pawns = self.piece_masks[Pawn] & self.color_masks[Black];

        while pawns != 0 {
            let i = pawns.trailing_zeros() as usize;
            let mut result = self.lookup_tables.lookup_pawn_push(i, Black);

            let empty = !(self.color_masks[White] | self.color_masks[Black]);

            // add double pushes to relevant unblocked single pushes
            result |= (result & SIXTH_RANK & empty) >> 8;

            // remove blocked double pushes
            result &= empty;

            result |= self.lookup_tables.lookup_pawn_attack(i, Black) & self.color_masks[White];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                moves.push((i as u8, target));

                result ^= 1 << target;
            }
            pawns ^= 1 << i;
        }
        moves
    }

    pub fn generate_pseudolegal_moves(&self, color: ColorIndex) -> Vec<(u8, u8)> {
        let mut moves = Vec::new();
        moves.extend(self.knight_moves(color));
        moves.extend(self.bishop_moves(color));
        moves.extend(self.rook_moves(color));
        moves.extend(self.queen_moves(color));
        moves.extend(self.king_moves(color));
        match color {
            White => moves.extend(self.white_pawn_moves()),
            Black => moves.extend(self.black_pawn_moves()),
        }

        moves
    }

    pub fn make_move(&mut self, start: usize, target: usize) {
        // TODO: special case castling and en passent

        // assume we only get legal moves from the UI
        let (piece, color) = self.piece_list[start].unwrap();

        // move the piece to the target square
        self.piece_masks[piece] |= 1 << target;
        self.piece_masks[piece] ^= 1 << start;

        // update the masks for both colors
        self.color_masks[color] |= 1 << target;
        self.color_masks[color] ^= 1 << start;

        // taking a piece
        self.color_masks[!color] &= !(1 << target);

        print_bitboard(self.color_masks[White] | self.color_masks[Black]);
    }
}

#[inline(always)]
/// Returns an integer with only the lowest '1' bit of the input set
fn lowest_set_bit(n: u64) -> u64 {
    n & (1 << n.trailing_zeros())
}
