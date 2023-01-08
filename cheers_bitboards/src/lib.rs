use std::{
    fmt::{Debug, Display},
    ops::{Deref, Index, IndexMut},
};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Ord, PartialOrd)]
pub struct BitBoard(pub u64);

impl BitBoard {
    #[inline(always)]
    pub fn empty() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
    #[inline(always)]
    pub fn is_not_empty(&self) -> bool {
        self.0 != 0
    }
    #[inline(always)]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
    #[inline(always)]
    pub fn inverse(&self) -> Self {
        Self(!self.0)
    }
    #[inline(always)]
    pub fn first_square(&self) -> Square {
        Square(self.0.trailing_zeros() as u8)
    }
    #[inline(always)]
    pub fn clear_first_square(&mut self) {
        self.0 &= self.0 - 1;
    }
    #[inline(always)]
    pub fn count_ones(&self) -> u32 {
        self.0.count_ones()
    }
    #[inline(always)]
    pub fn ishift(&self, n: i32) -> Self {
        if n > 0 {
            Self(self.0 << n)
        } else {
            Self(self.0 >> (-n))
        }
    }
}

impl Iterator for BitBoard {
    type Item = Square;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            None
        } else {
            let sq = Some(self.first_square());
            self.clear_first_square();
            sq
        }
    }
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count_ones() as usize, Some(self.count_ones() as usize))
    }

    #[inline(always)]
    fn count(self) -> usize {
        self.count_ones() as usize
    }
}
impl ExactSizeIterator for BitBoard {}

impl Display for BitBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let board_string = format!("{:064b}", self.0);
        let ranks = board_string
            .chars()
            .rev()
            .map(|c| if c == '0' { "." } else { "1" })
            .collect::<Vec<_>>()
            .chunks(8)
            .map(|c| c.join(" "))
            .rev()
            .collect::<Vec<String>>();

        writeln!(f)?;
        writeln!(f, "8  {}", ranks[0])?;
        writeln!(f, "7  {}", ranks[1])?;
        writeln!(f, "6  {}", ranks[2])?;
        writeln!(f, "5  {}", ranks[3])?;
        writeln!(f, "4  {}", ranks[4])?;
        writeln!(f, "3  {}", ranks[5])?;
        writeln!(f, "2  {}", ranks[6])?;
        writeln!(f, "1  {}", ranks[7])?;
        writeln!(f, "\n   a b c d e f g h")
    }
}

macro_rules! bb_impl_op {
    ($op: ident, $fn: ident) => {
        impl std::ops::$op for BitBoard {
            type Output = BitBoard;
            #[inline(always)]
            fn $fn(self, rhs: BitBoard) -> Self::Output {
                BitBoard(self.0.$fn(rhs.0))
            }
        }
    };
}
bb_impl_op!(BitAnd, bitand);
bb_impl_op!(BitOr, bitor);
bb_impl_op!(BitXor, bitxor);

macro_rules! bb_impl_op_assign {
    ($op: ident, $fn: ident) => {
        impl std::ops::$op for BitBoard {
            #[inline(always)]
            fn $fn(&mut self, rhs: Self) {
                self.0.$fn(rhs.0)
            }
        }
    };
}
bb_impl_op_assign!(BitAndAssign, bitand_assign);
bb_impl_op_assign!(BitOrAssign, bitor_assign);
bb_impl_op_assign!(BitXorAssign, bitxor_assign);

macro_rules! bb_impl_shift {
    ($sh: ident, $fn: ident, $n: ty) => {
        impl std::ops::$sh<$n> for BitBoard {
            type Output = Self;
            #[inline(always)]
            fn $fn(self, rhs: $n) -> Self::Output {
                Self(self.0.$fn(rhs))
            }
        }
    };
}

bb_impl_shift!(Shl, shl, u8);
bb_impl_shift!(Shr, shr, u8);

mod consts {
    use super::BitBoard;

    pub const NOT_A_FILE: BitBoard = BitBoard(!0x0101010101010101);
    pub const NOT_A_B_FILES: BitBoard = BitBoard(!0x0303030303030303);
    pub const NOT_H_FILE: BitBoard = BitBoard(!0x8080808080808080);
    pub const NOT_G_H_FILES: BitBoard = BitBoard(!0xC0C0C0C0C0C0C0C0);

    // masks for ranks/files
    pub const A_FILE: BitBoard = BitBoard(0x0101010101010101);
    pub const B_FILE: BitBoard = BitBoard(0x0202020202020202);
    pub const C_FILE: BitBoard = BitBoard(0x0404040404040404);
    pub const D_FILE: BitBoard = BitBoard(0x0808080808080808);
    pub const E_FILE: BitBoard = BitBoard(0x1010101010101010);
    pub const F_FILE: BitBoard = BitBoard(0x2020202020202020);
    pub const G_FILE: BitBoard = BitBoard(0x4040404040404040);
    pub const H_FILE: BitBoard = BitBoard(0x8080808080808080);

    pub const FILES: [BitBoard; 8] = [
        A_FILE, B_FILE, C_FILE, D_FILE, E_FILE, F_FILE, G_FILE, H_FILE,
    ];

    pub const FIRST_RANK: BitBoard = BitBoard(0x00000000000000FF);
    pub const SECOND_RANK: BitBoard = BitBoard(0x000000000000FF00);
    pub const THIRD_RANK: BitBoard = BitBoard(0x0000000000FF0000);
    pub const FOURTH_RANK: BitBoard = BitBoard(0x00000000FF000000);
    pub const FIFTH_RANK: BitBoard = BitBoard(0x000000FF00000000);
    pub const SIXTH_RANK: BitBoard = BitBoard(0x0000FF0000000000);
    pub const SEVENTH_RANK: BitBoard = BitBoard(0x00FF000000000000);
    pub const EIGHTH_RANK: BitBoard = BitBoard(0xFF00000000000000);

    pub const LIGHT_SQUARES: BitBoard = BitBoard(0x5555555555555555);
    pub const DARK_SQUARES: BitBoard = BitBoard(0xAAAAAAAAAAAAAAAA);

    pub const FULL_BOARD: BitBoard = BitBoard(0xFFFFFFFFFFFFFFFF);

    pub const LONG_DIAGONALS: BitBoard = BitBoard(0x8142241818244281);
}
pub use self::consts::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Square(u8);

impl Square {
    pub const A1: Self = Self(0);
    pub const B1: Self = Self(1);
    pub const C1: Self = Self(2);
    pub const D1: Self = Self(3);
    pub const E1: Self = Self(4);
    pub const F1: Self = Self(5);
    pub const G1: Self = Self(6);
    pub const H1: Self = Self(7);
    pub const A2: Self = Self(8);
    pub const B2: Self = Self(9);
    pub const C2: Self = Self(10);
    pub const D2: Self = Self(11);
    pub const E2: Self = Self(12);
    pub const F2: Self = Self(13);
    pub const G2: Self = Self(14);
    pub const H2: Self = Self(15);
    pub const A3: Self = Self(16);
    pub const B3: Self = Self(17);
    pub const C3: Self = Self(18);
    pub const D3: Self = Self(19);
    pub const E3: Self = Self(20);
    pub const F3: Self = Self(21);
    pub const G3: Self = Self(22);
    pub const H3: Self = Self(23);
    pub const A4: Self = Self(24);
    pub const B4: Self = Self(25);
    pub const C4: Self = Self(26);
    pub const D4: Self = Self(27);
    pub const E4: Self = Self(28);
    pub const F4: Self = Self(29);
    pub const G4: Self = Self(30);
    pub const H4: Self = Self(31);
    pub const A5: Self = Self(32);
    pub const B5: Self = Self(33);
    pub const C5: Self = Self(34);
    pub const D5: Self = Self(35);
    pub const E5: Self = Self(36);
    pub const F5: Self = Self(37);
    pub const G5: Self = Self(38);
    pub const H5: Self = Self(39);
    pub const A6: Self = Self(40);
    pub const B6: Self = Self(41);
    pub const C6: Self = Self(42);
    pub const D6: Self = Self(43);
    pub const E6: Self = Self(44);
    pub const F6: Self = Self(45);
    pub const G6: Self = Self(46);
    pub const H6: Self = Self(47);
    pub const A7: Self = Self(48);
    pub const B7: Self = Self(49);
    pub const C7: Self = Self(50);
    pub const D7: Self = Self(51);
    pub const E7: Self = Self(52);
    pub const F7: Self = Self(53);
    pub const G7: Self = Self(54);
    pub const H7: Self = Self(55);
    pub const A8: Self = Self(56);
    pub const B8: Self = Self(57);
    pub const C8: Self = Self(58);
    pub const D8: Self = Self(59);
    pub const E8: Self = Self(60);
    pub const F8: Self = Self(61);
    pub const G8: Self = Self(62);
    pub const H8: Self = Self(63);
    pub const NULL: Self = Self(64);

    #[inline(always)]
    pub fn bitboard(&self) -> BitBoard {
        BitBoard(1u64.wrapping_shl(self.0 as u32))
    }

    #[inline(always)]
    pub fn rank(&self) -> usize {
        (self.0 / 8) as usize
    }

    #[inline(always)]
    pub fn file(&self) -> usize {
        (self.0 % 8) as usize
    }

    #[inline(always)]
    pub fn offset(&self, file: i8, rank: i8) -> Self {
        Self((self.0 as i8 + rank * 8 + file) as u8)
    }

    pub fn coord(&self) -> String {
        let mut res = String::new();
        let file = match self.file() {
            0 => "a",
            1 => "b",
            2 => "c",
            3 => "d",
            4 => "e",
            5 => "f",
            6 => "g",
            7 => "h",
            _ => unreachable!(),
        };
        res += file;
        res.push_str(&(self.rank() + 1).to_string());
        res
    }

    pub fn from_coord<T: AsRef<str>>(coord: T) -> Self {
        let coord = coord.as_ref();
        let file = match coord.chars().nth(0) {
            Some('a') => 0,
            Some('b') => 1,
            Some('c') => 2,
            Some('d') => 3,
            Some('e') => 4,
            Some('f') => 5,
            Some('g') => 6,
            Some('h') => 7,
            _ => unreachable!(),
        };
        let rank = coord.chars().nth(1).unwrap().to_digit(10).unwrap() as u8 - 1;
        Self(rank * 8 + file)
    }
}

impl Deref for Square {
    type Target = u8;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T, const N: usize> Index<Square> for [T; N] {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: Square) -> &Self::Output {
        &self[index.0 as usize]
    }
}

impl<T, const N: usize> IndexMut<Square> for [T; N] {
    #[inline(always)]
    fn index_mut(&mut self, index: Square) -> &mut Self::Output {
        &mut self[index.0 as usize]
    }
}

macro_rules! square_from_impl {
    ($ty: ty) => {
        impl From<$ty> for Square {
            #[inline(always)]
            fn from(n: $ty) -> Self {
                Self(n as u8)
            }
        }
    };
}

square_from_impl!(u8);
square_from_impl!(u16);
square_from_impl!(u32);
square_from_impl!(u64);
square_from_impl!(usize);
square_from_impl!(i8);
square_from_impl!(i16);
square_from_impl!(i32);
square_from_impl!(i64);
square_from_impl!(isize);
