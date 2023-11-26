
// pub struct Search {
//     pub game: Board,
//     pub search_history: Vec<u64>,
//     pub pre_history: Vec<u64>,
//     pub search_stack: Vec<SearchStackEntry>,
//     pub seldepth: usize,
//     transposition_table: Arc<RwLock<TranspositionTable>>,
//     pawn_hash_table: PawnHashTable,
//     pub history_tables: [HistoryTable; 2],
//     pub countermove_tables: [[[Move; 64]; 6]; 2],
//     pub max_depth: Option<usize>,
//     pub max_nodes: Option<usize>,
//     pub max_time_ms: Option<(usize, usize)>,
//     pub abort_time_ms: Option<usize>,
//     start_time: Instant,
//     output: bool,
//     chess_960: bool,
//     options: SearchOptions,
//     pub local_nodes: usize,
// }
// #[derive(Clone)]
// pub struct SearchStackEntry {
//     pub eval: i16,
//     pub move_list: MoveList,
//     pub killer_moves: KillerMoves<NUM_KILLER_MOVES>,
// }

use crate::{search::{SearchStackEntry, SEARCH_MAX_PLY}, history_tables::HistoryTable};

pub struct ThreadInfo {
    pub search_stack: Box<[SearchStackEntry; SEARCH_MAX_PLY]>,
    pub history_tables: Box<[HistoryTable; 2]>,

}
