use std::marker::PhantomData;

use crate::{board::Board, moves::*, thread_data::ThreadData, types::TypeMoveGen};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Stage {
    TTMove,
    GenerateMoves,
    YieldGoodCaptures,
    YieldQuiets,
    YieldBadCaptures,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MoveSorter<M: TypeMoveGen> {
    stage: Stage,
    tt_move: Move,
    capture_index: usize,
    quiet_index: usize,
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
            capture_index: 0,
            quiet_index: 0,
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
            self.stage = Stage::GenerateMoves;

            if board.is_pseudolegal(self.tt_move) {
                return Some((self.tt_move, TT_MOVE_SCORE));
            }
        }

        // generate the moves as desired and score them all
        if self.stage == Stage::GenerateMoves {
            self.stage = Stage::YieldGoodCaptures;
            if M::CAPTURES {
                board.generate_legal_captures_into(&mut thread_data.search_stack[ply].captures);
                thread_data.search_stack[ply].quiets.reset();
            } else {
                board.generate_legal_moves_into(
                    &mut thread_data.search_stack[ply].captures,
                    &mut thread_data.search_stack[ply].quiets,
                );
            }
            thread_data.score_moves(board, ply);
        }

        // find the move with the next highest sort score
        // or return None if the end of the list has been reached
        if self.stage == Stage::YieldGoodCaptures {
            loop {
                if self.capture_index < thread_data.search_stack[ply].captures.len() {
                    let (mv, score) = thread_data.search_stack[ply]
                        .captures
                        .pick_move(self.capture_index);
                    self.capture_index += 1;
                    if mv == self.tt_move {
                        continue;
                    }

                    // reached the bad captures, replace the current move and skip to quietts
                    if score < 0 {
                        self.capture_index -= 1;
                        self.stage = Stage::YieldQuiets;
                        break;
                    }

                    return Some((mv, score));
                } else {
                    self.stage = Stage::YieldQuiets;
                    break;
                }
            }
        }
        if self.stage == Stage::YieldQuiets {
            loop {
                if self.quiet_index < thread_data.search_stack[ply].quiets.len() {
                    let (mv, score) = thread_data.search_stack[ply]
                        .quiets
                        .pick_move(self.quiet_index);
                    self.quiet_index += 1;
                    if mv == self.tt_move {
                        continue;
                    }

                    return Some((mv, score));
                } else {
                    self.stage = Stage::YieldBadCaptures;
                    break;
                }
            }
        }
        if self.stage == Stage::YieldBadCaptures {
            loop {
                if self.capture_index < thread_data.search_stack[ply].captures.len() {
                    let (mv, score) = thread_data.search_stack[ply]
                        .captures
                        .pick_move(self.capture_index);
                    self.capture_index += 1;
                    if mv == self.tt_move {
                        continue;
                    }

                    return Some((mv, score));
                } else {
                    // should be last moves yielded
                    debug_assert!(
                        self.capture_index == thread_data.search_stack[ply].captures.len()
                    );
                    debug_assert!(self.quiet_index == thread_data.search_stack[ply].quiets.len());
                    return None;
                }
            }
        }

        None
    }
}
