use std::sync::{Arc, RwLock};
use std::time::Instant;
use std::{fmt::Display, sync::atomic::*};

use cheers_pregen::LMR;
use eval_params::{EvalParams, CHECKMATE_SCORE, DRAW_SCORE, EVAL_PARAMS};

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
pub static TIME_ELAPSED: AtomicBool = AtomicBool::new(false);
pub static SEARCH_COMPLETE: AtomicBool = AtomicBool::new(false);
pub static NODE_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static NPS_COUNT: AtomicUsize = AtomicUsize::new(0);

const INF: i32 = i32::MAX;
const MINUS_INF: i32 = -INF;

const SEARCH_MAX_PLY: usize = 128;

pub const PV_MAX_LEN: usize = 16;
#[derive(Copy, Clone, Default, Debug)]
pub struct PrincipalVariation {
    pub len: usize,
    pub moves: [Move; PV_MAX_LEN],
}

impl PrincipalVariation {
    pub fn new() -> Self {
        Self::default()
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
    pub position_history: Vec<u64>,
    pub move_lists: Vec<MoveList>,
    pub seldepth: usize,
    transposition_table: Arc<RwLock<TranspositionTable>>,
    pawn_hash_table: PawnHashTable,
    killer_moves: KillerMoves<2>,
    history_tables: [[[i32; 64]; 6]; 2],
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
            position_history: Vec::new(),
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
            position_history: Vec::new(),
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

    pub fn position_history(mut self, position_history: Vec<u64>) -> Self {
        self.position_history = position_history;
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
            let mut window = if i == 1 {
                (MINUS_INF, INF)
            } else {
                (last_score - window_size, last_score + window_size)
            };

            let mut pv = PrincipalVariation::new();

            // repeat failed searches with wider windows until a search succeeds
            let score = loop {
                search.seldepth = 0;

                let score =
                    search.negamax(self.game.clone(), window.0, window.1, i as i32, 0, Move::null(), &mut pv, tt);

                if ABORT_SEARCH.load(Ordering::Relaxed) && i > 1 {
                    // can't trust results from a partial search
                    break 'id_loop;
                }

                // Expand the search window based on which bound the search failed on
                match (score > window.0, score < window.1) {
                    // fail high, expand upper window
                    (true, false) => {
                        window = (window.0, window.1 + window_size);
                        window_size *= 2;
                    }
                    // fail low, expand lower window
                    (false, true) => {
                        window = (window.0 - window_size, window.1);
                        window_size *= 2;
                    }
                    // exact score within the window, search success
                    (true, true) => break score,
                    _ => unreachable!(),
                }
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
            if i > pv.len + 100 && pv.len != PV_MAX_LEN {
                ABORT_SEARCH.store(false, Ordering::Relaxed);
                break;
            }
        }
        SEARCH_COMPLETE.store(true, Ordering::Relaxed);
        (last_score, last_pv)
    }

    fn negamax(
        &mut self,
        board: Board,
        mut alpha: i32,
        mut beta: i32,
        mut depth: i32,
        ply: usize,
        last_move: Move,
        pv: &mut PrincipalVariation,
        tt: &TranspositionTable,
    ) -> i32 {
        // check time and max nodes every 2048 nodes
        let nodes = NODE_COUNT.load(Ordering::Relaxed);
        if nodes & 2047 == 2047 {
            if let Some((_, abort_time)) = self.max_time_ms {
                if Instant::now().duration_since(self.start_time).as_millis() as usize > abort_time
                {
                    ABORT_SEARCH.store(true, Ordering::Relaxed);
                }
            }
            if let Some(max_nodes) = self.max_nodes {
                if nodes >= max_nodes {
                    ABORT_SEARCH.store(true, Ordering::Relaxed);
                }
            }
        }

        // terminate search early
        if ply >= SEARCH_MAX_PLY || ABORT_SEARCH.load(Ordering::Relaxed) && depth > 1 {
            return 0;
        }

        let current_player = board.current_player();
        // check extension before quiescence
        let in_check = board.in_check();
        if in_check {
            depth += 1;
        }

        // quiescence search at full depth
        if depth == 0 {
            // exact score so we must reset the pv
            pv.len = 0;
            let score = self.quiesce(board.clone(), alpha, beta, ply, last_move, EVAL_PARAMS, tt);
            // self.transposition_table
            //     .set(self.hash, Move::null(), depth as i8, score, Exact);
            return score;
        }

        NODE_COUNT.fetch_add(1, Ordering::Relaxed);
        NPS_COUNT.fetch_add(1, Ordering::Relaxed);
        self.seldepth = self.seldepth.max(ply);

        // check 50 move and repetition draws
        if board.halfmove_clock() == 100
            || self
                .position_history
                .iter()
                .filter(|&&p| p == board.hash())
                .count()
                == 2
        {
            // exact score so we must reset the pv
            pv.len = 0;
            return DRAW_SCORE;
        }

        // Mate distance pruning
        if ply != 0 {
            alpha = alpha.max(-CHECKMATE_SCORE + ply as i32);
            beta = beta.min(CHECKMATE_SCORE - ply as i32);

            if alpha >= beta {
                return alpha;
            }
        }

        let mut line = PrincipalVariation::new();
        let pv_node = alpha != beta - 1;

        // transposition table lookup
        let mut tt_move = Move::null();
        if let Some(tt_entry) = tt.get(board.hash()) {
            // prune on exact score/beta cutoff with equal/higher depth, unless we are at the root
            if tt_entry.depth as i32 >= depth
                && ply != 0
                && (tt_entry.node_type == Exact
                    || (tt_entry.node_type == LowerBound && tt_entry.score >= beta)
                    || (tt_entry.node_type == UpperBound && tt_entry.score <= alpha))
            {
                // exact score (?) so we must reset the pv
                pv.len = 0;
                // mate score adjustment: re-distance mates relative to the current ply
                let score = if tt_entry.score > CHECKMATE_SCORE - 500 {
                    tt_entry.score - ply as i32
                } else if tt_entry.score < -CHECKMATE_SCORE + 500 {
                    tt_entry.score + ply as i32
                } else {
                    tt_entry.score
                };
                return score;
            }

            tt_move = Move {
                // TT moves are verified so fallinbg back to pawn is safe
                piece: board.piece_on(tt_entry.move_from).unwrap_or(Pawn),
                from: tt_entry.move_from,
                to: tt_entry.move_to,
                promotion: tt_entry.promotion,
            };
        }

        // IIR: reduce the depth if no TT move is found
        if depth >= self.options.iir_depth && tt_move.is_null() {
            depth -= 1;
        }

        let eval = board.evaluate(&mut self.pawn_hash_table);

        // Enable futility pruning
        let fp_margins = [
            0,
            self.options.fp_margin_1,
            self.options.fp_margin_2,
            self.options.fp_margin_3,
        ];
        let futility_pruning = !pv_node
            && ply != 0
            && depth as usize <= 3
            && !in_check
            && eval + fp_margins[depth as usize] <= alpha;

        // Reverse Futility pruning
        if !pv_node && ply != 0 && eval - (depth * self.options.rfp_margin) >= beta {
            return eval - (depth * self.options.rfp_margin);
        }

        // Null move pruning
        // don't search the null move in the PV, when in check or only down to pawn/kings
        if depth >= self.options.nmp_depth
            && !in_check
            && board.has_non_pawn_material(current_player)
        {
            self.position_history.push(board.hash());
            let mut new = board.clone();
            new.make_null_move();
            let null_score = -self.negamax(
                new,
                -beta,
                -beta + 1,
                (depth - self.options.nmp_reduction).max(0),
                ply + 1,
                Move::null(),
                &mut line,
                tt,
            );
            self.position_history.pop();

            if null_score >= beta {
                return null_score;
            }
        }

        board.generate_legal_moves_into(&mut self.move_lists[ply]);

        if self.move_lists[ply].is_empty() {
            // exact score, so we must reset the pv
            pv.len = 0;
            if in_check {
                // checkmate, preferring shorter mating sequences
                return -(CHECKMATE_SCORE - (ply as i32));
            } else {
                // stalemate
                return DRAW_SCORE;
            }
        }

        self.move_lists[ply]
            .inner_mut()
            .iter_mut()
            .for_each(|mut m| {
                // try the transposition table move early
                if m.mv.from == tt_move.from && m.mv.to == tt_move.to {
                    m.score += 100_000;
                } else if board.is_capture(m.mv) {
                    // winning captures first, then equal, then quiets, then losing
                    let see = board.see(m.mv);
                    if see < 0 {
                        m.score -= 50_000 - see;
                    } else {
                        m.score += 50_000 + see;
                    }
                }
                // order queen and rook promotions ahead of quiet moves
                else if m.mv.promotion == Queen || m.mv.promotion == Rook {
                    m.score += 20_000 + EVAL_PARAMS.piece_values[(Midgame, m.mv.promotion)];
                } else {
                    // quiet killer moves get sorted aove the other quiets
                    if self.killer_moves[ply.min(127)].contains(&m.mv) {
                        m.score += 6_000;
                    }
                    // the countermove get sorted above the other quiets
                    if !last_move.is_null() {
                        let countermove =
                            self.countermove_tables[current_player][last_move.piece][last_move.to];
                        if m.mv.from == countermove.from && m.mv.to == countermove.to {
                            m.score += 3_000;
                        }
                    }

                    // all quiets are sorted with their history heuristic
                    m.score += self.history_tables[current_player][m.mv.piece][m.mv.to];
                }
            });
        // make sure the reported best move is at least legal
        let mut best_move = self.move_lists[ply][0];

        let old_alpha = alpha;
        for i in 0..self.move_lists[ply].len() {
            let (mv, _) = self.move_lists[ply].pick_move(i);

            let capture = board.is_capture(mv);

            // Late Move Pruning: skip quiet moves ordered late
            if !pv_node
                && depth > self.options.lmp_depth
                && i > (self.options.lmp_margin * depth * depth) as usize
                && !capture
            {
                continue;
            }

            // Futility pruning: skip quiet moves when static eval is below alpha
            if futility_pruning && !capture {
                continue;
            }

            // SEE pruning
            if depth < self.options.see_pruning_depth && ply != 0 && i > 0 && mv.promotion == Pawn {
                let see = board.see(mv);
                let depth_margin = depth
                    * if capture {
                        self.options.see_capture_margin
                    } else {
                        self.options.see_quiet_margin
                    };
                if see <= -depth * depth_margin {
                    continue;
                }
            }

            self.position_history.push(board.hash());
            let mut new = board.clone();
            new.make_move(mv);
            let mut score = MINUS_INF;
            // reduced-depth null-window search on most moves outside of PV nodes
            let full_depth = if depth > self.options.pvs_fulldepth && i > 0 && ply != 0 {
                // reductions and extensions
                let reduction = {
                    let mut r = 0;

                    // Late Move Reduction (LMR)
                    if !capture && mv.promotion != Queen && !in_check {
                        r += LMR[(depth as usize).min(31)][i.min(31)]
                    }

                    // make sure we reduce by at least 1 to avoid infinite search
                    r.max(1)
                };
                let reduced_depth = (depth - reduction).max(1);
                score = -self.negamax(
                    new,
                    -alpha - 1,
                    -alpha,
                    reduced_depth,
                    ply + 1,
                    mv,
                    &mut line,
                    tt,
                );
                score > alpha && reduced_depth < depth - 1
            } else {
                !pv_node || i > 0
            };

            // full-depth null-window search on reduced moves that improved alpha, later moves or non-pv nodes
            if full_depth {
                score = -self.negamax(new, -alpha - 1, -alpha, depth - 1, ply + 1, mv, &mut line, tt);
            }

            // full-depth, full-window search on first move in PV nodes and reduced moves that improve alpha
            if pv_node && (i == 0 || (score > alpha && score < beta)) {
                score = -self.negamax(new, -beta, -alpha, depth - 1, ply + 1, mv, &mut line, tt);
            }

            self.position_history.pop();

            if score >= beta {
                tt.set(
                    board.hash(),
                    mv,
                    depth as i8,
                    if score > CHECKMATE_SCORE - 500 {
                        score + ply as i32
                    } else if score < -CHECKMATE_SCORE + 500 {
                        score - ply as i32
                    } else {
                        score
                    },
                    LowerBound,
                    pv_node,
                );
                if !capture {
                    // Update History Heuristic tables and scale to below 2000
                    self.history_tables[current_player][mv.piece][mv.to] += depth * depth;
                    if self.history_tables[current_player][mv.piece][mv.to] > 2_000 {
                        self.history_tables[current_player]
                            .iter_mut()
                            .flatten()
                            .for_each(|h| *h >>= 1);
                    }

                    // Update Countermove Heuristic table with the previous move
                    if !last_move.is_null() {
                        self.countermove_tables[current_player][last_move.piece][last_move.to] = mv;
                    }

                    // Update Killer Heuristic tables for this ply
                    if mv.promotion == Pawn {
                        self.killer_moves.push(mv, ply.min(127));
                    }
                }
                return score;
            }
            if score > alpha {
                // update PV
                pv.moves[0] = mv;
                pv.moves[1..((line.len + 1).min(PV_MAX_LEN))]
                    .copy_from_slice(&line.moves[..(line.len).min(PV_MAX_LEN - 1)]);
                pv.len = (line.len + 1).min(PV_MAX_LEN);
                alpha = score;
                best_move = mv;
            }
        }
        tt.set(
            board.hash(),
            best_move,
            depth as i8,
            if alpha > CHECKMATE_SCORE - 500 {
                alpha + ply as i32
            } else if alpha < -CHECKMATE_SCORE + 500 {
                alpha - ply as i32
            } else {
                alpha
            },
            if alpha != old_alpha {
                Exact
            } else {
                UpperBound
            },
            pv_node,
        );
        alpha
    }

    pub fn quiesce(
        &mut self,
        board: Board,
        alpha: i32,
        beta: i32,
        ply: usize,
        last_move: Move,
        eval_params: EvalParams,
        tt: &TranspositionTable,
    ) -> i32 {
        self.quiesce_impl::<()>(board, alpha, beta, ply, last_move, eval_params, tt)
            .0
    }

    pub fn quiesce_impl<T: TraceTarget + Default>(
        &mut self,
        board: Board,
        mut alpha: i32,
        beta: i32,
        ply: usize,
        _last_move: Move,
        eval_params: EvalParams,
        tt: &TranspositionTable,
    ) -> (i32, T) {
        // check time and max nodes every 2048 nodes
        let nodes = NODE_COUNT.load(Ordering::Relaxed);
        if nodes & 2047 == 2047 {
            if let Some((_, abort_time)) = self.max_time_ms {
                if Instant::now().duration_since(self.start_time).as_millis() as usize > abort_time
                {
                    ABORT_SEARCH.store(true, Ordering::Relaxed);
                }
            }
            if let Some(max_nodes) = self.max_nodes {
                if nodes >= max_nodes {
                    ABORT_SEARCH.store(true, Ordering::Relaxed);
                }
            }
        }

        NODE_COUNT.fetch_add(1, Ordering::Relaxed);
        NPS_COUNT.fetch_add(1, Ordering::Relaxed);

        self.seldepth = self.seldepth.max(ply);

        let (stand_pat_score, mut best_trace) = board.evaluate_impl::<T>(&mut self.pawn_hash_table);

        if stand_pat_score >= beta {
            return (beta, best_trace);
        }
        alpha = alpha.max(stand_pat_score);

        // transposition table lookup
        let mut tt_move = Move::null();
        if !T::TRACING {
            if let Some(tt_entry) = tt.get(board.hash()) {
                if tt_entry.node_type == Exact
                    || (tt_entry.node_type == LowerBound && tt_entry.score >= beta)
                    || (tt_entry.node_type == UpperBound && tt_entry.score <= alpha)
                {
                    // mate score adjustment: re-distance mates relative to the current ply
                    let score = if tt_entry.score > CHECKMATE_SCORE - 500 {
                        tt_entry.score - ply as i32
                    } else if tt_entry.score < -CHECKMATE_SCORE + 500 {
                        tt_entry.score + ply as i32
                    } else {
                        tt_entry.score
                    };
                    // TT isn't used in tracing eval so we can return a blank trace
                    return (score, T::default());
                }
                tt_move = Move {
                    piece: board.piece_on(tt_entry.move_from).unwrap_or(Pawn),
                    from: tt_entry.move_from,
                    to: tt_entry.move_to,
                    promotion: tt_entry.promotion,
                };
            }
        }
        board.generate_legal_captures_into(&mut self.move_lists[ply]);
        self.move_lists[ply].inner_mut().iter_mut().for_each(|m| {
            // try the transposition table move early
            if m.mv.from == tt_move.from && m.mv.to == tt_move.to {
                m.score += 10_000;
            }

            // Delta pruning: if this capture immediately falls short by some margin, skip it
            if stand_pat_score
                + board
                    .piece_on(m.mv.to)
                    .map(|p| SEE_PIECE_VALUES[p])
                    .unwrap_or(0)
                + self.options.delta_pruning_margin
                <= alpha
            {
                m.score = -1000;
            } else {
                let see = board.see(m.mv);
                if see < 0 {
                    // SEE pruning: skip all moves with negative SEE
                    m.score -= 2000 - see
                } else {
                    // order all captures by SEE
                    m.score += 2000 + see;
                }
            }
        });

        let old_alpha = alpha;
        let mut best_move = Move::null();
        for i in 0..self.move_lists[ply].len() {
            let (mv, score) = self.move_lists[ply].pick_move(i);

            // once we hit the first pruned move, skip all the rest
            if score < 0 {
                break;
            }

            self.position_history.push(board.hash());
            let mut new = board.clone();
            new.make_move(mv);
            let (mut score, trace) =
                self.quiesce_impl::<T>(new, -beta, -alpha, ply + 1, mv, eval_params, tt);
            score = -score;
            self.position_history.pop();
            if score >= beta {
                if !T::TRACING {
                    tt.set(
                        board.hash(),
                        mv,
                        -1,
                        if score > CHECKMATE_SCORE - 500 {
                            score + ply as i32
                        } else if score < -CHECKMATE_SCORE + 500 {
                            score - ply as i32
                        } else {
                            score
                        },
                        LowerBound,
                        false,
                    );
                }
                return (beta, trace);
            }
            if score > alpha {
                alpha = score;
                best_trace = trace;
                best_move = mv;
            }
        }
        if !T::TRACING {
            tt.set(
                board.hash(),
                best_move,
                -1,
                if alpha > CHECKMATE_SCORE - 500 {
                    alpha + ply as i32
                } else if alpha < -CHECKMATE_SCORE + 500 {
                    alpha - ply as i32
                } else {
                    alpha
                },
                if alpha != old_alpha {
                    Exact
                } else {
                    UpperBound
                },
                false,
            );
        }
        (alpha, best_trace)
    }
}
