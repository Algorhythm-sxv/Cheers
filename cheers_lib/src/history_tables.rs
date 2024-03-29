use std::ops::{Index, IndexMut};

use crate::moves::Move;

pub const HISTORY_MAX: i16 = 4096;

#[derive(Copy, Clone, Debug)]
pub struct HistoryTable([[i16; 64]; 6]);

impl Default for HistoryTable {
    fn default() -> Self {
        Self([[0; 64]; 6])
    }
}

impl Index<Move> for HistoryTable {
    type Output = i16;

    #[inline(always)]
    fn index(&self, mv: Move) -> &Self::Output {
        &self.0[mv.piece()][mv.to()]
    }
}

impl IndexMut<Move> for HistoryTable {
    #[inline(always)]
    fn index_mut(&mut self, mv: Move) -> &mut Self::Output {
        &mut self.0[mv.piece()][mv.to()]
    }
}

#[inline(always)]
pub fn apply_history_bonus(score: &mut i16, delta: i16) {
    *score += (delta as i32 - (delta as i32 * *score as i32) / HISTORY_MAX as i32) as i16;
}

#[inline(always)]
pub fn apply_history_malus(score: &mut i16, delta: i16) {
    *score -= (delta as i32 + (delta as i32 * *score as i32) / HISTORY_MAX as i32) as i16;
}

#[derive(Copy, Clone, Debug)]
pub struct CounterMoveTable([[Move; 64]; 6]);

impl Default for CounterMoveTable {
    fn default() -> Self {
        Self([[Move::null(); 64]; 6])
    }
}

impl Index<Move> for CounterMoveTable {
    type Output = Move;

    fn index(&self, mv: Move) -> &Self::Output {
        &self.0[mv.piece()][mv.to()]
    }
}

impl IndexMut<Move> for CounterMoveTable {
    fn index_mut(&mut self, mv: Move) -> &mut Self::Output {
        &mut self.0[mv.piece()][mv.to()]
    }
}

#[cfg(test)]
mod tests {
    use crate::history_tables::{apply_history_bonus, apply_history_malus, HISTORY_MAX};

    #[test]
    fn test_history_bonus() {
        let mut score = 0;
        apply_history_bonus(&mut score, 16);
        assert_eq!(score, 16);
        score = HISTORY_MAX;
        apply_history_bonus(&mut score, 16);
        assert_eq!(score, HISTORY_MAX);
        score = -HISTORY_MAX;
        apply_history_bonus(&mut score, 16);
        assert_eq!(score, -HISTORY_MAX + 32);
    }

    #[test]
    fn test_history_malus() {
        let mut score = 0;
        apply_history_malus(&mut score, 16);
        assert_eq!(score, -16);
        score = HISTORY_MAX;
        apply_history_malus(&mut score, 16);
        assert_eq!(score, HISTORY_MAX - 32);
        score = -HISTORY_MAX;
        apply_history_malus(&mut score, 16);
        assert_eq!(score, -HISTORY_MAX);
    }
}
