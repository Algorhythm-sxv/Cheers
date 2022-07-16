use std::{fmt::Display, sync::atomic::*};

use crate::moves::{pick_move, KillerMoves};
use crate::transposition_table::{NodeType::*, TranspositionTable};
use crate::{
    chessgame::{
        eval_types::{GamePhase::*, TraceTarget},
        *,
    },
    moves::Move,
    types::PieceIndex::*,
};

pub static RUN_SEARCH: AtomicBool = AtomicBool::new(false);
pub static NODE_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static NPS_COUNT: AtomicUsize = AtomicUsize::new(0);

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

#[derive(Clone)]
pub struct Search {
    pub game: ChessGame,
    transposition_table: TranspositionTable,
    killer_moves: KillerMoves<2>,
    history_tables: [[[i32; 64]; 6]; 2],
    pub max_depth: Option<usize>,
    pub max_nodes: Option<usize>,
    pub max_time_ms: Option<usize>,
    output: bool,
}

impl Search {
    pub fn new(game: ChessGame) -> Self {
        Self {
            game,
            transposition_table: TranspositionTable::new(0),
            killer_moves: KillerMoves::new(),
            history_tables: [[[0; 64]; 6]; 2],
            max_depth: None,
            max_nodes: None,
            max_time_ms: None,
            output: false,
        }
    }

    pub fn tt_size_mb(mut self, tt_size_mb: usize) -> Self {
        self.transposition_table.set_size(tt_size_mb);
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

    pub fn search(&self) -> (i32, PrincipalVariation) {
        RUN_SEARCH.store(true, Ordering::Relaxed);
        let mut score = i32::MIN;
        let mut last_pv = PrincipalVariation::new();

        let mut search = self.clone();
        for i in 0.. {
            let mut pv = PrincipalVariation::new();
            score = search.negamax(
                i32::MIN + 1,
                i32::MAX - 1,
                i as i32,
                0,
                Move::null(),
                &mut pv,
            );
            if !RUN_SEARCH.load(Ordering::Relaxed) && i > 1 {
                // can't trust results from a partial search
                break;
            }

            if self.output {
                println!(
                    "info depth {i} score cp {score} pv {pv} nodes {}",
                    NODE_COUNT.load(Ordering::Relaxed)
                )
            };

            // terminate search at max depth or with forced mate/draw
            if let Some(max_depth) = self.max_depth {
                if i == max_depth {
                    RUN_SEARCH.store(false, Ordering::Relaxed);
                    break;
                }
            }
            if i > pv.len + 10 && pv.len != PV_MAX_LEN {
                RUN_SEARCH.store(false, Ordering::Relaxed);
                break;
            }

            last_pv = pv;
        }
        (score, last_pv)
    }

    fn negamax(
        &mut self,
        mut alpha: i32,
        beta: i32,
        depth: i32,
        ply: usize,
        last_move: Move,
        pv: &mut PrincipalVariation,
    ) -> i32 {
        // terminate search early
        if !RUN_SEARCH.load(Ordering::Relaxed) && depth > 1 {
            return 0;
        }

        // check extension before quiescence
        let depth = if self.game.in_check(self.game.current_player()) {
            depth + 1
        } else {
            depth
        };

        // quiescence search at full depth
        if depth == 0 {
            // exact score so we must reset the pv
            pv.len = 0;
            let score = self.quiesce(alpha, beta, 0, last_move, EVAL_PARAMS);
            // self.transposition_table
            //     .set(self.hash, Move::null(), depth as i8, score, Exact);
            return score;
        }

        NODE_COUNT.fetch_add(1, Ordering::Relaxed);
        NPS_COUNT.fetch_add(1, Ordering::Relaxed);

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

        let mut line = PrincipalVariation::new();

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
                return tt_entry.score;
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
        // don't search the null move when in check or only down to pawn/kings
        if depth >= 3
            && !self.game.in_check(self.game.current_player())
            && self.game.has_non_pawn_material(self.game.current_player())
        {
            self.game.make_null_move();
            let null_score = -self.negamax(
                -beta,
                -beta + 1,
                depth - 3,
                ply + 1,
                Move::null(),
                &mut line,
            );
            self.game.unmake_null_move();

            if null_score >= beta {
                return null_score;
            }
        }

        let mut moves = self.game.legal_moves();

        if moves.is_empty() {
            // exact score, so we must reset the pv
            pv.len = 0;
            if self.game.in_check(self.game.current_player()) {
                // checkmate, preferring shorter mating sequences
                return -(CHECKMATE_SCORE - ply as i32);
            } else {
                // stalemate
                return DRAW_SCORE;
            }
        }

        moves.iter_mut().for_each(|mut m| {
            // try the transposition table move early
            if m.start() == tt_move.start() && m.target() == tt_move.target() {
                m.score += 100_000;
            } else if m.capture() {
                // winning captures first, then equal, then quiets, then losing
                let see = self.game.see(*m);
                if see < 0 {
                    m.score -= 2000 - see;
                } else {
                    m.score += 2000 + see;
                }
            }
            // order queen and rook promotions ahead of quiet moves
            else if m.promotion() == Queen || m.promotion() == Rook {
                m.score += EVAL_PARAMS.piece_values[(Midgame, m.promotion())] + 100;
            }
            // quiet killer moves get sorted before other quiet moves
            else if self.killer_moves[ply].contains(&m) {
                m.score += 500;
            // quiet moves get ordered by their history heuristic
            } else {
                m.score += self.history_tables[self.game.current_player()][m.piece()]
                    [*m.target() as usize];
            }
        });
        // make sure the reported best move is at least legal
        let mut best_move = *moves.first().unwrap();

        for i in 0..moves.len() {
            pick_move(&mut moves, i);
            let move_ = moves[i];

            // SEE pruning
            if depth < 6 && ply != 0 && i > 0 && move_.promotion() == NoPiece {
                let see = self.game.see(move_);
                let depth_margin = depth * if move_.capture() { 100 } else { 50 };
                if see <= -depth * depth_margin {
                    continue;
                }
            }

            self.game.make_move(move_);
            // Principal Variation Search: search the first move at full width
            let score = if i == 0 {
                -self.negamax(-beta, -alpha, depth - 1, ply + 1, move_, &mut line)
            } else {
                // search remaining moves with a null window
                let mut score =
                    -self.negamax(-alpha - 1, -alpha, depth - 1, ply + 1, move_, &mut line);

                // if a null window search improves alpha, search again with a full window
                if score > alpha && score < beta {
                    score = -self.negamax(-beta, -alpha, depth - 1, ply + 1, move_, &mut line);
                }
                score
            };
            self.game.unmake_move();
            if score >= beta {
                self.transposition_table.set(
                    self.game.hash(),
                    move_,
                    depth as i8,
                    beta,
                    LowerBound,
                );
                if !move_.capture() {
                    self.history_tables[self.game.current_player()][move_.piece()]
                        [*move_.target() as usize] += depth * depth;
                    if move_.promotion() == NoPiece {
                        self.killer_moves.push(move_, ply);
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
        self.transposition_table
            .set(self.game.hash(), best_move, depth as i8, alpha, UpperBound);
        alpha
    }

    pub fn quiesce(
        &mut self,
        alpha: i32,
        beta: i32,
        depth: i32,
        last_move: Move,
        eval_params: EvalParams,
    ) -> i32 {
        self._quiesce::<()>(alpha, beta, depth, last_move, eval_params)
            .0
    }

    pub fn _quiesce<T: TraceTarget + Default>(
        &mut self,
        mut alpha: i32,
        beta: i32,
        depth: i32,
        _last_move: Move,
        eval_params: EvalParams,
    ) -> (i32, T) {
        NODE_COUNT.fetch_add(1, Ordering::Relaxed);
        NPS_COUNT.fetch_add(1, Ordering::Relaxed);

        let (stand_pat_score, mut best_trace) = self.game.evaluate::<T>();

        if stand_pat_score >= beta {
            return (beta, best_trace);
        }
        alpha = alpha.max(stand_pat_score);

        // transposition table lookup
        let mut tt_move = Move::null();
        if !T::TRACING {
            if let Some(tt_entry) = self.transposition_table.get(self.game.hash()) {
                if tt_entry.depth as i32 >= depth
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
        let mut moves: Vec<Move> = self
            .game
            .legal_moves()
            .into_iter()
            .filter(|m| m.capture())
            .map(|mut m| {
                // try the transposition table move early
                if m.start() == tt_move.start() && m.target() == tt_move.target() {
                    m.score += 10_000;
                }

                let see = self.game.see(m);
                if see < 0 {
                    m.score -= 2000 - see
                } else {
                    m.score += 2000 + see;
                }
                m
            })
            .filter(|m| m.score > 0)
            .collect();

        let mut best_move = Move::null();
        for i in 0..moves.len() {
            pick_move(&mut moves, i);
            let move_ = moves[i];

            self.game.make_move(move_);
            let (mut score, trace) =
                self._quiesce::<T>(-beta, -alpha, depth - 1, move_, eval_params);
            score = -score;
            self.game.unmake_move();
            if score >= beta {
                if !T::TRACING {
                    self.transposition_table.set(
                        self.game.hash(),
                        move_,
                        depth as i8,
                        beta,
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
                UpperBound,
            );
        }
        (alpha, best_trace)
    }
}
