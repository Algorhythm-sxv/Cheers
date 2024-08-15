use crate::{
    board::{see::SEE_PIECE_VALUES, Board},
    history_tables::{
        apply_history_bonus, apply_history_malus, CorrectionHistoryTable, CounterMoveTable,
        HistoryTable, CORRHIST_MAX, CORRHIST_TABLE_UNIT,
    },
    moves::*,
    search::{MINUS_INF, SEARCH_MAX_PLY},
    types::Color,
};

#[derive(Clone)]
pub struct SearchStackEntry {
    pub eval: i16,
    pub captures: MoveList,
    pub quiets: MoveList,
    pub current_move: Move,
    pub killer_moves: KillerMoves<NUM_KILLER_MOVES>,
}
impl Default for SearchStackEntry {
    fn default() -> Self {
        Self {
            eval: MINUS_INF,
            captures: MoveList::default(),
            quiets: MoveList::default(),
            current_move: Move::null(),
            killer_moves: KillerMoves::default(),
        }
    }
}

impl SearchStackEntry {
    pub fn num_moves(&self) -> usize {
        self.captures.len() + self.quiets.len()
    }
}

const CONTHIST_MAX: usize = 2;
#[derive(Clone)]
pub struct ThreadData {
    pub search_stack: Box<[SearchStackEntry]>,
    pub history_tables: Box<[HistoryTable; 2]>,
    pub capture_history_tables: Box<[HistoryTable; 2]>,
    pub conthist_tables: Box<[[[[HistoryTable; 64]; 6]; 2]; CONTHIST_MAX]>,
    pub countermove_tables: Box<[CounterMoveTable; 2]>,
    pub corrhist_table: Box<CorrectionHistoryTable>,
}

impl ThreadData {
    pub fn new() -> Self {
        Self {
            search_stack: vec![SearchStackEntry::default(); SEARCH_MAX_PLY].into_boxed_slice(),
            history_tables: Box::new([HistoryTable::default(); 2]),
            capture_history_tables: Box::new([HistoryTable::default(); 2]),
            conthist_tables: Box::new([[[[HistoryTable::default(); 64]; 6]; 2]; CONTHIST_MAX]),
            countermove_tables: Box::new([CounterMoveTable::default(); 2]),
            corrhist_table: Box::new(CorrectionHistoryTable::default()),
        }
    }

    pub fn update_quiet_history(
        &mut self,
        player: Color,
        delta: i16,
        bonus_quiet: Move,
        malus_quiets: &MoveList,
        ply: usize,
    ) {
        let mut conthist_moves = [None; CONTHIST_MAX];
        for i in 0..CONTHIST_MAX {
            conthist_moves[i] = ply
                .checked_sub(i + 1)
                .map(|p| self.search_stack[p].current_move);
        }

        // reward quiets that produce a beta cutoff
        for i in 0..CONTHIST_MAX {
            if let Some(cm) = conthist_moves[i] {
                apply_history_bonus(
                    &mut self.conthist_tables[i][player][cm.piece()][cm.to()][bonus_quiet],
                    delta,
                )
            } else {
                break; // once we miss a conthist move there will be no more
            }
        }

        apply_history_bonus(&mut self.history_tables[player][bonus_quiet], delta);

        // punish quiets that were played but didn't cause a beta cutoff
        for smv in malus_quiets.inner().iter() {
            let malus_quiet = smv.mv;
            debug_assert!(malus_quiet != bonus_quiet);
            for i in 0..CONTHIST_MAX {
                if let Some(cm) = conthist_moves[i] {
                    apply_history_malus(
                        &mut self.conthist_tables[i][player][cm.piece()][cm.to()][malus_quiet],
                        delta,
                    );
                } else {
                    break; // once we miss a conthist move there will be no more
                }
            }

            apply_history_malus(&mut self.history_tables[player][malus_quiet], delta);
        }
    }

    pub fn update_capture_history(
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

    pub fn update_correction_history(&mut self, board: &Board, depth: i8, diff: i16) {
        let hist = self
            .corrhist_table
            .get_mut(board.current_player(), board.pawn_hash());
        let diff = diff * CORRHIST_TABLE_UNIT;
        let weight = 16.min(1 + depth as i16);

        let update = *hist * (CORRHIST_TABLE_UNIT - weight) + diff * weight;
        *hist = i16::clamp(update / CORRHIST_TABLE_UNIT, -CORRHIST_MAX, CORRHIST_MAX);
    }

    pub fn corrected_eval(&self, board: &Board, eval: i16) -> i16 {
        let hist = self
            .corrhist_table
            .get(board.current_player(), board.pawn_hash());
        eval + hist / CORRHIST_TABLE_UNIT
    }

    pub fn score_moves(&mut self, board: &Board, ply: usize) {
        for i in 0..self.search_stack[ply].captures.len() {
            let mv = self.search_stack[ply].captures[i];

            *self.search_stack[ply].captures.score(i) = self.score_capture(board, mv);
        }
        for i in 0..self.search_stack[ply].quiets.len() {
            let mv = self.search_stack[ply].quiets[i];

            *self.search_stack[ply].quiets.score(i) = self.score_quiet(board, ply, mv);
        }
    }

    pub fn get_quiet_history(&self, mv: Move, current_player: Color, ply: usize) -> i16 {
        let mut history = self.history_tables[current_player][mv];
        if ply > 0 {
            for i in 0..CONTHIST_MAX {
                if let Some(cm) = ply
                    .checked_sub(i + 1)
                    .map(|p| self.search_stack[p].current_move)
                {
                    history += self.conthist_tables[i][current_player][cm.piece()][cm.to()][mv];
                } else {
                    break;
                }
            }
        }
        history
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
            let mut score = QUIET_SCORE + (self.history_tables[current_player][mv] as i32);

            for i in 0..CONTHIST_MAX {
                if let Some(cm) = ply
                    .checked_sub(i + 1)
                    .map(|p| self.search_stack[p].current_move)
                {
                    score += self.conthist_tables[i][current_player][cm.piece()][cm.to()][mv] as i32
                } else {
                    break;
                }
            }
            score
        }
    }
}

impl Default for ThreadData {
    fn default() -> Self {
        Self::new()
    }
}
