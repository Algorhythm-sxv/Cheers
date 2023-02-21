use std::sync::atomic::*;
use std::sync::{atomic::Ordering::*, Arc, RwLock};
use std::time::Instant;

use cheers_pregen::LMR;
use eval_params::{CHECKMATE_SCORE, DRAW_SCORE};

use crate::board::see::SEE_PIECE_VALUES;
use crate::{
    board::{eval_types::TraceTarget, *},
    hash_tables::{score_from_tt, score_into_tt, NodeType::*, PawnHashTable, TranspositionTable},
    move_sorting::MoveSorter,
    moves::{KillerMoves, Move, MoveList, MoveScore, PrincipalVariation, NUM_KILLER_MOVES},
    options::SearchOptions,
    types::{All, Captures, NotRoot, Piece::*, Root, TypeRoot},
};

pub static ABORT_SEARCH: AtomicBool = AtomicBool::new(false);
pub static NODE_COUNT: AtomicUsize = AtomicUsize::new(0);

const INF: i16 = i16::MAX;
const MINUS_INF: i16 = -INF;

pub const SEARCH_MAX_PLY: usize = 128;

#[derive(Clone)]
pub struct Search {
    pub game: Board,
    pub search_history: Vec<u64>,
    pub pre_history: Vec<u64>,
    pub move_lists: Vec<MoveList>,
    pub seldepth: usize,
    transposition_table: Arc<RwLock<TranspositionTable>>,
    pawn_hash_table: PawnHashTable,
    pub killer_moves: KillerMoves<NUM_KILLER_MOVES>,
    pub history_tables: [[[i16; 64]; 6]; 2],
    pub countermove_tables: [[[Move; 64]; 6]; 2],
    pub max_depth: Option<usize>,
    pub max_nodes: Option<usize>,
    pub max_time_ms: Option<(usize, usize)>,
    pub abort_time_ms: Option<usize>,
    start_time: Instant,
    output: bool,
    options: SearchOptions,
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
            options: SearchOptions::default(),
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
            options: SearchOptions::default(),
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

    pub fn options(mut self, options: SearchOptions) -> Self {
        self.options = options;
        self
    }

    pub fn search(&self) -> (i16, PrincipalVariation) {
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

            let mut pv = PrincipalVariation::new();

            // repeat failed searches with wider windows until a search succeeds
            let score = loop {
                search.seldepth = 0;

                let score = search.negamax::<Root>(
                    &self.game.clone(),
                    window.0,
                    window.1,
                    i as i8,
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

    fn negamax<R: TypeRoot>(
        &mut self,
        board: &Board,
        mut alpha: i16,
        mut beta: i16,
        mut depth: i8,
        ply: usize,
        last_move: Move,
        pv: &mut PrincipalVariation,
        tt: &TranspositionTable,
    ) -> i16 {
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

        // Check extensions: increase depth by 1 when in check to avoid tactical blindness
        let in_check = board.in_check();
        if in_check {
            // saturating add to avoid negative depths on overflow
            depth = depth.saturating_add(1);
        }

        // drop into quiescence search at depth 0
        if depth == 0 {
            pv.clear();
            return self.quiesce(board, alpha, beta, ply, last_move, tt);
        }

        // increment the node counters
        NODE_COUNT.fetch_add(1, Relaxed);

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

        let eval = if tt_score != MINUS_INF {
            tt_score
        } else {
            board.evaluate(&mut self.pawn_hash_table)
        };

        // Reverse Futility Pruning: if the static evaluation is high enough above beta assume we can skip search
        if !pv_node && !R::ROOT && eval - (depth as i16 * self.options.rfp_margin) >= beta {
            return eval - (depth as i16 * self.options.rfp_margin);
        }

        // the PV from this node will be gathered into this array
        let mut line = PrincipalVariation::new();

        // Null Move Pruning
        // if the opponent gets two moves in a row and the position is still good then prune
        if !pv_node
            && !in_check
            && depth >= self.options.nmp_depth
            && board.has_non_pawn_material(current_player)
        {
            self.search_history.push(board.hash());
            let mut new = board.clone();
            new.make_null_move();
            let score = -self.negamax::<NotRoot>(
                &new,
                -beta,
                -beta + 1,
                (depth - self.options.nmp_reduction).max(0),
                ply + 1,
                Move::null(),
                &mut line,
                tt,
            );
            self.search_history.pop();

            if score >= beta {
                return score;
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
        while let Some((mv, move_score)) = move_sorter.next(
            board,
            &self.killer_moves[ply],
            &self.countermove_tables,
            &self.history_tables,
            last_move,
            &mut self.move_lists[ply],
        ) {
            let capture = board.is_capture(mv);

            // Futility Pruning: skip quiets on nodes with bad static eval
            if futility_pruning
                && !capture
                && !matches!(
                    move_score,
                    MoveScore::KillerMove(_) | MoveScore::CounterMove,
                )
            {
                move_index += 1;
                continue;
            }
            // make the move on a copy of the board
            let mut new = board.clone();
            new.make_move(mv);

            // legality check for the TT move, which is only verified as pseudolegal
            if mv == tt_move && new.illegal_position() {
                // skip the TT move if it's illegal
                continue;
            }

            // increment the move counter if the move was legal
            let i = move_index;
            move_index += 1;

            let mut score = MINUS_INF;
            // perform a search on the new position, returning the score and the PV
            let full_depth_null_window = if depth > self.options.pvs_fulldepth && i > 0 && !R::ROOT
            {
                // reducing certain moves to same time, avoided for tactical and killer/counter moves
                let reduction = {
                    let mut r = 0;

                    // Late Move Reduction: moves that are sorted later are likely to fail low
                    if !capture
                        && !matches!(
                            move_score,
                            MoveScore::KillerMove(_) | MoveScore::CounterMove
                        )
                        && mv.promotion != Queen
                        && !in_check
                    {
                        r += LMR[(depth as usize).min(31)][i.min(31)];
                    }

                    r
                };

                // perform a cheap reduced, null-window search in the hope it fails low immediately
                let reduced_depth = (depth - 1 - reduction).max(0);
                score = -self.negamax::<NotRoot>(
                    &new,
                    -alpha - 1,
                    -alpha,
                    reduced_depth,
                    ply + 1,
                    mv,
                    &mut line,
                    tt,
                );

                // perform a full-depth null-window search if the reduced search improves alpha and the move was actually reduced
                score > alpha && reduction > 0
            } else {
                // if the first condition fails, perform the full depth null window search in non-pv nodes or later moves in PVS
                !pv_node || i > 0
            };

            // perform a full-depth null-window search on reduced moves that improve alpha, later moves or in non-pv nodes
            // we can't expand the window in non-pv nodes as alpha = beta-1
            if full_depth_null_window {
                score = -self.negamax::<NotRoot>(
                    &new,
                    -alpha - 1,
                    -alpha,
                    depth - 1,
                    ply + 1,
                    mv,
                    &mut line,
                    tt,
                );
            }

            // perform a full-depth full-window search in PV nodes on the first move and reduced moves that improve alpha
            if pv_node && (i == 0 || (score > alpha && score < beta)) {
                score = -self.negamax::<NotRoot>(
                    &new,
                    -beta,
                    -alpha,
                    depth - 1,
                    ply + 1,
                    mv,
                    &mut line,
                    tt,
                );
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
                tt.set(
                    board.hash(),
                    mv,
                    depth as i8,
                    score_into_tt(score, ply),
                    LowerBound,
                    pv_node,
                );

                // update killer, countermove and history tables for good quiets
                if !capture {
                    self.killer_moves.push(mv, ply);
                    self.countermove_tables[current_player][last_move.piece][last_move.to] = mv;
                    self.history_tables[current_player][mv.piece][mv.to] += (depth * depth) as i16;
                    // scale history scores down if they get too high
                    if self.history_tables[current_player][mv.piece][mv.to] > 4096 {
                        self.history_tables[current_player]
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
                        self.history_tables[current_player][mv.piece][mv.to] -=
                            (depth * depth) as i16;
                        if self.history_tables[current_player][mv.piece][mv.to] < -4096 {
                            self.history_tables[current_player]
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

        // check for checkmate and stalemate
        if self.move_lists[ply].len() == 0 {
            if in_check {
                // checkmate, preferring shorter mating sequences
                pv.clear();
                return -(CHECKMATE_SCORE - (ply as i16));
            } else {
                // stalemate
                pv.clear();
                return DRAW_SCORE;
            }
        }

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
        alpha: i16,
        beta: i16,
        ply: usize,
        last_move: Move,
        tt: &TranspositionTable,
    ) -> i16 {
        self.quiesce_impl::<()>(board, alpha, beta, ply, last_move, tt)
            .0
    }

    pub fn quiesce_impl<T: TraceTarget + Default>(
        &mut self,
        board: &Board,
        mut alpha: i16,
        beta: i16,
        ply: usize,
        _last_move: Move,
        tt: &TranspositionTable,
    ) -> (i16, T) {
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

        // move ordering: try heuristically good moves first to reduce the AB search tree
        // quiescence search only looks at captures and promotions to ensure termination
        let mut move_sorter = MoveSorter::<Captures>::new(tt_move);

        let old_alpha = alpha;

        // add the current position to the history
        self.search_history.push(board.hash());

        let mut best_move = Move::null();
        while let Some((mv, _)) = move_sorter.next(
            board,
            &self.killer_moves[ply],
            &self.countermove_tables,
            &self.history_tables,
            Move::null(),
            &mut self.move_lists[ply],
        ) {
            // Delta Pruning: if this capture immediately falls short by some margin, skip it
            if static_eval
                + board
                    .piece_on(mv.to)
                    .map(|p| SEE_PIECE_VALUES[p])
                    .unwrap_or(0)
                + self.options.delta_pruning_margin
                <= alpha
            {
                continue;
            }

            // make the move on a copy of the board
            let mut new = board.clone();
            new.make_move(mv);

            // legality check for the TT move, which is only verified as pseudolegal
            if mv == tt_move && new.illegal_position() {
                // skip the TT move if it's illegal
                continue;
            }

            let (mut score, trace) = self.quiesce_impl::<T>(&new, -beta, -alpha, ply + 1, mv, tt);
            score = -score;

            if score >= beta {
                // beta cutoff, this move is too good and so the opponent won't go into this position

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

                // raise alpha so worse moves after this one will be pruned early
                alpha = score;
            }
        }

        self.search_history.pop();
        
        // if there are no legal captures, check for checkmate/stalemate
        // disable when tracing to avoid empty traces
        if !T::TRACING && self.move_lists[ply].len() == 0 {
            let mut some_moves = false;
            board.generate_legal_moves(|mvs| some_moves = some_moves || mvs.moves.is_not_empty());

            if !some_moves {
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

        (alpha, best_trace)
    }
}
