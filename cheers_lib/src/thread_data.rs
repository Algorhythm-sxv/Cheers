use crate::{
    board::{see::SEE_PIECE_VALUES, Board},
    history_tables::{apply_history_bonus, apply_history_malus, CounterMoveTable, HistoryTable},
    moves::*,
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
    pub search_stack: Box<[SearchStackEntry]>,
    pub history_tables: Box<[HistoryTable; 2]>,
    pub capture_history_tables: Box<[HistoryTable; 2]>,
    pub countermove_history_tables: Box<[[[HistoryTable; 64]; 6]; 2]>,
    pub continuation_history_tables: Box<[[[HistoryTable; 64]; 6]; 2]>,
    pub countermove_tables: Box<[CounterMoveTable; 2]>,
}

impl ThreadData {
    pub fn new() -> Self {
        Self {
            search_stack: vec![SearchStackEntry::default(); SEARCH_MAX_PLY].into_boxed_slice(),
            history_tables: Box::new([HistoryTable::default(); 2]),
            capture_history_tables: Box::new([HistoryTable::default(); 2]),
            countermove_history_tables: Box::new([[[HistoryTable::default(); 64]; 6]; 2]),
            continuation_history_tables: Box::new([[[HistoryTable::default(); 64]; 6]; 2]),
            countermove_tables: Box::new([CounterMoveTable::default(); 2]),
        }
    }

    pub fn update_quiet_histories(
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
            .get(ply.wrapping_sub(1))
            .map(|s| s.current_move)
            .unwrap_or(Move::null());
        let continuation_move = self
            .search_stack
            .get(ply.wrapping_sub(2))
            .map(|s| s.current_move)
            .unwrap_or(Move::null());

        if !countermove.is_null() {
            apply_history_bonus(
                &mut self.countermove_history_tables[player][countermove.piece()][countermove.to()]
                    [bonus_quiet],
                delta,
            )
        }

        if !continuation_move.is_null() {
            apply_history_bonus(
                &mut self.continuation_history_tables[player][continuation_move.piece()]
                    [continuation_move.to()][bonus_quiet],
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
            if !continuation_move.is_null() {
                apply_history_malus(
                    &mut self.continuation_history_tables[player][continuation_move.piece()]
                        [continuation_move.to()][malus_quiet],
                    delta,
                )
            }
            apply_history_malus(&mut self.history_tables[player][malus_quiet], delta);
        }
    }

    pub fn update_capture_histories(
        &mut self,
        player: Color,
        delta: i16,
        bonus_capture: Option<Move>,
        malus_captures: &MoveList,
    ) {
        if let Some(bonus_capture) = bonus_capture {
            apply_history_bonus(
                &mut self.capture_history_tables[player][bonus_capture],
                delta,
            );
        }

        // punish quiets that were played but didn't cause a beta cutoff
        for smv in malus_captures.inner().iter() {
            let malus_capture = smv.mv;
            debug_assert!(Some(malus_capture) != bonus_capture);
            apply_history_malus(
                &mut self.capture_history_tables[player][malus_capture],
                delta,
            );
        }
    }

    pub fn score_moves(&mut self, board: &Board, ply: usize, captures_only: bool) {
        // for m in self.search_stack[ply].move_list.inner_mut() {
        for i in 0..self.search_stack[ply].move_list.len() {
            let mv = self.search_stack[ply].move_list[i];

            let score = if captures_only || mv.promotion() != Piece::Pawn || board.is_capture(mv) {
                self.score_capture(board, mv)
            } else {
                self.score_quiet(board, ply, mv)
            };

            *self.search_stack[ply].move_list.score(i) = score
        }
    }

    pub fn score_capture(&self, board: &Board, mv: Move) -> i32 {
        use crate::types::Piece::*;
        // filter out underpromotions
        if matches!(mv.promotion(), Knight | Bishop | Rook) {
            return UNDERPROMO_SCORE + (SEE_PIECE_VALUES[mv.promotion()] as i32);
        }
        let piece_bonuses = [0, 240, 240, 480, 960];
        let mvv_bonus = 2 * piece_bonuses[board.piece_on(mv.to()).unwrap_or(Pawn)];
        let capture_history = self.capture_history_tables[board.current_player()][mv] as i32;

        // sort winning captures before quiets, losing captures after
        if board.see_beats_threshold(mv, 0) {
            WINNING_CAPTURE_SCORE + 50_000 + capture_history + mvv_bonus
        } else {
            LOSING_CAPTURE_SCORE + 50_000 + capture_history + mvv_bonus
        }
    }

    pub fn score_quiet(&self, board: &Board, ply: usize, mv: Move) -> i32 {
        let current_player = board.current_player();
        if self.search_stack[ply].killer_moves.contains(&mv) {
            // there can be more than 1 killer move, so sort them by their respective histories
            KILLER_MOVE_SCORE + (self.history_tables[current_player][mv] as i32)
        } else if self.countermove_tables[current_player][self
            .search_stack
            .get(ply.wrapping_sub(1))
            .map(|s| s.current_move)
            .unwrap_or(Move::null())]
            == mv
        {
            COUNTERMOVE_SCORE
        } else {
            let countermove_score = if let Some(countermove) = self
                .search_stack
                .get(ply.wrapping_sub(1))
                .map(|s| s.current_move)
            {
                self.countermove_history_tables[current_player][countermove.piece()]
                    [countermove.to()][mv] as i32
            } else {
                0
            };
            let continuation_score = if let Some(continuation_move) = self
                .search_stack
                .get(ply.wrapping_sub(2))
                .map(|s| s.current_move)
            {
                self.continuation_history_tables[current_player][continuation_move.piece()]
                    [continuation_move.to()][mv] as i32
            } else {
                0
            };
            QUIET_SCORE
                + (self.history_tables[current_player][mv] as i32)
                + countermove_score
                + continuation_score
        }
    }
}

impl Default for ThreadData {
    fn default() -> Self {
        Self::new()
    }
}
