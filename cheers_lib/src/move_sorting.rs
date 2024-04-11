use std::marker::PhantomData;

use crate::{board::Board, moves::*, thread_data::ThreadData, types::TypeMoveGen};

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
    moves_given: usize,
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
            moves_given: 0,
            _captures: PhantomData,
        }
    }

    pub fn next(
        &mut self,
        board: &Board,
        thread_data: &mut ThreadData,
        ply: usize,
    ) -> Option<(Move, usize)> {
        // return the TT move first if it is pseudolegal and pray that there is no hash collision
        // a beta cutoff here could skip movegen altogether
        if self.stage == Stage::TTMove {
            self.stage = Stage::GenerateMoves;

            if board.is_pseudolegal(self.tt_move) {
                let move_index = self.moves_given;
                self.moves_given += 1;
                return Some((self.tt_move, move_index));
            }
        }

        // generate the moves as desired and score them all
        if self.stage == Stage::GenerateMoves {
            self.stage = Stage::SortMoves;
            if M::CAPTURES {
                board.generate_legal_noisy_into(&mut thread_data.search_stack[ply].move_list);
                thread_data.score_moves(board, ply, true);
            } else {
                board.generate_legal_moves_into(&mut thread_data.search_stack[ply].move_list);
                thread_data.score_moves(board, ply, false);
            }
        }

        // find the move with the next highest sort score
        // or return None if the end of the list has been reached
        loop {
            match thread_data.search_stack[ply].move_list.pick_move() {
                Some((mv, _)) => {
                    // if mv == self.tt_move {
                    //     continue;
                    // }

                    let move_index = self.moves_given;
                    self.moves_given += 1;
                    return Some((mv, move_index));
                }
                None => return None,
            }
        }
    }
}
