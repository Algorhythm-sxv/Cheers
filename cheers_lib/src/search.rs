use std::time::Instant;
use std::{fmt::Display, sync::atomic::*};

use cheers_pregen::LMR;

use crate::chessgame::movegen::{All, Captures, MoveList};
use crate::chessgame::see::SEE_PIECE_VALUES;
use crate::hash_tables::{NodeType::*, PawnHashTable, TranspositionTable};
use crate::moves::{pick_move, KillerMoves};
use crate::{
    chessgame::{
        eval_types::{GamePhase::*, TraceTarget},
        *,
    },
    moves::Move,
    types::PieceIndex::*,
};

pub static ABORT_SEARCH: AtomicBool = AtomicBool::new(false);
pub static TIME_ELAPSED: AtomicBool = AtomicBool::new(false);
pub static SEARCH_COMPLETE: AtomicBool = AtomicBool::new(false);
pub static NODE_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static NPS_COUNT: AtomicUsize = AtomicUsize::new(0);

const INF: i32 = i32::MAX;
const MINUS_INF: i32 = -INF;

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
}

pub const NMP_DEPTH: i32 = 2;
pub const NMP_REDUCTION: i32 = 4;
pub const SEE_PRUNING_DEPTH: i32 = 6;
pub const SEE_CAPTURE_MARGIN: i32 = 93;
pub const SEE_QUIET_MARGIN: i32 = 32;
pub const PVS_FULLDEPTH: i32 = 1;
pub const DELTA_PRUNING_MARGIN: i32 = 118;

impl Default for EngineOptions {
    fn default() -> Self {
        Self { tt_size_mb: 8 }
    }
}

#[derive(Clone)]
pub struct Search {
    pub game: ChessGame,
    pub move_lists: Vec<MoveList>,
    pub seldepth: usize,
    transposition_table: TranspositionTable,
    pawn_hash_table: PawnHashTable,
    killer_moves: KillerMoves<2>,
    history_tables: [[[i32; 64]; 6]; 2],
    pub max_depth: Option<usize>,
    pub max_nodes: Option<usize>,
    pub max_time_ms: Option<usize>,
    pub abort_time_ms: Option<usize>,
    output: bool,
    options: EngineOptions,
}

impl Search {
    pub fn new(game: ChessGame) -> Self {
        Self {
            game,
            move_lists: vec![MoveList::new(); 128],
            seldepth: 0,
            transposition_table: TranspositionTable::new(0),
            pawn_hash_table: PawnHashTable::new(0),
            killer_moves: KillerMoves::new(),
            history_tables: [[[0; 64]; 6]; 2],
            max_depth: None,
            max_nodes: None,
            max_time_ms: None,
            abort_time_ms: None,
            output: false,
            options: EngineOptions::default(),
        }
    }

    pub fn tt_size_mb(mut self, tt_size_mb: usize) -> Self {
        self.transposition_table.set_size(tt_size_mb);
        self.pawn_hash_table = PawnHashTable::new(tt_size_mb / 8);
        self
    }

    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    pub fn max_nodes(mut self, nodes: usize) -> Self {
        self.max_nodes = Some(nodes);
        unimplemented!("Max nodes is currently unsupported!");
        #[allow(unreachable_code)]
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
        let start = Instant::now();
        for i in 0.. {
            search.seldepth = 0;
            let mut pv = PrincipalVariation::new();
            let score = search.negamax(MINUS_INF, INF, i as i32, 0, Move::null(), &mut pv);
            if ABORT_SEARCH.load(Ordering::Relaxed) && i > 1 {
                // can't trust results from a partial search
                break;
            }
            let end = Instant::now();
            let mate_distance = CHECKMATE_SCORE - score.abs();
            let score_string = if mate_distance < 100 {
                format!("mate {}", score.signum() * ((mate_distance + 1) / 2))
            } else {
                format!("cp {score}")
            };
            let hash_fill = self.transposition_table.sample_fill();
            // we can trust the results from the previous search
            if self.output {
                println!(
                    "info depth {i} seldepth {} score {score_string} pv {pv} nodes {} hashfull {} time {}",
                    search.seldepth,
                    NODE_COUNT.load(Ordering::Relaxed),
                    hash_fill,
                    (end - start).as_millis(),
                )
            };

            last_pv = pv;
            last_score = score;
            // terminate search if we are hinted to do so
            if TIME_ELAPSED.load(Ordering::Relaxed) && i > 1 {
                break;
            }

            // terminate search at max depth or with forced mate/draw
            if let Some(max_depth) = self.max_depth {
                if i == max_depth {
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
        mut alpha: i32,
        mut beta: i32,
        depth: i32,
        ply: usize,
        last_move: Move,
        pv: &mut PrincipalVariation,
    ) -> i32 {
        // terminate search early
        if ABORT_SEARCH.load(Ordering::Relaxed) && depth > 1 {
            return 0;
        }

        // check extension before quiescence
        let in_check = self.game.in_check(self.game.current_player());
        let depth = if in_check { depth + 1 } else { depth };

        // quiescence search at full depth
        if depth == 0 {
            // exact score so we must reset the pv
            pv.len = 0;
            let score = self.quiesce(alpha, beta, 0, ply, last_move, EVAL_PARAMS);
            // self.transposition_table
            //     .set(self.hash, Move::null(), depth as i8, score, Exact);
            return score;
        }

        NODE_COUNT.fetch_add(1, Ordering::Relaxed);
        NPS_COUNT.fetch_add(1, Ordering::Relaxed);
        self.seldepth = self.seldepth.max(ply);

        // check 50 move and repetition draws
        if self.game.halfmove_clock() == 100
            || self
                .game
                .position_history()
                .iter()
                .filter(|&&p| p == self.game.hash())
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
        if let Some(tt_entry) = self.transposition_table.get(self.game.hash()) {
            // prune on exact score/beta cutoff with equal/higher depth, unless we are at the root
            if tt_entry.depth as i32 >= depth
                && ply != 0
                && (tt_entry.node_type == Exact
                    || (tt_entry.node_type == LowerBound && tt_entry.score >= beta)
                    || (tt_entry.node_type == UpperBound && tt_entry.score <= alpha))
            {
                // exact score (?) so we must reset the pv
                pv.len = 0;
                // mate score adustment: re-distance mates relative to the current ply
                let score = if tt_entry.score > CHECKMATE_SCORE - 500 {
                    tt_entry.score - ply as i32
                } else if tt_entry.score < -CHECKMATE_SCORE + 500 {
                    tt_entry.score + ply as i32
                } else {
                    tt_entry.score
                };
                return score;
            }

            tt_move = Move::new(
                tt_entry.move_start,
                tt_entry.move_target,
                self.game.piece_at(tt_entry.move_start),
                tt_entry.promotion,
                tt_entry.en_passent_capture || self.game.piece_at(tt_entry.move_target) != NoPiece,
                tt_entry.double_pawn_push,
                tt_entry.en_passent_capture,
                tt_entry.castling,
            );
        }

        // Null move pruning
        // don't search the null move in the PV, when in check or only down to pawn/kings
        if depth >= NMP_DEPTH
            && !self.game.in_check(self.game.current_player())
            && self.game.has_non_pawn_material(self.game.current_player())
        {
            self.game.make_null_move();
            let null_score = -self.negamax(
                -beta,
                -beta + 1,
                (depth - NMP_REDUCTION).max(0),
                ply + 1,
                Move::null(),
                &mut line,
            );
            self.game.unmake_null_move();

            if null_score >= beta {
                return null_score;
            }
        }

        self.game
            .generate_legal_moves::<All>(&mut self.move_lists[ply]);

        if self.move_lists[ply].is_empty() {
            // exact score, so we must reset the pv
            pv.len = 0;
            if self.game.in_check(self.game.current_player()) {
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
                if m.start() == tt_move.start() && m.target() == tt_move.target() {
                    m.score += 100_000;
                } else if m.capture() {
                    // winning captures first, then equal, then quiets, then losing
                    let see = self.game.see(*m);
                    if see < 0 {
                        m.score -= 50_000 - see;
                    } else {
                        m.score += 50_000 + see;
                    }
                }
                // order queen and rook promotions ahead of quiet moves
                else if m.promotion() == Queen || m.promotion() == Rook {
                    m.score += 10_000 + EVAL_PARAMS.piece_values[(Midgame, m.promotion())];
                } else {
                    // quiet killer moves get sorted before other quiet moves
                    if self.killer_moves[ply.min(127)].contains(&m) {
                        m.score += 5_000;
                    }
                    // quiet moves get ordered by their history heuristic
                    m.score += self.history_tables[self.game.current_player()][m.piece()]
                        [*m.target() as usize];
                }
            });
        // make sure the reported best move is at least legal
        let mut best_move = *self.move_lists[ply].inner().first().unwrap();

        let old_alpha = alpha;
        for i in 0..self.move_lists[ply].len() {
            pick_move(self.move_lists[ply].inner_mut(), i);
            let move_ = self.move_lists[ply][i];

            // SEE pruning
            if depth < SEE_PRUNING_DEPTH && ply != 0 && i > 0 && move_.promotion() == NoPiece {
                let see = self.game.see(move_);
                let depth_margin = depth
                    * if move_.capture() {
                        SEE_CAPTURE_MARGIN
                    } else {
                        SEE_QUIET_MARGIN
                    };
                if see <= -depth * depth_margin {
                    continue;
                }
            }

            self.game.make_move(move_);
            let mut score = MINUS_INF;
            // reduced-depth null-window search on most moves outside of PV nodes
            let full_depth = if depth > PVS_FULLDEPTH && i > 0 && ply != 0 {
                // reductions and extensions
                let reduction = {
                    let mut r = 0;

                    // Late Move Reduction (LMR)
                    if !move_.capture() && move_.promotion() != Queen && !in_check {
                        r += LMR[(depth as usize).min(31)][i.min(31)]
                    }

                    // make sure we reduce by at least 1 to avoid infinite search
                    r.max(1)
                };
                let reduced_depth = (depth - reduction).max(1);
                score = -self.negamax(-alpha - 1, -alpha, reduced_depth, ply + 1, move_, &mut line);
                score > alpha && reduced_depth < depth - 1
            } else {
                !pv_node || i > 0
            };

            // full-depth null-window search on reduced moves that improved alpha, later moves or non-pv nodes
            if full_depth {
                score = -self.negamax(-alpha - 1, -alpha, depth - 1, ply + 1, move_, &mut line);
            }

            // full-depth, full-window search on first move in PV nodes and reduced moves that improve alpha
            if pv_node && (i == 0 || (score > alpha && score < beta)) {
                score = -self.negamax(-beta, -alpha, depth - 1, ply + 1, move_, &mut line);
            }

            self.game.unmake_move();
            if score >= beta {
                self.transposition_table.set(
                    self.game.hash(),
                    move_,
                    depth as i8,
                    if score > CHECKMATE_SCORE - 500 {
                        score + ply as i32
                    } else if score < -CHECKMATE_SCORE + 500 {
                        score - ply as i32
                    } else {
                        score
                    },
                    LowerBound,
                );
                if !move_.capture() {
                    self.history_tables[self.game.current_player()][move_.piece()]
                        [*move_.target() as usize] += depth * depth;
                    if self.history_tables[self.game.current_player()][move_.piece()]
                        [move_.target()]
                        > 2_000
                    {
                        self.history_tables[self.game.current_player()]
                            .iter_mut()
                            .flatten()
                            .for_each(|h| *h >>= 1);
                    }
                    if move_.promotion() == NoPiece {
                        self.killer_moves.push(move_, ply.min(127));
                    }
                }
                return score;
            }
            if score > alpha {
                // update PV
                pv.moves[0] = move_;
                pv.moves[1..((line.len + 1).min(PV_MAX_LEN))]
                    .copy_from_slice(&line.moves[..(line.len).min(PV_MAX_LEN - 1)]);
                pv.len = (line.len + 1).min(PV_MAX_LEN);
                alpha = score;
                best_move = move_;
            }
        }
        self.transposition_table.set(
            self.game.hash(),
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
        );
        alpha
    }

    pub fn quiesce(
        &mut self,
        alpha: i32,
        beta: i32,
        depth: i32,
        ply: usize,
        last_move: Move,
        eval_params: EvalParams,
    ) -> i32 {
        self.quiesce_impl::<()>(alpha, beta, depth, ply, last_move, eval_params)
            .0
    }

    pub fn quiesce_impl<T: TraceTarget + Default>(
        &mut self,
        mut alpha: i32,
        beta: i32,
        depth: i32,
        ply: usize,
        _last_move: Move,
        eval_params: EvalParams,
    ) -> (i32, T) {
        NODE_COUNT.fetch_add(1, Ordering::Relaxed);
        NPS_COUNT.fetch_add(1, Ordering::Relaxed);

        self.seldepth = self.seldepth.max(ply);

        let (stand_pat_score, mut best_trace) =
            self.game.evaluate_impl::<T>(&mut self.pawn_hash_table);

        if stand_pat_score >= beta {
            return (beta, best_trace);
        }
        alpha = alpha.max(stand_pat_score);

        // transposition table lookup
        let mut tt_move = Move::null();
        if !T::TRACING {
            if let Some(tt_entry) = self.transposition_table.get(self.game.hash()) {
                if tt_entry.depth as i32 >= depth
                    && ply != 0
                    && (tt_entry.node_type == Exact
                        || (tt_entry.node_type == LowerBound && tt_entry.score >= beta)
                        || (tt_entry.node_type == UpperBound && tt_entry.score <= alpha))
                {
                    // TT isn't used in tracing eval so we can return a blank trace
                    return (tt_entry.score, T::default());
                }
                tt_move = Move::new(
                    tt_entry.move_start,
                    tt_entry.move_target,
                    self.game.piece_at(tt_entry.move_start),
                    tt_entry.promotion,
                    tt_entry.en_passent_capture
                        || self.game.piece_at(tt_entry.move_target) != NoPiece,
                    tt_entry.double_pawn_push,
                    tt_entry.en_passent_capture,
                    tt_entry.castling,
                );
            }
        }
        self.game
            .generate_legal_moves::<Captures>(&mut self.move_lists[ply]);
        self.move_lists[ply].inner_mut().iter_mut().for_each(|m| {
            // try the transposition table move early
            if m.start() == tt_move.start() && m.target() == tt_move.target() {
                m.score += 10_000;
            }

            // Delta pruning: if this capture immediately falls short by some margin, skip it
            if stand_pat_score
                + SEE_PIECE_VALUES[self.game.piece_at(m.target())]
                + DELTA_PRUNING_MARGIN
                <= alpha
            {
                m.score = -1000;
            } else {
                let see = self.game.see(*m);
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
            pick_move(self.move_lists[ply].inner_mut(), i);
            let move_ = self.move_lists[ply][i];

            // once we hit the first pruned move, skip all the rest
            if move_.score < 0 {
                break;
            }

            self.game.make_move(move_);
            let (mut score, trace) =
                self.quiesce_impl::<T>(-beta, -alpha, depth - 1, ply + 1, move_, eval_params);
            score = -score;
            self.game.unmake_move();
            if score >= beta {
                if !T::TRACING {
                    self.transposition_table.set(
                        self.game.hash(),
                        move_,
                        depth as i8,
                        score,
                        LowerBound,
                    );
                }
                return (beta, trace);
            }
            if score > alpha {
                alpha = score;
                best_trace = trace;
                best_move = move_;
            }
        }
        if !T::TRACING {
            self.transposition_table.set(
                self.game.hash(),
                best_move,
                depth as i8,
                alpha,
                if alpha != old_alpha {
                    Exact
                } else {
                    UpperBound
                },
            );
        }
        (alpha, best_trace)
    }
}
