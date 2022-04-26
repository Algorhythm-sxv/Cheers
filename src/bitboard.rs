use std::{
    fmt::{Debug, Display},
    ops,
};

use overload::overload;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct BitBoard(pub u64);

impl BitBoard {
    #[inline]
    pub fn empty() -> Self {
        Self(0)
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
    #[inline]
    pub fn is_not_empty(&self) -> bool {
        self.0 != 0
    }
    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
    #[inline]
    pub fn inverse(&self) -> Self {
        Self(!self.0)
    }
    #[inline]
    pub fn lsb_index(&self) -> u32 {
        self.0.trailing_zeros()
    }
    #[inline]
    pub fn clear_lsb(&mut self) {
        self.0 &= self.0 - 1;
    }
    #[inline]
    pub fn count_ones(&self) -> u32 {
        self.0.count_ones()
    }
}

impl Iterator for BitBoard {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            None
        } else {
            let i = self.lsb_index();
            self.clear_lsb();
            Some(i as u8)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count_ones() as usize, Some(self.count_ones() as usize))
    }
}
impl ExactSizeIterator for BitBoard {}

impl Debug for BitBoard {
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

impl Display for BitBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

overload!((a: ?BitBoard) & (b: ?BitBoard) -> BitBoard {BitBoard(a.0 & b.0)});
overload!((a: &mut BitBoard) &= (b: ?BitBoard) {a.0 &= b.0});

overload!((a: ?BitBoard) | (b: ?BitBoard) -> BitBoard {BitBoard(a.0 | b.0)});
overload!((a: &mut BitBoard) |= (b: ?BitBoard) {a.0 |= b.0});

overload!((a: ?BitBoard) ^ (b: ?BitBoard) -> BitBoard {BitBoard(a.0 ^ b.0)});
overload!((a: &mut BitBoard) ^= (b: ?BitBoard) {a.0 ^= b.0});

overload!((a: ?BitBoard) << (b: ?u64) -> BitBoard {BitBoard(a.0 << b)});
overload!((a: ?BitBoard) >> (b: ?u64) -> BitBoard {BitBoard(a.0 >> b)});
