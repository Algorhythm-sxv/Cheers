use std::marker::PhantomData;

use crate::{board::Board, moves::*, thread_data::ThreadData, types::TypeMoveGen};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Stage {
    TTMove,
    GenerateNoisy,
    SortNoisy,
    GenerateQuiet,
    SortQuiet,
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
                Stage::GenerateNoisy
            },
            tt_move,
            index: 0,
            _captures: PhantomData,
        }
    }

    pub fn next(
        &mut self,
        board: &Board,
        thread_data: &mut ThreadData,
        ply: usize,
    ) -> Option<(Move, i32)> {
        // return the TT move first if it is pseudolegal and pray that there is no hash collision
        // a beta cutoff here could skip movegen altogether
        if self.stage == Stage::TTMove {
            self.stage = Stage::GenerateNoisy;

            if board.is_pseudolegal(self.tt_move) {
                return Some((self.tt_move, TT_MOVE_SCORE));
            }
        }

        // generate the moves as desired and score them all
        if self.stage == Stage::GenerateNoisy {
            self.stage = Stage::SortNoisy;
            board.generate_legal_noisy_into(&mut thread_data.search_stack[ply].move_list);
            thread_data.score_moves(board, ply, true);
        }

        // find the move with the next highest sort score
        // or return None if the end of the list has been reached
        loop {
            if self.stage == Stage::GenerateQuiet {
                self.stage = Stage::SortQuiet;
                self.index = 0;
                board.generate_legal_quiet_into(&mut thread_data.search_stack[ply].move_list);
                thread_data.score_moves(board, ply, false);
            }
            if self.index < thread_data.search_stack[ply].move_list.len() {
                let (mv, score) = thread_data.search_stack[ply]
                    .move_list
                    .pick_move(self.index);
                self.index += 1;
                // tt move has already been reported
                if mv == self.tt_move {
                    continue;
                }
                return Some((mv, score));
            } else {
                if self.stage == Stage::SortNoisy && !M::CAPTURES {
                    self.stage = Stage::GenerateQuiet;
                    continue;
                } else {
                    return None;
                }
            }
        }
    }
}
