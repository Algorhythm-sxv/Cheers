use std::marker::PhantomData;

use crate::{board::Board, moves::*, thread_data::ThreadData, types::TypeMoveGen};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Stage {
    TTMove,
    GenerateNoisy,
    WinningNoisy,
    GenerateQuiet,
    Quiet,
    LosingNoisy,
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
                Stage::GenerateNoisy
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
        // may need to try more than once
        loop {
            // return the TT move first if it is pseudolegal and pray that there is no hash collision
            // a beta cutoff here could skip movegen altogether
            if self.stage == Stage::TTMove {
                self.stage = Stage::GenerateNoisy;

                if board.is_pseudolegal(self.tt_move) {
                    let move_index = self.moves_given;
                    self.moves_given += 1;
                    return Some((self.tt_move, move_index));
                }
            }

            // generate the moves as desired and score them all
            if self.stage == Stage::GenerateNoisy {
                self.stage = Stage::WinningNoisy;
                board.generate_legal_noisy_into(&mut thread_data.search_stack[ply].noisy_move_list);
                thread_data.score_noisies(board, ply);
            }

            if self.stage == Stage::GenerateQuiet {
                self.stage = Stage::Quiet;
                board.generate_legal_quiet_into(&mut thread_data.search_stack[ply].move_list);
                thread_data.score_quiets(board, ply);
            }

            // find the move with the next highest sort score
            // or return None if the end of the list has been reached
            let move_list = match self.stage {
                Stage::WinningNoisy | Stage::LosingNoisy => {
                    &mut thread_data.search_stack[ply].noisy_move_list
                }
                _ => &mut thread_data.search_stack[ply].move_list,
            };
            match move_list.pick_move() {
                Some((mv, score)) => {
                    // move between noisy and quiet moves
                    if self.stage == Stage::WinningNoisy && score < QUIET_SCORE {
                        // if we're only giving captures, let it continue
                        if M::CAPTURES {
                            self.stage = Stage::LosingNoisy;
                        } else {
                            // put the move back in for later
                            move_list.unpick_move();
                            self.stage = Stage::GenerateQuiet;
                            continue;
                        }
                    }
                    // don't report the TT move more than once
                    if mv == self.tt_move {
                        continue;
                    }

                    let move_index = self.moves_given;
                    self.moves_given += 1;
                    return Some((mv, move_index));
                }
                None => match self.stage {
                    Stage::WinningNoisy => {
                        if M::CAPTURES {
                            self.stage = Stage::LosingNoisy;
                            return None;
                        } else {
                            self.stage = Stage::GenerateQuiet;
                            continue;
                        }
                    }
                    Stage::Quiet => {
                        self.stage = Stage::LosingNoisy;
                        continue;
                    }
                    _ => return None,
                },
            }
        }
    }
}
