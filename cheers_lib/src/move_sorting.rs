use std::marker::PhantomData;

use crate::{
    board::{
        evaluate::{relative_board_index, EVAL_PARAMS},
        see::{MVV_LVA, SEE_PIECE_VALUES},
        Board,
    },
    history_tables::{CounterMoveTable, HistoryTable},
    moves::*,
    thread_data::ThreadData,
    types::{Black, Color, Piece::*, TypeMoveGen, White},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Stage {
    TTMove,
    GenerateMoves,
    SortMoves,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MoveSorter<M: TypeMoveGen> {
    stage: Stage,
    tt_move: Move,
    index: usize,
    _captures: PhantomData<M>,
}

impl<M: TypeMoveGen> MoveSorter<M> {
    pub fn new(tt_move: Move) -> Self {
        Self {
            stage: if !tt_move.is_null() {
                Stage::TTMove
            } else {
                Stage::GenerateMoves
            },
            tt_move,
            index: 0,
            _captures: PhantomData::default(),
        }
    }

    pub fn next(
        &mut self,
        board: &Board,
        thread_data: &mut ThreadData,
        ply: usize,
        last_move: Move,
    ) -> Option<(Move, i32)> {
        // return the TT move first if it is pseudolegal and pray that there is no hash collision
        // a beta cutoff here could skip movegen altogether
        if self.stage == Stage::TTMove {
            self.stage = Stage::GenerateMoves;

            if board.is_pseudolegal(self.tt_move) {
                return Some((self.tt_move, TT_MOVE_SCORE));
            }
        }

        // generate the moves as desired and score them all
        if self.stage == Stage::GenerateMoves {
            self.stage = Stage::SortMoves;
            if M::CAPTURES {
                board.generate_legal_captures_into(&mut thread_data.search_stack[ply].move_list);
                for m in thread_data.search_stack[ply].move_list.inner_mut() {
                    m.score = score_capture(board, m.mv);
                }
            } else {
                board.generate_legal_moves_into(&mut thread_data.search_stack[ply].move_list);
                thread_data.score_moves(board, ply, last_move);
            }
        }

        // find the move with the next highest sort score
        // or return None if the end of the list has been reached
        if self.index < thread_data.search_stack[ply].move_list.len() {
            let (mut mv, mut score) = thread_data.search_stack[ply]
                .move_list
                .pick_move(self.index);
            // tt move has already been reported
            if mv == self.tt_move {
                self.index += 1;
                if self.index < thread_data.search_stack[ply].move_list.len() {
                    (mv, score) = thread_data.search_stack[ply]
                        .move_list
                        .pick_move(self.index);
                } else {
                    return None;
                }
            }
            self.index += 1;
            Some((mv, score))
        } else {
            None
        }
    }
}

pub fn score_capture(board: &Board, mv: Move) -> i32 {
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

pub fn score_quiet(
    board: &Board,
    killer_moves: &KillerMoves<NUM_KILLER_MOVES>,
    history_tables: &[HistoryTable; 2],
    countermove_tables: &[CounterMoveTable; 2],
    last_move: Move,
    mv: Move,
) -> i32 {
    let current_player = board.current_player();
    if killer_moves.contains(&mv) {
        // there can be more than 1 killer move, so sort them by their respective histories
        KILLER_MOVE_SCORE + (history_tables[current_player][mv] as i32)
    } else if countermove_tables[current_player][last_move] == mv {
        COUNTERMOVE_SCORE
    } else {
        QUIET_SCORE + (history_tables[current_player][mv] as i32)
    }
}
