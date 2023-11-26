use crate::{
    board::Board,
    history_tables::{apply_history_bonus, apply_history_malus, CounterMoveTable, HistoryTable},
    move_sorting::{score_capture, score_quiet},
    moves::{KillerMoves, Move, MoveList, NUM_KILLER_MOVES},
    search::{MINUS_INF, SEARCH_MAX_PLY},
    types::{Color, Piece},
};

#[derive(Clone)]
pub struct SearchStackEntry {
    pub eval: i16,
    pub move_list: MoveList,
    pub killer_moves: KillerMoves<NUM_KILLER_MOVES>,
}
impl Default for SearchStackEntry {
    fn default() -> Self {
        Self {
            eval: MINUS_INF,
            move_list: MoveList::default(),
            killer_moves: KillerMoves::default(),
        }
    }
}

#[derive(Clone)]
pub struct ThreadData {
    pub search_stack: Box<[SearchStackEntry; SEARCH_MAX_PLY]>,
    pub history_tables: Box<[HistoryTable; 2]>,
    pub countermove_tables: Box<[CounterMoveTable; 2]>,
}

impl ThreadData {
    pub fn new() -> Self {
        Self {
            search_stack: Box::new(std::array::from_fn(|_| SearchStackEntry::default())),
            history_tables: Box::new([HistoryTable::default(); 2]),
            countermove_tables: Box::new([CounterMoveTable::default(); 2]),
        }
    }

    pub fn update_histories(
        &mut self,
        player: Color,
        delta: i16,
        bonus_quiet: Move,
        malus_quiets: &MoveList,
    ) {
        // reward quiets that produce a beta cutoff
        apply_history_bonus(&mut self.history_tables[player][bonus_quiet], delta);

        // punish quiets that were played but didn't cause a beta cutoff
        for smv in malus_quiets.inner().iter() {
            let malus_quiet = smv.mv;
            debug_assert!(malus_quiet != bonus_quiet);
            apply_history_malus(&mut self.history_tables[player][malus_quiet], delta);
        }
    }

    pub fn score_moves(&mut self, board: &Board, ply: usize, last_move: Move) {
        for m in self.search_stack[ply].move_list.inner_mut() {
            if m.mv.promotion() != Piece::Pawn || board.is_capture(m.mv) {
                m.score = score_capture(board, m.mv);
            } else {
                m.score = score_quiet(
                    board,
                    &self.search_stack[ply].killer_moves,
                    &self.history_tables,
                    &self.countermove_tables,
                    last_move,
                    m.mv,
                )
            }
        }
    }
}
