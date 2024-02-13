use crate::{
    board::Board,
    history_tables::{apply_history_bonus, apply_history_malus, CounterMoveTable, HistoryTable},
    move_sorting::score_capture,
    moves::{
        KillerMoves, Move, MoveList, COUNTERMOVE_SCORE, KILLER_MOVE_SCORE, NUM_KILLER_MOVES,
        QUIET_SCORE,
    },
    search::{MINUS_INF, SEARCH_MAX_PLY},
    types::{Color, Piece},
};

#[derive(Clone)]
pub struct SearchStackEntry {
    pub eval: i16,
    pub move_list: MoveList,
    pub current_move: Move,
    pub killer_moves: KillerMoves<NUM_KILLER_MOVES>,
}
impl Default for SearchStackEntry {
    fn default() -> Self {
        Self {
            eval: MINUS_INF,
            move_list: MoveList::default(),
            current_move: Move::null(),
            killer_moves: KillerMoves::default(),
        }
    }
}

#[derive(Clone)]
pub struct ThreadData {
    pub search_stack: Box<[SearchStackEntry; SEARCH_MAX_PLY]>,
    pub history_tables: Box<[HistoryTable; 2]>,
    pub countermove_history_tables: Box<[[[HistoryTable; 64]; 6]; 2]>,
    pub countermove_tables: Box<[CounterMoveTable; 2]>,
}

impl ThreadData {
    pub fn new() -> Self {
        Self {
            search_stack: Box::new(std::array::from_fn(|_| SearchStackEntry::default())),
            history_tables: Box::new([HistoryTable::default(); 2]),
            countermove_history_tables: Box::new([[[HistoryTable::default(); 64]; 6]; 2]),
            countermove_tables: Box::new([CounterMoveTable::default(); 2]),
        }
    }

    pub fn update_histories(
        &mut self,
        player: Color,
        delta: i16,
        bonus_quiet: Move,
        malus_quiets: &MoveList,
        ply: usize,
    ) {
        // reward quiets that produce a beta cutoff
        let countermove = self
            .search_stack
            .get(ply - 1)
            .map(|s| s.current_move)
            .unwrap_or(Move::null());
        if !countermove.is_null() {
            apply_history_bonus(
                &mut self.countermove_history_tables[player][countermove.piece()][countermove.to()]
                    [bonus_quiet],
                delta,
            )
        }
        apply_history_bonus(&mut self.history_tables[player][bonus_quiet], delta);

        // punish quiets that were played but didn't cause a beta cutoff
        for smv in malus_quiets.inner().iter() {
            let malus_quiet = smv.mv;
            debug_assert!(malus_quiet != bonus_quiet);
            if !countermove.is_null() {
                apply_history_malus(
                    &mut self.countermove_history_tables[player][countermove.piece()]
                        [countermove.to()][malus_quiet],
                    delta,
                )
            }
            apply_history_malus(&mut self.history_tables[player][malus_quiet], delta);
        }
    }

    pub fn score_moves(&mut self, board: &Board, ply: usize) {
        // for m in self.search_stack[ply].move_list.inner_mut() {
        for i in 0..self.search_stack[ply].move_list.len() {
            let mv = self.search_stack[ply].move_list[i];

            let score = if mv.promotion() != Piece::Pawn || board.is_capture(mv) {
                score_capture(board, mv)
            } else {
                self.score_quiet(board, ply, mv)
            };

            *self.search_stack[ply].move_list.score(i) = score
        }
    }

    pub fn score_quiet(&self, board: &Board, ply: usize, mv: Move) -> i32 {
        let current_player = board.current_player();
        if self.search_stack[ply].killer_moves.contains(&mv) {
            // there can be more than 1 killer move, so sort them by their respective histories
            KILLER_MOVE_SCORE + (self.history_tables[current_player][mv] as i32)
        } else if self.countermove_tables[current_player][self
            .search_stack
            .get(ply - 1)
            .map(|s| s.current_move)
            .unwrap_or(Move::null())]
            == mv
        {
            // TODO: Prevent countermove scoring at root (ply > 0 condition)
            COUNTERMOVE_SCORE
        } else {
            let countermove_score =
                if let Some(countermove) = self.search_stack.get(ply - 1).map(|s| s.current_move) {
                    self.countermove_history_tables[current_player][countermove.piece()]
                        [countermove.to()][mv] as i32
                } else {
                    0
                };
            QUIET_SCORE + (self.history_tables[current_player][mv] as i32) + countermove_score
        }
    }
}

impl Default for ThreadData {
    fn default() -> Self {
        Self::new()
    }
}
