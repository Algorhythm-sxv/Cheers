use std::sync::atomic::*;
use std::sync::{atomic::Ordering::*, Arc, RwLock};
use std::thread;
use std::time::Instant;

use cheers_pregen::{LMP_MARGINS, LMR};
use eval_params::{CHECKMATE_SCORE, DRAW_SCORE};

use crate::board::see::SEE_PIECE_VALUES;
use crate::moves::*;
use crate::thread_data::ThreadData;
use crate::types::{HelperThread, MainThread, TypeMainThread};
use crate::{
    board::{eval_types::TraceTarget, *},
    hash_tables::{score_from_tt, score_into_tt, NodeType::*, PawnHashTable, TranspositionTable},
    move_sorting::MoveSorter,
    options::SearchOptions,
    types::{All, Captures, NotRoot, Piece::*, Root, TypeRoot},
};

pub static ABORT_SEARCH: AtomicBool = AtomicBool::new(false);
pub static NODE_COUNT: AtomicUsize = AtomicUsize::new(0);

pub const INF: i16 = i16::MAX;
pub const MINUS_INF: i16 = -INF;

pub const SEARCH_MAX_PLY: usize = 128;

pub const MAX_HISTORY: i16 = 4096;

#[derive(Clone)]
pub struct Search {
    pub game: Board,
    pub search_history: Vec<u64>,
    pub pre_history: Vec<u64>,
    pub seldepth: usize,
    transposition_table: Arc<RwLock<TranspositionTable>>,
    pawn_hash_table: PawnHashTable,
    pub thread_data: ThreadData,
    pub max_depth: Option<usize>,
    pub max_nodes: Option<usize>,
    pub max_time_ms: Option<(usize, usize)>,
    pub abort_time_ms: Option<usize>,
    start_time: Instant,
    output: bool,
    chess_960: bool,
    options: SearchOptions,
    pub local_nodes: usize,
}

impl Search {
    pub fn new(game: Board) -> Self {
        Self {
            game,
            search_history: Vec::new(),
            pre_history: Vec::new(),
            seldepth: 0,
            transposition_table: Arc::new(RwLock::new(TranspositionTable::new(0))),
            pawn_hash_table: PawnHashTable::new(0),
            thread_data: ThreadData::new(),
            max_depth: None,
            max_nodes: None,
            max_time_ms: None,
            abort_time_ms: None,
            start_time: Instant::now(),
            output: false,
            chess_960: false,
            options: SearchOptions::default(),
            local_nodes: 0,
        }
    }

    pub fn new_with_tt(game: Board, tt: Arc<RwLock<TranspositionTable>>, pawn_hash: usize) -> Self {
        Self {
            game,
            search_history: Vec::new(),
            pre_history: Vec::new(),
            seldepth: 0,
            transposition_table: tt,
            pawn_hash_table: PawnHashTable::new(pawn_hash),
            thread_data: ThreadData::new(),
            max_depth: None,
            max_nodes: None,
            max_time_ms: None,
            abort_time_ms: None,
            start_time: Instant::now(),
            output: false,
            chess_960: false,
            options: SearchOptions::default(),
            local_nodes: 0,
        }
    }

    pub fn tt_size_mb(mut self, tt_size_mb: usize, pawn_hash_size_mb: usize) -> Self {
        self.transposition_table
            .write()
            .unwrap()
            .set_size(tt_size_mb);
        self.pawn_hash_table = PawnHashTable::new(pawn_hash_size_mb);
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

    pub fn chess_960(mut self, chess_960: bool) -> Self {
        self.chess_960 = chess_960;
        self
    }

    pub fn options(mut self, options: SearchOptions) -> Self {
        self.options = options;
        self
    }

    pub fn smp_search(&self) -> (i16, PrincipalVariation) {
        ABORT_SEARCH.store(false, Relaxed);
        NODE_COUNT.store(0, Ordering::Relaxed);
        let mut score = MINUS_INF;
        let mut pv = PrincipalVariation::new();

        // Lazy SMP: start all threads at the same depth, communicating only
        // via the shared TT
        thread::scope(|s| {
            // main thread: this is the only thread that reports back over UCI
            let _main_thread = s.spawn(|| {
                let search = self.clone();
                (score, pv) = search.search::<MainThread>(true);
                ABORT_SEARCH.store(true, Relaxed);
            });

            // helper threads: these only have their results added to the TT
            for _ in 1..self.options.threads {
                s.spawn(|| {
                    let search = self.clone();
                    let _ = search.search::<HelperThread>(true);
                });
            }
        });

        (score, pv)
    }

    pub fn search<M: TypeMainThread>(&self, set_global_abort: bool) -> (i16, PrincipalVariation) {
        ABORT_SEARCH.store(false, Relaxed);
        if M::MAIN_THREAD {
            NODE_COUNT.store(0, Ordering::Relaxed);
        }
        let mut last_score = i16::MIN;
        let mut last_pv = PrincipalVariation::new();

        let mut search = self.clone();
        let tt = &*self.transposition_table.read().unwrap();

        let start = Instant::now();

        // Iterative Deepening: search with increasing depth, exploiting the results
        // of shallower searches to speed up deeper ones
        'id_loop: for i in 1..SEARCH_MAX_PLY {
            // Aspiration Window: search a narrow window around the score in hope of saving
            // some search time
            let mut window_size = 50;
            let mut window = if i == 1 {
                (MINUS_INF, INF)
            } else {
                // saturate to prevent overflows
                (
                    last_score.saturating_sub(window_size).max(-INF),
                    last_score.saturating_add(window_size).min(INF),
                )
            };

            let mut pv = PrincipalVariation::new().chess_960(self.chess_960);

            let this_depth_start = Instant::now();
            // repeat failed searches with wider windows until a search succeeds
            let score = loop {
                search.seldepth = 0;

                let score = search.negamax::<Root, M>(
                    &self.game.clone(),
                    window.0,
                    window.1,
                    i as i8,
                    0,
                    &mut pv,
                    tt,
                    true,
                );

                // add helper thread nodes to global count
                if !M::MAIN_THREAD {
                    NODE_COUNT.fetch_add(search.local_nodes, Relaxed);
                    search.local_nodes = 0;
                }

                if ABORT_SEARCH.load(Relaxed) && i > 1 {
                    // can't trust results from a partial search, but report accurate statistics
                    let end = Instant::now();
                    let mate_distance = CHECKMATE_SCORE - last_score.abs();
                    let score_string = if mate_distance < SEARCH_MAX_PLY as i16 {
                        format!("mate {}", last_score.signum() * ((mate_distance + 1) / 2))
                    } else {
                        format!("cp {last_score}")
                    };
                    let hash_fill = tt.sample_fill();
                    let nodes = if set_global_abort {
                        NODE_COUNT.load(Relaxed)
                    } else {
                        search.local_nodes
                    };
                    if M::MAIN_THREAD && self.output {
                        println!(
                            "info depth {} seldepth {} score {score_string} nodes {} nps {} hashfull {} time {} pv {last_pv}",
                            i-1,
                            search.seldepth,
                            nodes,
                            ((nodes) as f32 / (end - start).as_secs_f32()) as usize,
                            hash_fill,
                            (end - start).as_millis(),
                        )
                    }
                    break 'id_loop;
                }

                // Expand the search window based on which bound the search failed on
                match (score > window.0, score < window.1) {
                    // fail high, expand upper window
                    (true, false) => {
                        window = (window.0, window.1.saturating_add(window_size).min(INF));
                        window_size = window_size.saturating_mul(2);
                    }
                    // fail low, expand lower window
                    (false, true) => {
                        window = (window.0.saturating_sub(window_size).max(-INF), window.1);
                        window_size = window_size.saturating_mul(2);
                    }
                    // exact score within the window, search success
                    (true, true) => break score,
                    _ => {
                        panic!(
                            "Window error: [{}, {}], {}, {}",
                            window.0, window.1, window_size, score
                        );
                    }
                }
            };

            let end = Instant::now();
            let mate_distance = CHECKMATE_SCORE - score.abs();
            let score_string = if mate_distance < SEARCH_MAX_PLY as i16 {
                format!("mate {}", score.signum() * ((mate_distance + 1) / 2))
            } else {
                format!("cp {score}")
            };
            let hash_fill = tt.sample_fill();
            let nodes = if set_global_abort {
                NODE_COUNT.load(Relaxed)
            } else {
                search.local_nodes
            };
            // we can trust the results from the previous search
            if M::MAIN_THREAD && self.output {
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
            // terminate search if we are hinted to do so or the next depth would likely take too long
            if let Some((stop_hint, abort_time)) = self.max_time_ms {
                if (end - start).as_millis() as usize >= stop_hint {
                    break;
                }

                // only stop early if this is not a fixed-time search
                if stop_hint != abort_time {
                    let this_depth_time = end - this_depth_start;
                    let estimated_next_time = end + 2 * this_depth_time;
                    if (estimated_next_time - start).as_millis() as usize >= abort_time {
                        break;
                    }
                }
            }

            // terminate search at max nodes
            if let Some(max_nodes) = self.max_nodes {
                if nodes >= max_nodes {
                    if set_global_abort {
                        ABORT_SEARCH.store(true, Relaxed);
                    }
                    break;
                }
            }
            // terminate search at max depth or with forced mate/draw
            if let Some(max_depth) = self.max_depth {
                if M::MAIN_THREAD && i >= max_depth {
                    if set_global_abort {
                        ABORT_SEARCH.store(true, Relaxed);
                    }
                    break;
                }
            }
            if i >= SEARCH_MAX_PLY {
                if set_global_abort {
                    ABORT_SEARCH.store(true, Relaxed);
                }
                break;
            }
        }
        (last_score, last_pv)
    }

    #[allow(clippy::too_many_arguments)]
    fn negamax<R: TypeRoot, M: TypeMainThread>(
        &mut self,
        board: &Board,
        mut alpha: i16,
        mut beta: i16,
        mut depth: i8,
        ply: usize,
        pv: &mut PrincipalVariation,
        tt: &TranspositionTable,
        allow_nmp: bool,
    ) -> i16 {
        // check time and max nodes every 2048 nodes in the main thread
        let nodes = self.local_nodes;
        if M::MAIN_THREAD && nodes & 2047 == 2047 {
            if let Some((_, abort_time)) = self.max_time_ms {
                // signal an abort if time has exceeded alloted time
                if Instant::now().duration_since(self.start_time).as_millis() as usize > abort_time
                {
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

        // prefetch the transposition table: hint to the CPU that we want this in the cache well
        // before we actually use it
        tt.prefetch(board.hash());

        // Check extensions: increase depth by 1 when in check to avoid tactical blindness
        let in_check = board.in_check();
        if in_check {
            // saturating add to avoid negative depths on overflow
            depth = depth.saturating_add(1);
        }

        // the PV from this node will be gathered into this array
        let mut line = PrincipalVariation::new();

        // drop into quiescence search at depth 0
        if depth == 0 {
            let score = self.quiesce::<M>(board, alpha, beta, ply, &mut line, tt);
            *pv = line;
            return score;
        }

        // increment the node counters
        if M::MAIN_THREAD {
            NODE_COUNT.fetch_add(1, Relaxed);
        }
        self.local_nodes += 1;

        // increase the seldepth if this node is deeper
        self.seldepth = self.seldepth.max(ply);

        let pv_node = alpha != beta - 1;
        let current_player = board.current_player();

        // check 50 move and repetition draws when not at the root
        if !R::ROOT
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
            // randomise around the draw score slightly to improve searching of draws
            return DRAW_SCORE + 4 - (nodes & 7) as i16;
        }

        // Mate distance pruning: we can never find a score better than mate at the current ply
        // or worse than being mated at the current ply
        if !R::ROOT {
            alpha = alpha.max(-CHECKMATE_SCORE + ply as i16);
            beta = beta.min(CHECKMATE_SCORE - ply as i16);

            if alpha >= beta {
                return beta;
            }
        }

        let mut tt_move = Move::null();
        let mut tt_score = MINUS_INF;
        let mut tt_depth = 0;
        let mut tt_bound = UpperBound;
        if let Some(entry) = tt.get(board.hash()) {
            // TT pruning when the bounds are correct, but not at in the PV
            if !pv_node
                && entry.depth >= depth
                && (entry.node_type == Exact
                    || (entry.node_type == LowerBound && entry.score >= beta)
                    || (entry.node_type == UpperBound && entry.score <= alpha))
            {
                pv.clear();
                return score_from_tt(entry.score, ply);
            }

            // otherwise use the score as an improved static eval
            // and the move for move ordering
            tt_score = score_from_tt(entry.score, ply);
            tt_bound = entry.node_type;

            tt_move = Move::new(entry.piece, entry.move_from, entry.move_to, entry.promotion);
            tt_depth = entry.depth;
        }

        // IIR: reduce the search depth if no TT move is present
        if !R::ROOT && !pv_node && depth >= self.options.iir_depth && tt_move.is_null() {
            depth -= 1;
        }

        let eval = if matches!(tt_bound, LowerBound | Exact) {
            tt_score
        } else if !in_check {
            board.evaluate(&mut self.pawn_hash_table)
        } else {
            // static eval isn't valid when in check
            MINUS_INF
        };

        // store the current 'static' eval to use with heuristics
        self.thread_data.search_stack[ply].eval = eval;

        // Improving: if the current eval is better than 2 plies ago we prune/reduce differently
        let improving = ply >= 2
            && !in_check
            && self.thread_data.search_stack[ply].eval
                > self.thread_data.search_stack[ply - 2].eval;

        // Whole-node pruning techniques: these techniques will prune a whole node before move
        // generation and search is performed
        if !R::ROOT && !pv_node && !in_check {
            //Reverse Futility Pruning: if the static evaluation is high enough above beta assume we can skip search
            if depth <= self.options.rfp_depth
                && eval.saturating_sub(
                    depth as i16 * self.options.rfp_margin
                        + improving as i16 * self.options.rfp_improving_margin,
                ) >= beta
            {
                return eval
                    - (depth as i16 * self.options.rfp_margin
                        + improving as i16 * self.options.rfp_improving_margin);
            }

            // Null Move Pruning
            // if the opponent gets two moves in a row and the position is still good then prune
            let skip_nmp = !tt_move.is_null()
                && tt_depth >= depth - 2
                && tt_score <= alpha
                && tt_bound == UpperBound;
            if allow_nmp
                && !skip_nmp
                && depth >= self.options.nmp_depth
                && eval >= beta
                && board.has_non_pawn_material(current_player)
            {
                // reduce by at least 1
                let reduction = (self
                    .options
                    .nmp_const_reduction
                    .saturating_add(depth / self.options.nmp_linear_divisor)
                    .saturating_add(((eval - beta) / 200).min(3) as i8))
                .max(1);
                self.search_history.push(board.hash());
                self.thread_data.search_stack[ply].current_move = Move::null();
                let mut new = *board;
                new.make_null_move();
                let score = -self.negamax::<NotRoot, M>(
                    &new,
                    -beta,
                    -beta + 1,
                    (depth - reduction).max(0),
                    ply + 1,
                    &mut line,
                    tt,
                    // don't allow subsequent null moves
                    false,
                );
                self.search_history.pop();

                if score >= beta {
                    return score;
                }
            }
        }

        // Futility Pruning: if the static eval is bad enough skip quiet moves
        let fp_margins = [
            0,
            self.options.fp_margin_1,
            self.options.fp_margin_2,
            self.options.fp_margin_3,
        ];
        // decide if FP should be enabled
        let futility_pruning = !R::ROOT
            && !pv_node
            && !in_check
            && depth <= 3
            && eval + fp_margins[depth as usize] <= alpha;

        // move ordering: try heuristically good moves first to reduce the AB search tree
        let mut move_sorter = MoveSorter::<All>::new(tt_move);

        let mut best_move = Move::null();

        // save the old alpha to see if any moves improve the PV
        let old_alpha = alpha;

        // push this position to the history
        self.search_history.push(board.hash());

        let mut move_index = 0;
        let mut quiets_tried = MoveList::new();
        let mut captures_tried = MoveList::new();
        while let Some((mv, move_score)) = move_sorter.next(board, &mut self.thread_data, ply) {
            let capture = board.is_capture(mv);

            // Futility Pruning: skip quiets on nodes with bad static eval
            if futility_pruning
                && !capture
                && !(COUNTERMOVE_SCORE..KILLER_MOVE_SCORE + 50_000).contains(&move_score)
            {
                quiets_tried.push(SortingMove::new(mv));
                move_index += 1;
                continue;
            }

            // Late Move Pruning: skip moves ordered late, earlier if not improving
            if !R::ROOT
                && !pv_node
                && !capture
                && depth <= self.options.lmp_depth
                && quiets_tried.len() >= LMP_MARGINS[depth.min(31) as usize][improving as usize]
            {
                quiets_tried.push(SortingMove::new(mv));
                move_index += 1;
                continue;
            }

            // SEE pruning: if the move loses too much material at low depth then skip it
            if !R::ROOT && depth <= self.options.see_pruning_depth {
                let threshold = depth as i16
                    * if capture {
                        self.options.see_capture_margin
                    } else {
                        self.options.see_quiet_margin
                    };
                if !board.see_beats_threshold(mv, threshold) {
                    if !capture {
                        quiets_tried.push(SortingMove::new(mv));
                    } else {
                        captures_tried.push(SortingMove::new(mv));
                    }
                    move_index += 1;
                    continue;
                }
            }

            // make the move on a copy of the board
            self.thread_data.search_stack[ply].current_move = mv;
            let mut new = *board;
            new.make_move(mv);

            // legality check for the TT move, which is only verified as pseudolegal
            if mv == tt_move && new.illegal_position() {
                // skip the TT move if it's illegal
                continue;
            }

            let mut score = MINUS_INF;
            // perform a search on the new position, returning the score and the PV
            // allow LMR after the first move except at the root, where it is allowed after the second
            let full_depth_null_window = if depth > self.options.pvs_fulldepth
                && move_index > R::ROOT as usize
            {
                // reducing certain moves to same time, avoided for tactical and killer/counter moves
                let reduction = {
                    let mut r = 0;

                    // Late Move Reduction: moves that are sorted later are likely to fail low
                    if !capture
                        && !(COUNTERMOVE_SCORE..KILLER_MOVE_SCORE + 50_000).contains(&move_score)
                        && mv.promotion() != Queen
                    {
                        r += LMR[(depth as usize).min(63)][move_index.min(63)];

                        // reduce more outside of PV
                        r += !pv_node as i8;
                    }

                    r
                };

                // perform a cheap reduced, null-window search in the hope it fails low immediately
                let reduced_depth = (depth - 1 - reduction).max(0);
                score = -self.negamax::<NotRoot, M>(
                    &new,
                    -alpha - 1,
                    -alpha,
                    reduced_depth,
                    ply + 1,
                    &mut line,
                    tt,
                    true,
                );

                // perform a full-depth null-window search if the reduced search improves alpha and the move was actually reduced
                score > alpha && reduction > 0
            } else {
                // if the first condition fails, perform the full depth null window search in non-pv nodes or later moves in PVS
                !pv_node || move_index > 0
            };

            // perform a full-depth null-window search on reduced moves that improve alpha, later moves or in non-pv nodes
            // we can't expand the window in non-pv nodes as alpha = beta-1
            if full_depth_null_window {
                score = -self.negamax::<NotRoot, M>(
                    &new,
                    -alpha - 1,
                    -alpha,
                    depth - 1,
                    ply + 1,
                    &mut line,
                    tt,
                    true,
                );
            }

            // perform a full-depth full-window search in PV nodes on the first move and reduced moves that improve alpha
            if pv_node && (move_index == 0 || (score > alpha && score < beta)) {
                score = -self.negamax::<NotRoot, M>(
                    &new,
                    -beta,
                    -alpha,
                    depth - 1,
                    ply + 1,
                    &mut line,
                    tt,
                    true,
                );
            }

            // scores can't be trusted after an abort, don't let them get into the TT
            if ABORT_SEARCH.load(Relaxed) && depth > 1 {
                // remove this position from the history
                self.search_history.pop();
                return 0;
            }

            if score >= beta {
                // beta cutoff, this move is too good and so the opponent won't go into this position
                pv.clear();

                // add the score and move to TT
                tt.set(
                    board.hash(),
                    mv,
                    depth,
                    score_into_tt(score, ply),
                    LowerBound,
                    pv_node,
                );

                // update killer, countermove and history tables for good quiets
                let delta = if depth > 13 {
                    32
                } else {
                    4 * depth as i16 * depth as i16
                };
                if !capture {
                    self.thread_data.search_stack[ply].killer_moves.push(mv);

                    if let Some(last_move) = self
                        .thread_data
                        .search_stack
                        .get(ply - 1)
                        .map(|s| s.current_move)
                    {
                        self.thread_data.countermove_tables[current_player][last_move] = mv;
                    }

                    self.thread_data.update_quiet_histories(
                        current_player,
                        delta,
                        mv,
                        &quiets_tried,
                        ply,
                    );
                }
                // update capture histories for all moves that cause a beta cutoff
                self.thread_data.update_capture_histories(
                    current_player,
                    delta,
                    // provide the best move if it was a capture
                    capture.then_some(mv),
                    &captures_tried,
                );

                // remove this position from the history
                self.search_history.pop();

                return score;
            }
            if score > alpha {
                // a score between alpha and beta represents a new best move
                best_move = mv;

                // update the parent PV with the new PV
                pv.update_from(mv, &line);

                // raise alpha so worse moves after this one will be pruned early
                alpha = score;
            }
            // increment the move counter if the move was legal
            move_index += 1;
            if !capture {
                quiets_tried.push(SortingMove::new(mv));
            } else {
                captures_tried.push(SortingMove::new(mv));
            }
        }
        // remove this position from the history
        self.search_history.pop();

        // check for checkmate and stalemate
        if self.thread_data.search_stack[ply].move_list.is_empty() {
            pv.clear();
            return if in_check {
                // checkmate, preferring shorter mating sequences
                -(CHECKMATE_SCORE - (ply as i16))
            } else {
                // stalemate
                DRAW_SCORE
            };
        }

        // after all moves have been searched, alpha is either unchanged
        // (this position is bad) or raised (new pv from this node)
        // add the score and the new best move to the TT
        tt.set(
            board.hash(),
            best_move,
            depth,
            score_into_tt(alpha, ply),
            if alpha > old_alpha { Exact } else { UpperBound },
            pv_node,
        );

        if alpha == old_alpha {
            // no move was found from this position, clear the PV
            pv.clear();
        }
        alpha
    }

    pub fn quiesce<M: TypeMainThread>(
        &mut self,
        board: &Board,
        alpha: i16,
        beta: i16,
        ply: usize,
        pv: &mut PrincipalVariation,
        tt: &TranspositionTable,
    ) -> i16 {
        self.quiesce_impl::<(), M>(board, alpha, beta, ply, pv, tt)
            .0
    }

    pub fn quiesce_impl<T: TraceTarget + Default, M: TypeMainThread>(
        &mut self,
        board: &Board,
        mut alpha: i16,
        beta: i16,
        ply: usize,
        pv: &mut PrincipalVariation,
        tt: &TranspositionTable,
    ) -> (i16, T) {
        // check time and max nodes every 2048 nodes
        let nodes = self.local_nodes;
        if M::MAIN_THREAD && nodes & 2047 == 2047 {
            if let Some((_, abort_time)) = self.max_time_ms {
                // signal an abort if time has exceeded alloted time
                if Instant::now().duration_since(self.start_time).as_millis() as usize > abort_time
                {
                    ABORT_SEARCH.store(true, Relaxed);
                    return (0, T::default());
                }
            }
        }

        // check for abort
        if ABORT_SEARCH.load(Relaxed) || ply >= SEARCH_MAX_PLY {
            pv.clear();
            return (0, T::default());
        }

        // prefetch the transposition table: hint to the CPU that we want this in the cache well
        // before we actually use it
        tt.prefetch(board.hash());

        // increment node counters
        if M::MAIN_THREAD {
            NODE_COUNT.fetch_add(1, Relaxed);
        }
        self.local_nodes += 1;

        // increase the seldepth if this node is deeper
        self.seldepth = self.seldepth.max(ply);

        // check 50 move draw
        if board.halfmove_clock() >= 100 {
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
                    pv.clear();
                    return (score_from_tt(entry.score, ply), T::default());
                }

                // otherwise use the score as an improved static eval
                // and the move for move ordering
                if matches!(entry.node_type, LowerBound | Exact) {
                    tt_score = score_from_tt(entry.score, ply);
                }
                tt_move = Move::new(entry.piece, entry.move_from, entry.move_to, entry.promotion);
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
            pv.clear();
            return (beta, best_trace);
        }

        // if the static eval is better than alpha, use it to prune moves instead
        alpha = alpha.max(static_eval);

        let mut line = PrincipalVariation::new();

        // move ordering: try heuristically good moves first to reduce the AB search tree
        // quiescence search only looks at captures and promotions to ensure termination
        let mut move_sorter = MoveSorter::<Captures>::new(tt_move);

        let old_alpha = alpha;

        // add the current position to the history
        self.search_history.push(board.hash());

        let mut best_move = Move::null();
        while let Some((mv, _)) = move_sorter.next(board, &mut self.thread_data, ply) {
            // Delta Pruning: if this capture immediately falls short by some margin, skip it
            if static_eval
                .saturating_add(
                    board
                        .piece_on(mv.to())
                        .map(|p| SEE_PIECE_VALUES[p])
                        .unwrap_or(0),
                )
                .saturating_add(self.options.delta_pruning_margin)
                <= alpha
            {
                continue;
            }

            // SEE pruning: if the move loses material, skip it
            if !board.see_beats_threshold(mv, 0) {
                continue;
            }

            // make the move on a copy of the board
            let mut new = *board;
            new.make_move(mv);

            // legality check for the TT move, which is only verified as pseudolegal
            if mv == tt_move && new.illegal_position() {
                // skip the TT move if it's illegal
                continue;
            }

            let (mut score, trace) =
                self.quiesce_impl::<T, M>(&new, -beta, -alpha, ply + 1, &mut line, tt);
            score = -score;

            if score >= beta {
                // beta cutoff, this move is too good and so the opponent won't go into this position
                pv.clear();
                // add the score to the TT
                tt.set(
                    board.hash(),
                    mv,
                    -1,
                    score_into_tt(score, ply),
                    LowerBound,
                    false,
                );
                // return to the previous history state
                self.search_history.pop();
                return (score, trace);
            } else if score > alpha {
                // a score between alpha and beta represents a new best move
                best_move = mv;
                best_trace = trace;

                pv.update_from(best_move, &line);
                // raise alpha so worse moves after this one will be pruned early
                alpha = score;
            }
        }

        self.search_history.pop();

        // if there are no legal captures, check for checkmate/stalemate
        // disable when tracing to avoid empty traces
        if !T::TRACING && self.thread_data.search_stack[ply].move_list.is_empty() {
            let mut some_moves = false;
            board.generate_legal_moves(|mvs| some_moves = some_moves || mvs.moves.is_not_empty());

            if !some_moves {
                pv.clear();
                if board.in_check() {
                    return (-(CHECKMATE_SCORE - (ply as i16)), T::default());
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

        if alpha == old_alpha {
            // no move was found from this position, clear the PV
            pv.clear();
        }
        (alpha, best_trace)
    }
}
