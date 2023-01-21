use std::ops::Index;
use std::sync::{atomic::Ordering::*, Arc, RwLock};
use std::time::Instant;
use std::{fmt::Display, sync::atomic::*};

use cheers_pregen::LMR;
use eval_params::{EvalParams, CHECKMATE_SCORE, DRAW_SCORE, EVAL_PARAMS};

use crate::board::see::MVV_LVA;
use crate::hash_tables::{score_from_tt, score_into_tt};
use crate::moves::MoveScore;
use crate::{
    board::{
        eval_types::{GamePhase::*, TraceTarget},
        see::SEE_PIECE_VALUES,
        *,
    },
    hash_tables::{NodeType::*, PawnHashTable, TranspositionTable},
    moves::{KillerMoves, Move, MoveList},
    types::Piece::*,
};

pub static ABORT_SEARCH: AtomicBool = AtomicBool::new(false);
pub static NODE_COUNT: AtomicUsize = AtomicUsize::new(0);

const INF: i32 = i32::MAX;
const MINUS_INF: i32 = -INF;

pub const SEARCH_MAX_PLY: usize = 128;

pub const PV_MAX_LEN: usize = 16;
#[derive(Copy, Clone, Default, Debug)]
pub struct PrincipalVariation {
    len: usize,
    moves: [Move; PV_MAX_LEN],
}

impl PrincipalVariation {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn update_from(&mut self, next: Move, other: &Self) {
        self.moves[0] = next;
        self.moves[1..(other.len + 1).min(PV_MAX_LEN)]
            .copy_from_slice(&other.moves[..(other.len.min(PV_MAX_LEN - 1))]);
        self.len = (other.len + 1).min(PV_MAX_LEN);
    }
    pub fn clear(&mut self) {
        self.len = 0;
    }
}
impl Display for PrincipalVariation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, m) in self.moves.iter().take(self.len).enumerate() {
            if i == 0 {
                write!(f, "{}", m.coords())?;
            } else {
                write!(f, " {}", m.coords())?;
            }
        }
        Ok(())
    }
}
impl Index<usize> for PrincipalVariation {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.moves[index]
    }
}

#[derive(Clone, Copy)]
pub struct EngineOptions {
    pub tt_size_mb: usize,
    pub nmp_depth: i32,
    pub nmp_reduction: i32,
    pub see_pruning_depth: i32,
    pub see_capture_margin: i32,
    pub see_quiet_margin: i32,
    pub pvs_fulldepth: i32,
    pub delta_pruning_margin: i32,
    pub fp_margin_1: i32,
    pub fp_margin_2: i32,
    pub fp_margin_3: i32,
    pub rfp_margin: i32,
    pub lmp_depth: i32,
    pub lmp_margin: i32,
    pub iir_depth: i32,
}

pub const NMP_DEPTH: i32 = 2;
pub const NMP_REDUCTION: i32 = 5;
pub const SEE_PRUNING_DEPTH: i32 = 6;
pub const SEE_CAPTURE_MARGIN: i32 = 59;
pub const SEE_QUIET_MARGIN: i32 = 39;
pub const PVS_FULLDEPTH: i32 = 1;
pub const DELTA_PRUNING_MARGIN: i32 = 91;
pub const FP_MARGIN_1: i32 = 115;
pub const FP_MARGIN_2: i32 = 344;
pub const FP_MARGIN_3: i32 = 723;
pub const RFP_MARGIN: i32 = 106;
pub const LMP_DEPTH: i32 = 1;
pub const LMP_MARGIN: i32 = 2;
pub const IIR_DEPTH: i32 = 4;

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            tt_size_mb: 8,
            nmp_depth: NMP_DEPTH,
            nmp_reduction: NMP_REDUCTION,
            see_pruning_depth: SEE_PRUNING_DEPTH,
            see_capture_margin: SEE_CAPTURE_MARGIN,
            see_quiet_margin: SEE_QUIET_MARGIN,
            pvs_fulldepth: PVS_FULLDEPTH,
            delta_pruning_margin: DELTA_PRUNING_MARGIN,
            fp_margin_1: FP_MARGIN_1,
            fp_margin_2: FP_MARGIN_2,
            fp_margin_3: FP_MARGIN_3,
            rfp_margin: RFP_MARGIN,
            lmp_depth: LMP_DEPTH,
            lmp_margin: LMP_MARGIN,
            iir_depth: IIR_DEPTH,
        }
    }
}

#[derive(Clone)]
pub struct Search {
    pub game: Board,
    pub search_history: Vec<u64>,
    pub pre_history: Vec<u64>,
    pub move_lists: Vec<MoveList>,
    pub seldepth: usize,
    transposition_table: Arc<RwLock<TranspositionTable>>,
    pawn_hash_table: PawnHashTable,
    killer_moves: KillerMoves<2>,
    history_tables: [[[i16; 64]; 6]; 2],
    countermove_tables: [[[Move; 64]; 6]; 2],
    pub max_depth: Option<usize>,
    pub max_nodes: Option<usize>,
    pub max_time_ms: Option<(usize, usize)>,
    pub abort_time_ms: Option<usize>,
    start_time: Instant,
    output: bool,
    options: EngineOptions,
}

impl Search {
    pub fn new(game: Board) -> Self {
        Self {
            game,
            search_history: Vec::new(),
            pre_history: Vec::new(),
            move_lists: vec![MoveList::new(); 128],
            seldepth: 0,
            transposition_table: Arc::new(RwLock::new(TranspositionTable::new(0))),
            pawn_hash_table: PawnHashTable::new(0),
            killer_moves: KillerMoves::new(),
            history_tables: [[[0; 64]; 6]; 2],
            countermove_tables: [[[Move::null(); 64]; 6]; 2],
            max_depth: None,
            max_nodes: None,
            max_time_ms: None,
            abort_time_ms: None,
            start_time: Instant::now(),
            output: false,
            options: EngineOptions::default(),
        }
    }

    pub fn new_with_tt(game: Board, tt: Arc<RwLock<TranspositionTable>>) -> Self {
        Self {
            game,
            search_history: Vec::new(),
            pre_history: Vec::new(),
            move_lists: vec![MoveList::new(); 128],
            seldepth: 0,
            transposition_table: tt,
            pawn_hash_table: PawnHashTable::new(0),
            killer_moves: KillerMoves::new(),
            history_tables: [[[0; 64]; 6]; 2],
            countermove_tables: [[[Move::null(); 64]; 6]; 2],
            max_depth: None,
            max_nodes: None,
            max_time_ms: None,
            abort_time_ms: None,
            start_time: Instant::now(),
            output: false,
            options: EngineOptions::default(),
        }
    }

    pub fn tt_size_mb(mut self, tt_size_mb: usize) -> Self {
        self.transposition_table
            .write()
            .unwrap()
            .set_size(tt_size_mb);
        self.pawn_hash_table = PawnHashTable::new(tt_size_mb / 8);
        self
    }

    pub fn pre_history(mut self, pre_history: Vec<u64>) -> Self {
        self.pre_history = pre_history;
        self
    }

    pub fn max_depth(mut self, depth: Option<usize>) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn max_nodes(mut self, nodes: Option<usize>) -> Self {
        self.max_nodes = nodes;
        self
    }

    pub fn output(mut self, output: bool) -> Self {
        self.output = output;
        self
    }

    pub fn options(mut self, options: EngineOptions) -> Self {
        self.options = options;
        self
    }

    pub fn search(&self) -> (i32, PrincipalVariation) {
        let mut last_score = i32::MIN;
        let mut last_pv = PrincipalVariation::new();

        let mut search = self.clone();
        let tt = &*self.transposition_table.read().unwrap();

        let start = Instant::now();

        // Iterative Deepening: search with increasing depth, exploiting the results
        // of shallower searches to speed up deeper ones
        'id_loop: for i in 1.. {
            // Aspiration Window: search a narrow window around the score in hope of saving
            // some search time
            let mut window_size = 50;
            // let mut window = if i == 1 {
            //     (MINUS_INF, INF)
            // } else {
            //     (last_score - window_size, last_score + window_size)
            // };
            let window = (MINUS_INF, INF);

            let mut pv = PrincipalVariation::new();

            // repeat failed searches with wider windows until a search succeeds
            let score = loop {
                search.seldepth = 0;

                let score = search.negamax(
                    &self.game.clone(),
                    window.0,
                    window.1,
                    i as i32,
                    0,
                    Move::null(),
                    &mut pv,
                    tt,
                );

                if ABORT_SEARCH.load(Ordering::Relaxed) && i > 1 {
                    // can't trust results from a partial search
                    break 'id_loop;
                }

                // Expand the search window based on which bound the search failed on
                // match (score > window.0, score < window.1) {
                //     // fail high, expand upper window
                //     (true, false) => {
                //         window = (window.0, window.1 + window_size);
                //         window_size *= 2;
                //     }
                //     // fail low, expand lower window
                //     (false, true) => {
                //         window = (window.0 - window_size, window.1);
                //         window_size *= 2;
                //     }
                //     // exact score within the window, search success
                //     (true, true) => break score,
                //     _ => unreachable!(),
                // }
                break score;
            };

            let end = Instant::now();
            let mate_distance = CHECKMATE_SCORE - score.abs();
            let score_string = if mate_distance < 100 {
                format!("mate {}", score.signum() * ((mate_distance + 1) / 2))
            } else {
                format!("cp {score}")
            };
            let hash_fill = tt.sample_fill();
            let nodes = NODE_COUNT.load(Ordering::Relaxed);
            // we can trust the results from the previous search
            if self.output {
                println!(
                    "info depth {i} seldepth {} score {score_string} nodes {} nps {} hashfull {} time {} pv {pv}",
                    search.seldepth,
                    nodes,
                    ((nodes) as f32 / (end - start).as_secs_f32()) as usize,
                    hash_fill,
                    (end - start).as_millis(),
                )
            };

            last_pv = pv;
            last_score = score;
            let time = Instant::now();
            // terminate search if we are hinted to do so
            if let Some((stop_hint, _)) = self.max_time_ms {
                if (time - start).as_millis() as usize >= stop_hint {
                    break;
                }
            }

            // terminate search at max depth or with forced mate/draw
            if let Some(max_depth) = self.max_depth {
                if i >= max_depth {
                    ABORT_SEARCH.store(false, Ordering::Relaxed);
                    break;
                }
            }
            if i >= SEARCH_MAX_PLY {
                ABORT_SEARCH.store(false, Ordering::Relaxed);
                break;
            }
        }
        (last_score, last_pv)
    }

    fn negamax(
        &mut self,
        board: &Board,
        mut alpha: i32,
        mut beta: i32,
        mut depth: i32,
        ply: usize,
        last_move: Move,
        pv: &mut PrincipalVariation,
        tt: &TranspositionTable,
    ) -> i32 {
        // check time and max nodes every 2048 nodes
        let nodes = NODE_COUNT.load(Relaxed);
        if nodes & 2047 == 2047 {
            if let Some((_, abort_time)) = self.max_time_ms {
                // signal an abort if time has exceeded alloted time
                if Instant::now().duration_since(self.start_time).as_millis() as usize > abort_time
                {
                    ABORT_SEARCH.store(true, Relaxed);
                    return 0;
                }
            }
            if let Some(nodes) = self.max_nodes {
                if NODE_COUNT.load(Relaxed) >= nodes {
                    ABORT_SEARCH.store(true, Relaxed);
                    return 0;
                }
            }
        }

        // abort the search, making sure we search to at least depth 1
        if (ABORT_SEARCH.load(Relaxed) && ply > 1) || ply >= SEARCH_MAX_PLY {
            // there are no moves beyond this one, so clear the parent PV
            pv.clear();
            return 0;
        }

        // drop into quiescence search at depth 0
        if depth == 0 {
            pv.clear();
            return self.quiesce(board, alpha, beta, ply, last_move, &EVAL_PARAMS, tt);
        }

        // increment the node counters
        NODE_COUNT.fetch_add(1, Relaxed);

        // increase the seldepth if this node is deeper
        self.seldepth = self.seldepth.max(ply);

        let in_check = board.in_check();
        let root = ply == 0;
        let pv_node = alpha != beta - 1;

        // check 50 move and repetition draws when not at the root
        if !root
            && (board.halfmove_clock() >= 100
                || self
                    .pre_history
                    .iter()
                    .rev()
                    .take(board.halfmove_clock() as usize)
                    .filter(|&&h| h == board.hash())
                    .count()
                    >= 2
                || self
                    .search_history
                    .iter()
                    .rev()
                    .take(board.halfmove_clock() as usize)
                    .any(|h| *h == board.hash()))
        {
            pv.clear();
            return DRAW_SCORE;
        }

        let mut tt_move = Move::null();
        let mut tt_score = MINUS_INF;
        if let Some(entry) = tt.get(board.hash()) {
            // TT pruning when the bounds are correct, but not at in the PV
            if !pv_node
                && entry.depth >= depth as i8
                && (entry.node_type == Exact
                    || (entry.node_type == LowerBound && entry.score >= beta)
                    || (entry.node_type == UpperBound && entry.score <= alpha))
            {
                pv.clear();
                return score_from_tt(entry.score, ply);
            }

            // otherwise use the score as an improved static eval
            // and the move for move ordering
            if matches!(entry.node_type, LowerBound | Exact) {
                tt_score = score_from_tt(entry.score, ply);
            }
            tt_move = Move {
                piece: entry.piece,
                from: entry.move_from,
                to: entry.move_to,
                promotion: entry.promotion,
            };
        }

        // the PV from this node will be gathered into this array
        let mut line = PrincipalVariation::new();

        // generate legal moves into the list for this depth
        board.generate_legal_moves_into(&mut self.move_lists[ply]);

        // check for checkmate and stalemate
        if self.move_lists[ply].len() == 0 {
            if in_check {
                // checkmate, preferring shorter mating sequences
                pv.clear();
                return -(CHECKMATE_SCORE - (ply as i32));
            } else {
                // stalemate
                pv.clear();
                return DRAW_SCORE;
            }
        }

        // move ordering: try heuristically good moves first to reduce the AB search tree
        for smv in self.move_lists[ply].inner_mut().iter_mut() {
            // search the TT move first
            if smv.mv == tt_move {
                smv.score = MoveScore::TTMove
            // sort the winning/losing captures and capture promotions
            } else if board.is_capture(smv.mv) {
                if smv.mv.promotion == Queen {
                    smv.score = MoveScore::WinningCapture(SEE_PIECE_VALUES[Queen] as i16)
                } else if smv.mv.promotion != Pawn {
                    smv.score = MoveScore::UnderPromotion(SEE_PIECE_VALUES[smv.mv.promotion] as i16)
                } else {
                    let mvv_lva = MVV_LVA[board.piece_on(smv.mv.to).unwrap_or(Pawn)][smv.mv.piece];
                    if mvv_lva > 0 {
                        smv.score = MoveScore::WinningCapture(mvv_lva)
                    } else {
                        smv.score = MoveScore::LosingCapture(mvv_lva)
                    }
                }
            // sort quiet promotions
            } else if smv.mv.promotion != Pawn {
                if smv.mv.promotion == Queen {
                    smv.score = MoveScore::WinningCapture(SEE_PIECE_VALUES[Queen] as i16)
                } else {
                    smv.score = MoveScore::UnderPromotion(SEE_PIECE_VALUES[smv.mv.promotion] as i16)
                }
            // sort quiet moves
            } else {
                // killer moves
                if self.killer_moves[ply].contains(&smv.mv) {
                    smv.score = MoveScore::KillerMove
                //  countermove
                } else if smv.mv
                    == self.countermove_tables[board.current_player()][last_move.piece]
                        [last_move.to]
                {
                    smv.score = MoveScore::CounterMove
                // Other quiets get sorted by history heuristic
                } else {
                    smv.score = MoveScore::Quiet(
                        self.history_tables[board.current_player()][smv.mv.piece][smv.mv.to],
                    )
                }
            }
        }

        // make sure the reported best move is at least legal
        let mut best_move = self.move_lists[ply][0];

        // save the old alpha to see if any moves improve the PV
        let old_alpha = alpha;

        // push this position to the history
        self.search_history.push(board.hash());

        for i in 0..self.move_lists[ply].len() {
            // pick the move with the next highest sorting score
            let (mv, score) = self.move_lists[ply].pick_move(i);

            let capture = board.is_capture(mv);

            // make the move on a copy of the board
            let mut new = board.clone();
            new.make_move(mv);

            let mut score = MINUS_INF;
            // perform a search on the new position, returning the score and the PV
            let full_width = i == 0 || {
                // null window search on later moves
                score = -self.negamax(
                    &new,
                    -alpha - 1,
                    -alpha,
                    depth - 1,
                    ply + 1,
                    mv,
                    &mut line,
                    tt,
                );

                score > alpha && score < beta
            };

            // full window search on the first move and later moves that improved alpha
            if full_width {
                score = -self.negamax(&new, -beta, -alpha, depth - 1, ply + 1, mv, &mut line, tt);
            }

            // scores can't be trusted after an abort, don't let them get into the TT
            if ABORT_SEARCH.load(Relaxed) {
                // remove this position from the history
                self.search_history.pop();
                return 0;
            }

            if score >= beta {
                // beta cutoff, this move is too good and so the opponent won't go into this position
                pv.clear();

                // add the score and move to TT
                tt.set(board.hash(), mv, depth as i8, score_into_tt(score, ply), LowerBound, pv_node);

                // update killer, countermove and history tables for good quiets
                if !capture {
                    self.killer_moves.push(mv, ply);
                    self.countermove_tables[board.current_player()][last_move.piece]
                        [last_move.to] = mv;
                    self.history_tables[board.current_player()][mv.piece][mv.to] +=
                        (depth * depth) as i16;
                    // scale history scores down if they get too high
                    if self.history_tables[board.current_player()][mv.piece][mv.to] > 4096 {
                        self.history_tables[board.current_player()]
                            .iter_mut()
                            .flatten()
                            .for_each(|x| *x /= 64);
                    }

                    // punish quiets that were played but didn't cause a beta cutoff
                    for smv in self.move_lists[ply].inner()[..(i.max(1) - 1)]
                        .iter()
                        .filter(|smv| !board.is_capture(smv.mv))
                    {
                        let mv = smv.mv;
                        self.history_tables[board.current_player()][mv.piece][mv.to] -=
                            (depth * depth) as i16;
                        if self.history_tables[board.current_player()][mv.piece][mv.to] < -4096 {
                            self.history_tables[board.current_player()]
                                .iter_mut()
                                .flatten()
                                .for_each(|x| *x /= 64)
                        }
                    }
                }

                // remove this position from the history
                self.search_history.pop();

                return score;
            } else if score > alpha {
                // a score between alpha and beta represents a new best move
                best_move = mv;

                // update the parent PV with the new PV
                pv.update_from(mv, &line);

                // raise alpha so worse moves after this one will be pruned early
                alpha = score;
            }
        }

        // remove this position from the history
        self.search_history.pop();

        // after all moves have been searched, alpha is either unchanged
        // (this position is bad) or raised (new pv from this node)
        // add the score and the new best move to the TT
        tt.set(
            board.hash(),
            best_move,
            depth as i8,
            score_into_tt(alpha, ply),
            if alpha > old_alpha { Exact } else { UpperBound },
            pv_node,
        );

        alpha
    }

    pub fn quiesce(
        &mut self,
        board: &Board,
        alpha: i32,
        beta: i32,
        ply: usize,
        last_move: Move,
        eval_params: &EvalParams,
        tt: &TranspositionTable,
    ) -> i32 {
        self.quiesce_impl::<()>(board, alpha, beta, ply, last_move, eval_params, tt)
            .0
    }

    pub fn quiesce_impl<T: TraceTarget + Default>(
        &mut self,
        board: &Board,
        mut alpha: i32,
        beta: i32,
        ply: usize,
        _last_move: Move,
        eval_params: &EvalParams,
        tt: &TranspositionTable,
    ) -> (i32, T) {
        // check time and max nodes every 2048 nodes
        let nodes = NODE_COUNT.load(Relaxed);
        if nodes & 2047 == 2047 {
            if let Some((_, abort_time)) = self.max_time_ms {
                // signal an abort if time has exceeded alloted time
                if Instant::now().duration_since(self.start_time).as_millis() as usize > abort_time
                {
                    ABORT_SEARCH.store(true, Relaxed);
                    return (0, T::default());
                }
            }
            if let Some(nodes) = self.max_nodes {
                if NODE_COUNT.load(Relaxed) >= nodes {
                    ABORT_SEARCH.store(true, Relaxed);
                    return (0, T::default());
                }
            }
        }

        // check for abort
        if ABORT_SEARCH.load(Relaxed) || ply >= SEARCH_MAX_PLY {
            return (0, T::default());
        }

        // increment node counter
        NODE_COUNT.fetch_add(1, Relaxed);

        // increase the seldepth if this node is deeper
        self.seldepth = self.seldepth.max(ply);

        // check 50 move and repetition draws
        if board.halfmove_clock() >= 100
            || self
                .pre_history
                .iter()
                .rev()
                .take(board.halfmove_clock() as usize)
                .filter(|&&h| h == board.hash())
                .count()
                >= 2
            || self
                .search_history
                .iter()
                .rev()
                .take(board.halfmove_clock() as usize)
                .any(|&h| h == board.hash())
        {
            return (DRAW_SCORE, T::default());
        }

        // Transposition Table lookup when tracing is disabled
        let mut tt_move = Move::null();
        let mut tt_score = MINUS_INF;
        if !T::TRACING {
            if let Some(entry) = tt.get(board.hash()) {
                // TT pruning when the bounds are correct
                if entry.node_type == Exact
                    || (entry.node_type == LowerBound && entry.score >= beta)
                    || (entry.node_type == UpperBound && entry.score <= alpha)
                {
                    return (score_from_tt(entry.score, ply), T::default());
                }

                // otherwise use the score as an improved static eval
                // and the move for move ordering
                if matches!(entry.node_type, LowerBound | Exact) {
                    tt_score = score_from_tt(entry.score, ply);
                }
                tt_move = Move {
                    piece: entry.piece,
                    from: entry.move_from,
                    to: entry.move_to,
                    promotion: entry.promotion,
                };
            }
        }

        // the static evaluation allows us to prune moves that are worse than 'standing pat' at this node
        let (static_eval, mut best_trace) = if !T::TRACING && tt_score != MINUS_INF {
            (tt_score, T::default())
        } else {
            board.evaluate_impl::<T>(&mut self.pawn_hash_table)
        };

        // if the static eval is above beta, then the opponent won't play into this position
        if static_eval >= beta {
            return (beta, best_trace);
        }

        // if the static eval is better than alpha, use it to prune moves instead
        alpha = alpha.max(static_eval);

        // quiescence search only looks at captures to ensure fast completion
        board.generate_legal_captures_into(&mut self.move_lists[ply]);

        // move ordering: try heuristically good moves first to reduce the AB search tree
        for smv in self.move_lists[ply].inner_mut().iter_mut() {
            // search the TT move first
            if smv.mv == tt_move {
                smv.score = MoveScore::TTMove
            // sort winning/losing captures
            } else {
                // underpromotions
                if matches!(smv.mv.promotion, Knight | Bishop | Rook) {
                    smv.score =
                        MoveScore::UnderPromotion(SEE_PIECE_VALUES[smv.mv.promotion] as i16);
                } else {
                    let mvv_lva = MVV_LVA[board.piece_on(smv.mv.to).unwrap_or(Pawn)][smv.mv.piece];
                    let promotion = SEE_PIECE_VALUES[smv.mv.promotion] as i16 - 100;
                    if mvv_lva + promotion > 0 {
                        smv.score = MoveScore::WinningCapture(mvv_lva + promotion);
                    } else {
                        smv.score = MoveScore::LosingCapture(mvv_lva + promotion);
                    }
                }
            }
        }

        let old_alpha = alpha;

        // make sure the best move is at least legal
        let mut best_move = self.move_lists[ply][0];

        for i in 0..self.move_lists[ply].len() {
            // pick the move with the next highest sorting score
            let (mv, score) = self.move_lists[ply].pick_move(i);

            // make the move on a copy of the board
            self.search_history.push(board.hash());
            let mut new = board.clone();
            new.make_move(mv);

            let (mut score, trace) =
                self.quiesce_impl::<T>(&new, -beta, -alpha, ply + 1, mv, &EVAL_PARAMS, tt);
            score = -score;

            // 'unmake' the move by removing it from the position history
            self.search_history.pop();

            if score >= beta {
                // beta cutoff, this move is too good and so the opponent won't go into this position

                // add the score to the TT
                tt.set(board.hash(), mv, -1, score_into_tt(score, ply), LowerBound, false);
                return (score, trace);
            } else if score > alpha {
                // a score between alpha and beta represents a new best move
                best_move = mv;
                best_trace = trace;

                // raise alpha so worse moves after this one will be pruned early
                alpha = score;
            }
        }

        // if there are no legal captures, check for checkmate/stalemate
        // disable when tracing to avoid empty traces
        if !T::TRACING && self.move_lists[ply].len() == 0 {
            let mut some_moves = false;
            board.generate_legal_moves(|mvs| some_moves = some_moves || mvs.moves.is_not_empty());

            if !some_moves {
                if board.in_check() {
                    return (-(CHECKMATE_SCORE - (ply as i32)), T::default());
                } else {
                    return (DRAW_SCORE, T::default());
                }
            }
        }

        // after all moves are searched alpha is either unchanged (this position is bad) or raised (new pv)
        // add the score to the TT
        tt.set(
            board.hash(),
            best_move,
            -1,
            score_into_tt(alpha, ply),
            if alpha > old_alpha { Exact } else { UpperBound },
            false,
        );

        (alpha, best_trace)
    }
}
