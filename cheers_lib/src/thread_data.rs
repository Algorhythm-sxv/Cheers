use crate::{
    board::{
        evaluate::{relative_board_index, EVAL_PARAMS},
        see::{MVV_LVA, SEE_PIECE_VALUES},
        Board,
    },
    history_tables::{apply_history_bonus, apply_history_malus, CounterMoveTable, HistoryTable},
    moves::*,
    search::{MINUS_INF, SEARCH_MAX_PLY},
    types::{Black, Color, Piece, White},
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
                Self::score_capture(board, mv)
            } else {
                self.score_quiet(board, ply, mv)
            };

            *self.search_stack[ply].move_list.score(i) = score
        }
    }

    pub fn score_capture(board: &Board, mv: Move) -> i32 {
        use crate::types::Piece::*;
        // filter out underpromotions
        if matches!(mv.promotion(), Knight | Bishop | Rook) {
            return UNDERPROMO_SCORE + (SEE_PIECE_VALUES[mv.promotion()] as i32);
        }
        let mvv_lva = if mv.promotion() == Queen {
            MVV_LVA[Queen][Pawn]
        } else {
            MVV_LVA[board.piece_on(mv.to()).unwrap_or(Pawn)][mv.piece()]
        };
        let relative_square = if board.current_player() == Color::White {
            relative_board_index::<White>(mv.to())
        } else {
            relative_board_index::<Black>(mv.to())
        };
        let psqt_score = EVAL_PARAMS.piece_tables[(mv.piece(), relative_square)].mg() as i32 / 16;

        // sort all captures before quiets
        WINNING_CAPTURE_SCORE + 1000 * (mvv_lva as i32) + psqt_score
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
