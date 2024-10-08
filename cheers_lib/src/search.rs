use std::sync::atomic::*;
use std::sync::{atomic::Ordering::*, Arc, RwLock};
use std::thread;
use std::time::Instant;

use cheers_bitboards::BitBoard;
use cheers_pregen::{LMP_MARGINS, LMR};
use eval_params::{CHECKMATE_SCORE, DRAW_SCORE};
use pyrrhic_rs::{DtzProbeValue, TableBases, WdlProbeResult};

use crate::board::see::SEE_PIECE_VALUES;
use crate::moves::*;
use crate::thread_data::ThreadData;
use crate::types::{HelperThread, MainThread, TypeMainThread};
use crate::{
    board::*,
    hash_tables::{score_from_tt, score_into_tt, NodeType::*, PawnHashTable, TranspositionTable},
    move_sorting::MoveSorter,
    options::SearchOptions,
    types::{All, Captures, NotRoot, Piece::*, Root, TypeRoot},
};

use self::evaluate::TB_WIN_SCORE;
use self::tb_adapter::MovegenAdapter;

pub static ABORT_SEARCH: AtomicBool = AtomicBool::new(false);
pub static NODE_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static TB_HITS: AtomicUsize = AtomicUsize::new(0);

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
    tablebases: Option<TableBases<MovegenAdapter>>,
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
    root_nodes: [[usize; 64]; 64],
}

impl Search {
    pub fn new(game: Board) -> Self {
        Self {
            game,
            search_history: Vec::new(),
            pre_history: Vec::new(),
            seldepth: 0,
            transposition_table: Arc::new(RwLock::new(TranspositionTable::new(0))),
            tablebases: None,
            pawn_hash_table: PawnHashTable::new(),
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
            root_nodes: [[0; 64]; 64],
        }
    }

    pub fn new_with_tt(game: Board, tt: Arc<RwLock<TranspositionTable>>) -> Self {
        Self {
            game,
            search_history: Vec::new(),
            pre_history: Vec::new(),
            seldepth: 0,
            transposition_table: tt,
            tablebases: None,
            pawn_hash_table: PawnHashTable::new(),
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
            root_nodes: [[0; 64]; 64],
        }
    }

    pub fn tt_size_mb(self, tt_size_mb: usize) -> Self {
        self.transposition_table
            .write()
            .unwrap()
            .set_size(tt_size_mb);
        self
    }

    pub fn tablebases(mut self, tablebases: Option<TableBases<MovegenAdapter>>) -> Self {
        self.tablebases = tablebases;
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

    pub fn smp_search(self) -> (i16, PrincipalVariation, Option<TableBases<MovegenAdapter>>) {
        ABORT_SEARCH.store(false, Relaxed);
        NODE_COUNT.store(0, Ordering::Relaxed);
        TB_HITS.store(0, Ordering::Relaxed);

        // if tablebases are available at the root, take the best move from there
        if let Some(ref tb) = self.tablebases {
            if self.game.piece_count() <= tb.max_pieces() {
                if let Ok(dtz_result) = self.game.probe_root(tb) {
                    if let DtzProbeValue::DtzResult(result) = dtz_result.root {
                        let tb_move = Move::from_dtz_result(&result);

                        let tb_score = if result.dtz > 0 {
                            TB_WIN_SCORE - result.dtz as i16
                        } else {
                            -TB_WIN_SCORE + result.dtz as i16
                        };

                        let mut tb_pv = PrincipalVariation::new();
                        tb_pv.push(tb_move);

                        if self.output {
                            println!(
                                "info string Syzygy WDL: {:?}, DTZ: {}",
                                result.wdl, result.dtz
                            );
                            println!(
                                "info depth 0 seldepth 0 score cp {tb_score} nodes 0 nps 0 tbhits 1 pv {}",
                                tb_move.coords()
                            )
                        }

                        // cloning the TB handle satisfies the borrow checker, the original
                        // will just be dropped immediately anyway
                        return (tb_score, tb_pv, Some(tb.clone()));
                    }
                }
            }
        }

        // Lazy SMP: start all threads at the same depth, communicating only
        // via the shared TT
        let mut score = MINUS_INF;
        let mut pv = PrincipalVariation::new();
        let mut tablebases = None;
        thread::scope(|s| {
            // helper threads: these only have their results added to the TT
            for _ in 1..self.options.threads {
                let search = self.clone();
                s.spawn(|| {
                    let _ = search.search::<HelperThread>(true);
                });
            }
            // main thread: this is the only thread that reports back over UCI
            (score, pv, tablebases) = self.search::<MainThread>(true);
            ABORT_SEARCH.store(true, Relaxed);
        });

        (score, pv, tablebases)
    }

    pub fn search<M: TypeMainThread>(
        mut self,
        set_global_abort: bool,
    ) -> (i16, PrincipalVariation, Option<TableBases<MovegenAdapter>>) {
        ABORT_SEARCH.store(false, Relaxed);
        if M::MAIN_THREAD {
            NODE_COUNT.store(0, Ordering::Relaxed);
        }
        let mut last_score = i16::MIN;
        let mut last_pv = PrincipalVariation::new();

        // fraction of main thread nodes spent on the best move
        let mut node_fraction = 0;

        let tt_handle = self.transposition_table.clone();
        let tt = &tt_handle.read().unwrap();

        let start = Instant::now();

        // Iterative Deepening: search with increasing depth, exploiting the results
        // of shallower searches to speed up deeper ones
        'id_loop: for i in 1..SEARCH_MAX_PLY {
            // Aspiration Window: search a narrow window around the score in hope of saving
            // some search time
            let mut window_size = 20;
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

            // repeat failed searches with wider windows until a search succeeds
            let score = loop {
                self.seldepth = 0;

                let score = self.negamax::<Root, M>(
                    &self.game.clone(),
                    window.0,
                    window.1,
                    i as i8,
                    0,
                    &mut pv,
                    tt,
                    true,
                );

                if M::MAIN_THREAD {
                    node_fraction =
                        (self.root_nodes[pv[0].from()][pv[0].to()] * 1000) / self.local_nodes;
                }

                // add helper thread nodes to global count
                if !M::MAIN_THREAD {
                    NODE_COUNT.fetch_add(self.local_nodes, Relaxed);
                    self.local_nodes = 0;
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
                        self.local_nodes
                    };
                    let tbhits_string = if self.tablebases.is_some() {
                        format!("tbhits {}", TB_HITS.load(Ordering::Relaxed))
                    } else {
                        String::new()
                    };

                    if M::MAIN_THREAD && self.output {
                        println!(
                            "info depth {} seldepth {} score {score_string} nodes {} nps {} {tbhits_string} hashfull {} time {} pv {last_pv}",
                            i-1,
                            self.seldepth,
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
                self.local_nodes
            };

            let tbhits_string = if self.tablebases.is_some() {
                format!("tbhits {} ", TB_HITS.load(Ordering::Relaxed))
            } else {
                String::new()
            };

            // we can trust the results from the previous search
            if M::MAIN_THREAD && self.output {
                println!(
                    "info depth {i} seldepth {} score {score_string} nodes {} nps {} {tbhits_string}hashfull {} time {} pv {pv}",
                    self.seldepth,
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

                // nodetm: if the fraction of nodes spent on the best move is very high, use less time
                // avoids spending too much time when the best move is 'obvious'
                // not done in fixed-time searches
                if stop_hint != abort_time
                    && (end - start).as_millis() as usize
                        > (stop_hint * (1500 - node_fraction)) / 1000
                {
                    break;
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
        (last_score, last_pv, self.tablebases)
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
        // check time every 2048 nodes in the main thread
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

        // increment the node counters and check max nodes
        self.local_nodes += 1;
        if M::MAIN_THREAD {
            let old_nodes = NODE_COUNT.fetch_add(1, Relaxed);
            // if this is the last node, allow it to complete, otherwise subtract this node from the count
            if self.max_nodes.is_some_and(|n| old_nodes >= n) {
                NODE_COUNT.fetch_sub(1, Relaxed);
                ABORT_SEARCH.store(true, Relaxed);
                pv.clear();
                return 0;
            }
        }

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
        let tt_entry = tt.get(board.hash());
        if let Some(entry) = &tt_entry {
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

        // Probe the Syzygy tablebases if they are available
        let (mut tb_max, mut tb_min) = (INF, MINUS_INF);
        if !R::ROOT
            && board.piece_count()
                <= self
                    .tablebases
                    .as_ref()
                    .map(|tb| tb.max_pieces())
                    .unwrap_or(0)
            && board.castling_rights() == &[[BitBoard::empty(); 2]; 2]
        {
            if let Some(tb) = &self.tablebases {
                if let Ok(wdl_result) = board.probe_wdl(tb) {
                    TB_HITS.fetch_add(1, Ordering::Relaxed);
                    let tb_score = match wdl_result {
                        WdlProbeResult::Loss => -TB_WIN_SCORE + ply as i16,
                        WdlProbeResult::Win => TB_WIN_SCORE - ply as i16,
                        _ => 0,
                    };

                    let tb_bound = match wdl_result {
                        WdlProbeResult::Loss => UpperBound,
                        WdlProbeResult::Win => LowerBound,
                        _ => Exact,
                    };

                    if tb_bound == Exact
                        || (tb_bound == LowerBound && tb_score >= beta)
                        || (tb_bound == UpperBound && tb_score <= alpha)
                    {
                        // store TB score in the TT for cutoffs
                        tt.set(
                            board.hash(),
                            Move::null(),
                            depth,
                            tb_score,
                            tb_bound,
                            pv_node,
                        );
                        pv.clear();
                        return tb_score;
                    }

                    if pv_node && tb_bound == LowerBound {
                        alpha = alpha.max(tb_score);
                        tb_min = tb_score;
                    }

                    if pv_node && tb_bound == UpperBound {
                        tb_max = tb_score;
                    }
                }
            }
        }

        // IIR: reduce the search depth if the position was missing in the TT
        if !R::ROOT && !pv_node && depth >= self.options.iir_depth && tt_entry.is_none() {
            depth -= 1;
        }

        let mut eval = if in_check {
            // static eval isn't valid when in check
            MINUS_INF
        } else {
            board.evaluate(&mut self.pawn_hash_table)
        };
        if tt_entry.is_some()
            && (tt_bound == Exact
                || (tt_bound == LowerBound && tt_score > eval)
                || (tt_bound == UpperBound && tt_score < eval))
        {
            eval = tt_score;
        }

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
            let skip_nmp = tt_entry.is_some()
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
                let mut score = -self.negamax::<NotRoot, M>(
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
                    // don't let TB results leak out of NMP
                    if score >= TB_WIN_SCORE - SEARCH_MAX_PLY as i16 {
                        score = beta;
                    }
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
        let mut best_score = MINUS_INF;

        // push this position to the history
        self.search_history.push(board.hash());

        let mut move_index = 0;
        let mut quiets_tried = MoveList::new();
        let mut captures_tried = MoveList::new();
        while let Some((mv, move_score)) = move_sorter.next(board, &mut self.thread_data, ply) {
            let capture = board.is_capture(mv);

            // Move-based pruning techniques, not done until we have searched at least one move
            if move_index >= 1 {
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
            }

            let old_nodes = self.local_nodes;

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

                        // reduce more or less relative to history
                        let histories = self.thread_data.get_quiet_history(mv, current_player, ply);
                        r -= (histories / self.options.history_lmr_divisor) as i8;
                        // don't allow negative reductions
                        r = r.max(0);
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

            // count the nodes used for this particular move at the root
            if M::MAIN_THREAD && R::ROOT {
                self.root_nodes[mv.from()][mv.to()] += self.local_nodes - old_nodes;
            }

            // scores can't be trusted after an abort, don't let them get into the TT
            if ABORT_SEARCH.load(Relaxed) && depth > 1 {
                // remove this position from the history
                self.search_history.pop();
                pv.clear();
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
                        .get(ply.wrapping_sub(1))
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
            if score > best_score {
                best_score = score;
                if score > alpha {
                    // a score between alpha and beta represents a new best move
                    best_move = mv;

                    // update the parent PV with the new PV
                    pv.update_from(mv, &line);

                    // raise alpha so worse moves after this one will be pruned early
                    alpha = score;
                }
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
        if self.thread_data.search_stack[ply].num_moves() == 0 {
            pv.clear();
            return if in_check {
                // checkmate, preferring shorter mating sequences
                -(CHECKMATE_SCORE - (ply as i16))
            } else {
                // stalemate
                DRAW_SCORE
            };
        }

        // don't allow scores better or worse than the retrieved tb value to get into the TT
        best_score = best_score.clamp(tb_min, tb_max);

        // after all moves have been searched, alpha is either unchanged
        // (this position is bad) or raised (new pv from this node)
        // add the score and the new best move to the TT
        tt.set(
            board.hash(),
            best_move,
            depth,
            score_into_tt(best_score, ply),
            if best_score > old_alpha {
                Exact
            } else {
                UpperBound
            },
            pv_node,
        );

        if alpha == old_alpha {
            // no move was found from this position, clear the PV
            pv.clear();
        }
        best_score
    }

    pub fn quiesce<M: TypeMainThread>(
        &mut self,
        board: &Board,
        mut alpha: i16,
        beta: i16,
        ply: usize,
        pv: &mut PrincipalVariation,
        tt: &TranspositionTable,
    ) -> i16 {
        // check time every 2048 nodes
        let nodes = self.local_nodes;
        if M::MAIN_THREAD && nodes & 2047 == 2047 {
            if let Some((_, abort_time)) = self.max_time_ms {
                // signal an abort if time has exceeded alloted time
                if Instant::now().duration_since(self.start_time).as_millis() as usize > abort_time
                {
                    ABORT_SEARCH.store(true, Relaxed);
                    pv.clear();
                    return 0;
                }
            }
        }

        // check for abort
        if ABORT_SEARCH.load(Relaxed) || ply >= SEARCH_MAX_PLY {
            pv.clear();
            return 0;
        }

        // prefetch the transposition table: hint to the CPU that we want this in the cache well
        // before we actually use it
        tt.prefetch(board.hash());

        // increment node counters and check for max nodes
        self.local_nodes += 1;
        if M::MAIN_THREAD {
            let old_nodes = NODE_COUNT.fetch_add(1, Relaxed);
            // if this is the last node, allow it to complete, otherwise subtract this node from the count
            if self.max_nodes.is_some_and(|n| old_nodes >= n) {
                NODE_COUNT.fetch_sub(1, Relaxed);
                ABORT_SEARCH.store(true, Relaxed);
                pv.clear();
                return 0;
            }
        }

        // increase the seldepth if this node is deeper
        self.seldepth = self.seldepth.max(ply);

        // check 50 move draw
        if board.halfmove_clock() >= 100 {
            return DRAW_SCORE;
        }

        // Transposition Table lookup
        let mut tt_move = Move::null();
        let mut tt_score = MINUS_INF;
        let mut tt_bound = UpperBound;
        let tt_entry = tt.get(board.hash());
        if let Some(entry) = &tt_entry {
            // TT pruning when the bounds are correct
            if entry.node_type == Exact
                || (entry.node_type == LowerBound && entry.score >= beta)
                || (entry.node_type == UpperBound && entry.score <= alpha)
            {
                pv.clear();
                return score_from_tt(entry.score, ply);
            }

            // otherwise use the score as an improved static eval
            // and the move for move ordering
            tt_score = score_from_tt(entry.score, ply);
            tt_bound = entry.node_type;
            tt_move = Move::new(entry.piece, entry.move_from, entry.move_to, entry.promotion);
        }

        // the static evaluation allows us to prune moves that are worse than 'standing pat' at this node
        let mut static_eval = board.evaluate(&mut self.pawn_hash_table);
        if tt_entry.is_some()
            && (tt_bound == Exact
                || (tt_bound == LowerBound && tt_score > static_eval)
                || (tt_bound == UpperBound && tt_score < static_eval))
        {
            static_eval = tt_score;
        }

        // if the static eval is above beta, then the opponent won't play into this position
        if static_eval >= beta {
            pv.clear();
            return static_eval;
        }

        // if the static eval is better than alpha, use it to prune moves instead
        alpha = alpha.max(static_eval);
        let old_alpha = alpha;

        let mut line = PrincipalVariation::new();

        // move ordering: try heuristically good moves first to reduce the AB search tree
        // quiescence search only looks at captures and promotions to ensure termination
        let mut move_sorter = MoveSorter::<Captures>::new(tt_move);

        // add the current position to the history
        self.search_history.push(board.hash());

        let mut best_move = Move::null();
        let mut best_score = static_eval;
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

            let score = -self.quiesce::<M>(&new, -beta, -alpha, ply + 1, &mut line, tt);

            // can't trust scores after an abort, don't let them get into the TT
            if ABORT_SEARCH.load(Relaxed) {
                self.search_history.pop();
                pv.clear();
                return 0;
            }

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
                return score;
            }
            if score > best_score {
                best_score = score;
                if score > alpha {
                    // a score between alpha and beta represents a new best move
                    best_move = mv;

                    pv.update_from(best_move, &line);
                    // raise alpha so worse moves after this one will be pruned early
                    alpha = score;
                }
            }
        }

        self.search_history.pop();

        // if there are no legal captures, check for checkmate/stalemate
        if self.thread_data.search_stack[ply].num_moves() == 0 {
            let mut some_moves = false;
            board.generate_legal_moves(|mvs| some_moves = some_moves || mvs.moves.is_not_empty());

            if !some_moves {
                pv.clear();
                if board.in_check() {
                    return -(CHECKMATE_SCORE - (ply as i16));
                } else {
                    return DRAW_SCORE;
                }
            }
        }

        // after all moves are searched alpha is either unchanged (this position is bad) or raised (new pv)
        // add the score to the TT
        tt.set(
            board.hash(),
            best_move,
            -1,
            score_into_tt(best_score, ply),
            if best_score > old_alpha {
                Exact
            } else {
                UpperBound
            },
            false,
        );

        if alpha == old_alpha {
            // no move was found from this position, clear the PV
            pv.clear();
        }
        best_score
    }
}
