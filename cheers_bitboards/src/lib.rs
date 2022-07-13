use std::{
    fmt::{Debug, Display},
    ops::{Index, IndexMut},
};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
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
        Square::try_from(self.0.trailing_zeros() as u8).unwrap()
    }
    #[inline(always)]
    pub fn try_first_square(&self) -> Option<Square> {
        Square::try_from(self.0.trailing_zeros() as u8)
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
        self.try_first_square().map(|s| {
            self.clear_first_square();
            s
        })
        // if self.0 == 0 {
        //     None
        // } else {
        //     let i = self.first_square();
        //     self.clear_first_square();
        //     Some(i)
        // }
    }
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count_ones() as usize, Some(self.count_ones() as usize))
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
            fn $fn(self, rhs: $n) -> Self::Output {
                Self(self.0.$fn(rhs))
            }
        }
    };
}

bb_impl_shift!(Shl, shl, u8);
bb_impl_shift!(Shr, shr, u8);

#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Square {
    A1 = 0,
    B1 = 1,
    C1 = 2,
    D1 = 3,
    E1 = 4,
    F1 = 5,
    G1 = 6,
    H1 = 7,
    A2 = 8,
    B2 = 9,
    C2 = 10,
    D2 = 11,
    E2 = 12,
    F2 = 13,
    G2 = 14,
    H2 = 15,
    A3 = 16,
    B3 = 17,
    C3 = 18,
    D3 = 19,
    E3 = 20,
    F3 = 21,
    G3 = 22,
    H3 = 23,
    A4 = 24,
    B4 = 25,
    C4 = 26,
    D4 = 27,
    E4 = 28,
    F4 = 29,
    G4 = 30,
    H4 = 31,
    A5 = 32,
    B5 = 33,
    C5 = 34,
    D5 = 35,
    E5 = 36,
    F5 = 37,
    G5 = 38,
    H5 = 39,
    A6 = 40,
    B6 = 41,
    C6 = 42,
    D6 = 43,
    E6 = 44,
    F6 = 45,
    G6 = 46,
    H6 = 47,
    A7 = 48,
    B7 = 49,
    C7 = 50,
    D7 = 51,
    E7 = 52,
    F7 = 53,
    G7 = 54,
    H7 = 55,
    A8 = 56,
    B8 = 57,
    C8 = 58,
    D8 = 59,
    E8 = 60,
    F8 = 61,
    G8 = 62,
    H8 = 63,
}

use Square::*;
impl Square {
    #[inline(always)]
    pub fn bitboard(&self) -> BitBoard {
        BitBoard(1u64.wrapping_shl(*self as u32))
    }

    #[inline(always)]
    pub fn rank(&self) -> usize {
        match self {
            A1 | B1 | C1 | D1 | E1 | F1 | G1 | H1 => 0,
            A2 | B2 | C2 | D2 | E2 | F2 | G2 | H2 => 1,
            A3 | B3 | C3 | D3 | E3 | F3 | G3 | H3 => 2,
            A4 | B4 | C4 | D4 | E4 | F4 | G4 | H4 => 3,
            A5 | B5 | C5 | D5 | E5 | F5 | G5 | H5 => 4,
            A6 | B6 | C6 | D6 | E6 | F6 | G6 | H6 => 5,
            A7 | B7 | C7 | D7 | E7 | F7 | G7 | H7 => 6,
            A8 | B8 | C8 | D8 | E8 | F8 | G8 | H8 => 7,
        }
    }

    #[inline(always)]
    pub fn file(&self) -> usize {
        match self {
            A1 | A2 | A3 | A4 | A5 | A6 | A7 | A8 => 0,
            B1 | B2 | B3 | B4 | B5 | B6 | B7 | B8 => 1,
            C1 | C2 | C3 | C4 | C5 | C6 | C7 | C8 => 2,
            D1 | D2 | D3 | D4 | D5 | D6 | D7 | D8 => 3,
            E1 | E2 | E3 | E4 | E5 | E6 | E7 | E8 => 4,
            F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 => 5,
            G1 | G2 | G3 | G4 | G5 | G6 | G7 | G8 => 6,
            H1 | H2 | H3 | H4 | H5 | H6 | H7 | H8 => 7,
        }
    }

    #[inline(always)]
    pub fn offset(&self, file: i8, rank: i8) -> Self {
        Self::try_from((*self as i8 + rank * 8 + file) as u8).unwrap()
    }

    #[inline(always)]
    pub fn try_from(n: u8) -> Option<Self> {
        match n {
            0 => Some(A1),
            1 => Some(B1),
            2 => Some(C1),
            3 => Some(D1),
            4 => Some(E1),
            5 => Some(F1),
            6 => Some(G1),
            7 => Some(H1),
            8 => Some(A2),
            9 => Some(B2),
            10 => Some(C2),
            11 => Some(D2),
            12 => Some(E2),
            13 => Some(F2),
            14 => Some(G2),
            15 => Some(H2),
            16 => Some(A3),
            17 => Some(B3),
            18 => Some(C3),
            19 => Some(D3),
            20 => Some(E3),
            21 => Some(F3),
            22 => Some(G3),
            23 => Some(H3),
            24 => Some(A4),
            25 => Some(B4),
            26 => Some(C4),
            27 => Some(D4),
            28 => Some(E4),
            29 => Some(F4),
            30 => Some(G4),
            31 => Some(H4),
            32 => Some(A5),
            33 => Some(B5),
            34 => Some(C5),
            35 => Some(D5),
            36 => Some(E5),
            37 => Some(F5),
            38 => Some(G5),
            39 => Some(H5),
            40 => Some(A6),
            41 => Some(B6),
            42 => Some(C6),
            43 => Some(D6),
            44 => Some(E6),
            45 => Some(F6),
            46 => Some(G6),
            47 => Some(H6),
            48 => Some(A7),
            49 => Some(B7),
            50 => Some(C7),
            51 => Some(D7),
            52 => Some(E7),
            53 => Some(F7),
            54 => Some(G7),
            55 => Some(H7),
            56 => Some(A8),
            57 => Some(B8),
            58 => Some(C8),
            59 => Some(D8),
            60 => Some(E8),
            61 => Some(F8),
            62 => Some(G8),
            63 => Some(H8),
            _ => None,
        }
    }
}

impl<T, const N: usize> Index<Square> for [T; N] {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: Square) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T, const N: usize> IndexMut<Square> for [T; N] {
    #[inline(always)]
    fn index_mut(&mut self, index: Square) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

macro_rules! square_from_impl {
    ($ty: ty) => {
        impl From<$ty> for Square {
            #[inline(always)]
            fn from(n: $ty) -> Self {
                Self::try_from(n as u8).unwrap()
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
