use std::marker::PhantomData;

use crate::{
    board::{
        see::{MVV_LVA, SEE_PIECE_VALUES},
        Board,
    },
    moves::{Move, MoveList, MoveScore, NUM_KILLER_MOVES},
    types::{Piece::*, TypeMoveGen},
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
            stage: if tt_move != Move::null() {
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
        killers: &[Move; NUM_KILLER_MOVES],
        counters: &[[[Move; 64]; 6]; 2],
        history: &[[[i16; 64]; 6]; 2],
        last_move: Move,
        list: &mut MoveList,
    ) -> Option<(Move, MoveScore)> {
        // return the TT move first if it is pseudolegal and pray that there is no hash collision
        // a beta cutoff here could skip movegen altogether
        if self.stage == Stage::TTMove {
            self.stage = Stage::GenerateMoves;

            if board.is_pseudolegal(self.tt_move) {
                return Some((self.tt_move, MoveScore::TTMove));
            }
        }

        // generate the moves as desired and score them all
        if self.stage == Stage::GenerateMoves {
            self.stage = Stage::SortMoves;
            if M::CAPTURES {
                board.generate_legal_captures_into(list);
                for m in list.inner_mut() {
                    m.score = score_capture(board, m.mv);
                }
            } else {
                board.generate_legal_moves_into(list);
                for m in list.inner_mut() {
                    if m.mv.promotion != Pawn || board.is_capture(m.mv) {
                        m.score = score_capture(board, m.mv);
                    } else {
                        m.score = score_quiet(board, killers, counters, history, last_move, m.mv);
                    }
                }
            }
        }

        // find the move with the next highest sort score
        // or return None if the end of the list has been reached
        if self.index < list.len() {
            let (mut mv, mut score) = list.pick_move(self.index);
            // tt move has already been reported
            if mv == self.tt_move {
                self.index += 1;
                if self.index < list.len() {
                    (mv, score) = list.pick_move(self.index);
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

fn score_capture(board: &Board, mv: Move) -> MoveScore {
    // filter out underpromotions
    if matches!(mv.promotion, Knight | Bishop | Rook) {
        return MoveScore::UnderPromotion(SEE_PIECE_VALUES[mv.promotion] as i16);
    } else if mv.promotion == Queen {
        // promotions here may not be actually be captures
        return MoveScore::WinningCapture(MVV_LVA[Queen][Pawn]);
    }

    let mvv_lva = MVV_LVA[board.piece_on(mv.to).unwrap_or(Pawn)][mv.piece];

    if mvv_lva >= 0 {
        MoveScore::WinningCapture(mvv_lva)
    } else {
        MoveScore::LosingCapture(mvv_lva)
    }
}

fn score_quiet(
    board: &Board,
    killers: &[Move; NUM_KILLER_MOVES],
    counters: &[[[Move; 64]; 6]; 2],
    history: &[[[i16; 64]; 6]; 2],
    last_move: Move,
    mv: Move,
) -> MoveScore {
    let current_player = board.current_player();
    if killers.contains(&mv) {
        MoveScore::KillerMove
    } else if counters[current_player][last_move.piece][last_move.to] == mv {
        MoveScore::CounterMove
    } else {
        MoveScore::Quiet(history[current_player][mv.piece][mv.to])
    }
}
