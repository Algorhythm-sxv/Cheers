#[cfg(feature = "eval-tracing")]
use bytemuck::{Pod, Zeroable};

use super::eval_types::{PieceTables, PieceValues};

#[cfg_attr(feature = "eval-tracing", derive(Pod, Zeroable))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct EvalParams {
    pub piece_values: PieceValues,

    pub knight_mobility: [[i16; 2]; 9],
    pub knight_behind_pawn: [i16; 2],
    pub knight_king_distance: [[i16; 2]; 4],
    pub knight_outpost: [[i16; 2]; 2],

    pub bishop_mobility: [[i16; 2]; 14],
    pub bishop_behind_pawn: [i16; 2],
    pub bishop_king_distance: [[i16; 2]; 4],
    pub bishop_outpost: [[i16; 2]; 2],
    pub bishop_pair: [i16; 2],
    pub bishop_long_diagonal: [i16; 2],

    pub rook_mobility: [[i16; 2]; 15],
    pub rook_open_file: [[i16; 2]; 2],
    pub rook_on_seventh: [i16; 2],
    pub rook_trapped: [[i16; 2]; 2],

    pub queen_mobility: [[i16; 2]; 28],
    pub queen_discovery_risk: [i16; 2],

    pub king_mobility: [[i16; 2]; 9],
    pub king_defenders: [[i16; 2]; 12],
    pub king_open_file: [[i16; 2]; 2],
    pub no_enemy_queen: [i16; 2],

    // passed pawn terms
    pub passed_pawn: [[i16; 2]; 8],
    pub passed_pawn_advanced: [[i16; 2]; 6],
    pub passed_pawn_unblocked: [i16; 2],
    pub passed_pawn_connected: [i16; 2],
    pub passed_pawn_friendly_rook: [i16; 2],
    pub passed_pawn_enemy_king_too_far: [i16; 2],

    pub double_pawn: [[i16; 2]; 8],
    pub isolated_pawn: [[i16; 2]; 8],
    pub connected_pawn: [[i16; 2]; 8],

    pub tempo: [i16; 2],

    pub piece_tables: PieceTables,
}

#[cfg(feature = "eval-tracing")]
impl EvalParams {
    pub const LEN: usize = std::mem::size_of::<Self>() / std::mem::size_of::<i16>();
    pub fn to_array(&self) -> [i16; Self::LEN] {
        bytemuck::cast::<EvalParams, [i16; Self::LEN]>(*self)
    }
    pub fn as_array(&self) -> &[i16; Self::LEN] {
        bytemuck::cast_ref::<EvalParams, [i16; Self::LEN]>(self)
    }
    pub fn from_array(params: [i16; Self::LEN]) -> Self {
        bytemuck::cast::<[i16; Self::LEN], EvalParams>(params)
    }
}

#[cfg_attr(feature = "eval-tracing", derive(Pod, Zeroable))]
#[derive(Clone, Copy)]
#[repr(C)]
pub struct EvalTrace {
    pub pawn_count: [i16; 2],
    pub knight_count: [i16; 2],
    pub bishop_count: [i16; 2],
    pub rook_count: [i16; 2],
    pub queen_count: [i16; 2],
    // pads to the length of PieceValues
    pub king_count: [i16; 2],

    pub knight_mobility: [[i16; 2]; 9],
    pub knights_behind_pawns: [i16; 2],
    pub knight_king_distance: [[i16; 2]; 4],
    pub knight_outposts: [[i16; 2]; 2],

    pub bishop_mobility: [[i16; 2]; 14],
    pub bishops_behind_pawns: [i16; 2],
    pub bishop_king_distance: [[i16; 2]; 4],
    pub bishop_outposts: [[i16; 2]; 2],
    pub bishop_pair: [i16; 2],
    pub bishop_long_diagonals: [i16; 2],

    pub rook_mobility: [[i16; 2]; 15],
    pub rook_open_files: [[i16; 2]; 2],
    pub rooks_on_seventh: [i16; 2],
    pub rook_trapped: [[i16; 2]; 2],

    pub queen_mobility: [[i16; 2]; 28],
    pub queen_discovery_risks: [i16; 2],

    pub king_mobility: [[i16; 2]; 9],
    pub king_defenders: [[i16; 2]; 12],
    pub king_open_file: [[i16; 2]; 2],
    pub no_enemy_queen: [i16; 2],

    pub passed_pawn: [[i16; 2]; 8],
    pub passed_pawn_advanced: [[i16; 2]; 6],
    pub passed_pawn_unblocked: [i16; 2],
    pub passed_pawn_connected: [i16; 2],
    pub passed_pawn_friendly_rook: [i16; 2],
    pub passed_pawn_enemy_king_too_far: [i16; 2],

    pub double_pawn: [[i16; 2]; 8],
    pub isolated_pawn: [[i16; 2]; 8],
    pub connected_pawn: [[i16; 2]; 8],

    pub tempo: [i16; 2],

    pub pawn_placement: [[i16; 2]; 64],
    pub knight_placement: [[i16; 2]; 64],
    pub bishop_placement: [[i16; 2]; 64],
    pub rook_placement: [[i16; 2]; 64],
    pub queen_placement: [[i16; 2]; 64],
    pub king_placement: [[i16; 2]; 64],

    pub turn: i16,
}

#[cfg(feature = "eval-tracing")]
impl EvalTrace {
    pub const LEN: usize = std::mem::size_of::<Self>() / std::mem::size_of::<i16>();
    pub fn new() -> Self {
        bytemuck::cast::<[i16; Self::LEN], Self>([0i16; Self::LEN])
    }
    pub fn to_array(&self) -> [i16; Self::LEN] {
        bytemuck::cast::<Self, [i16; Self::LEN]>(*self)
    }
}

#[cfg(feature = "eval-tracing")]
impl Default for EvalTrace {
    fn default() -> Self {
        Self::new()
    }
}

pub const CHECKMATE_SCORE: i16 = 30000;
pub const DRAW_SCORE: i16 = 0;

pub const EVAL_PARAMS: EvalParams = EvalParams {
    piece_values: PieceValues([
        [81, 72],
        [376, 278],
        [402, 280],
        [613, 440],
        [791, 806],
        [20000, 20000],
    ]),
    knight_mobility: [
        [-54, -124],
        [-6, -64],
        [17, -29],
        [22, -11],
        [41, -17],
        [47, -5],
        [56, -7],
        [68, -6],
        [80, -17],
    ],
    knight_behind_pawn: [8, 11],
    knight_king_distance: [[-15, 6], [-12, 0], [-23, 2], [-49, 7]],
    knight_outpost: [[13, -13], [32, 8]],
    bishop_mobility: [
        [0, -78],
        [-6, -52],
        [13, -46],
        [31, -29],
        [42, -21],
        [52, -12],
        [59, -6],
        [64, -5],
        [71, 0],
        [76, -4],
        [86, -3],
        [101, -15],
        [102, -3],
        [113, -24],
    ],
    bishop_behind_pawn: [9, 6],
    bishop_king_distance: [[-6, 0], [-6, -2], [-9, -2], [-39, 12]],
    bishop_outpost: [[8, -5], [43, -8]],
    bishop_pair: [-2, 37],
    bishop_long_diagonal: [34, -13],
    rook_mobility: [
        [-31, -213],
        [-72, -45],
        [-23, -7],
        [-16, 11],
        [-17, 21],
        [-12, 17],
        [-9, 24],
        [-2, 24],
        [0, 29],
        [9, 27],
        [15, 31],
        [20, 33],
        [30, 36],
        [32, 36],
        [68, 18],
    ],
    rook_open_file: [[14, 11], [40, -3]],
    rook_on_seventh: [9, -21],
    rook_trapped: [[-31, 16], [-2, 25]],
    queen_mobility: [
        [-40, -1],
        [-202, -81],
        [-111, -155],
        [-44, -117],
        [-35, -22],
        [-25, -53],
        [-19, 4],
        [-13, -8],
        [-13, 43],
        [-2, -4],
        [-8, 68],
        [-1, 48],
        [4, 65],
        [8, 58],
        [15, 54],
        [13, 73],
        [8, 83],
        [19, 78],
        [11, 88],
        [7, 98],
        [24, 88],
        [37, 87],
        [22, 90],
        [93, 41],
        [123, 31],
        [89, 46],
        [95, 28],
        [94, 29],
    ],
    queen_discovery_risk: [-25, 46],
    king_mobility: [
        [-77, 108],
        [-59, 11],
        [-44, 39],
        [-12, 3],
        [-14, 16],
        [8, -15],
        [40, -8],
        [44, -5],
        [64, -32],
    ],
    king_defenders: [
        [-64, -3],
        [-27, 4],
        [-6, 2],
        [9, 0],
        [18, 2],
        [28, -2],
        [24, -22],
        [28, -7],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
    ],
    king_open_file: [[-20, 12], [-50, -1]],
    no_enemy_queen: [497, 16],
    passed_pawn: [
        [8, -9],
        [5, 4],
        [-9, 4],
        [-21, 5],
        [-11, 3],
        [-7, 9],
        [9, 13],
        [8, 2],
    ],
    passed_pawn_advanced: [[-19, -16], [-8, -15], [-10, 0], [5, 5], [28, 22], [57, -40]],
    passed_pawn_unblocked: [8, 20],
    passed_pawn_connected: [17, -3],
    passed_pawn_friendly_rook: [17, 13],
    passed_pawn_enemy_king_too_far: [-47, 61],
    double_pawn: [
        [-21, -23],
        [17, -26],
        [-18, -16],
        [-11, -23],
        [-13, -17],
        [-8, -10],
        [3, -16],
        [1, -30],
    ],
    isolated_pawn: [
        [-10, 0],
        [-11, -3],
        [-8, -7],
        [-19, -4],
        [-16, -6],
        [-6, -5],
        [-6, -8],
        [-18, 4],
    ],
    connected_pawn: [
        [0, 8],
        [9, 11],
        [11, 13],
        [7, 16],
        [10, 16],
        [5, 8],
        [18, 10],
        [13, 11],
    ],
    tempo: [13, 13],
    piece_tables: PieceTables([
        [
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [-6, 17],
            [4, 14],
            [1, 21],
            [8, 16],
            [7, 22],
            [33, 7],
            [38, 1],
            [7, -6],
            [-8, 6],
            [-16, 6],
            [0, 4],
            [-4, 9],
            [1, 10],
            [-8, 5],
            [0, 0],
            [-8, -7],
            [-12, 12],
            [-17, 12],
            [-2, 1],
            [16, -3],
            [7, 1],
            [14, -6],
            [-19, 6],
            [-28, -1],
            [1, 29],
            [-8, 24],
            [-3, 19],
            [12, 8],
            [11, 8],
            [12, 8],
            [-13, 20],
            [-9, 11],
            [-7, 73],
            [-4, 66],
            [1, 64],
            [4, 45],
            [0, 44],
            [52, 34],
            [-15, 53],
            [-14, 52],
            [16, 190],
            [22, 173],
            [-4, 170],
            [30, 136],
            [47, 129],
            [45, 131],
            [-21, 171],
            [-82, 170],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
            [0, 0],
        ],
        [
            [1, -40],
            [23, -33],
            [-7, -26],
            [-1, -17],
            [6, -24],
            [-4, -19],
            [13, -31],
            [-42, -15],
            [36, -46],
            [9, -30],
            [25, -35],
            [28, -23],
            [28, -25],
            [16, -27],
            [19, -36],
            [16, -38],
            [25, -34],
            [18, -18],
            [24, -23],
            [7, 0],
            [24, -6],
            [15, -19],
            [35, -36],
            [7, -37],
            [41, -25],
            [25, -18],
            [30, -3],
            [22, 0],
            [32, 3],
            [42, -14],
            [28, -26],
            [18, -17],
            [57, -24],
            [45, -15],
            [28, -3],
            [71, 1],
            [46, -5],
            [59, -5],
            [40, -16],
            [72, -34],
            [28, -35],
            [23, -27],
            [20, 0],
            [58, -4],
            [87, -26],
            [124, -29],
            [63, -36],
            [20, -26],
            [-27, -26],
            [0, -15],
            [98, -41],
            [50, -23],
            [107, -45],
            [91, -45],
            [47, -36],
            [43, -50],
            [-168, 4],
            [14, -64],
            [-3, -26],
            [-13, -19],
            [71, -36],
            [-61, -28],
            [-32, -45],
            [-65, -64],
        ],
        [
            [34, -15],
            [32, -12],
            [46, -26],
            [28, -9],
            [26, -10],
            [5, -7],
            [39, -31],
            [11, -8],
            [39, -13],
            [23, -10],
            [45, -16],
            [35, -13],
            [35, -7],
            [47, -19],
            [29, -14],
            [34, -22],
            [63, -24],
            [56, -13],
            [12, 3],
            [29, 0],
            [40, -2],
            [1, 2],
            [32, -26],
            [46, -23],
            [37, -16],
            [25, -11],
            [39, -1],
            [7, 7],
            [16, 8],
            [30, -2],
            [33, -20],
            [20, -14],
            [4, 2],
            [34, 0],
            [28, -3],
            [2, 17],
            [5, 13],
            [14, -4],
            [41, -12],
            [23, -10],
            [15, -10],
            [12, -4],
            [23, 4],
            [10, 6],
            [68, -11],
            [20, 12],
            [53, -10],
            [54, -12],
            [-6, -9],
            [-16, 3],
            [1, -8],
            [11, -18],
            [25, -13],
            [64, -22],
            [-18, 8],
            [42, -34],
            [-19, 0],
            [-11, -23],
            [14, -18],
            [-18, -7],
            [-68, -3],
            [4, -15],
            [17, -18],
            [-51, 7],
        ],
        [
            [-1, 7],
            [0, 4],
            [8, 4],
            [13, 5],
            [22, -5],
            [21, 0],
            [2, -5],
            [29, -28],
            [-20, 0],
            [-15, 2],
            [-19, 8],
            [-2, 0],
            [0, -2],
            [10, -4],
            [37, -16],
            [-26, -3],
            [-25, 1],
            [-19, 2],
            [-14, 3],
            [2, -1],
            [2, -1],
            [14, -9],
            [38, -22],
            [19, -17],
            [-27, 10],
            [-39, 13],
            [-36, 16],
            [-26, 16],
            [-20, 13],
            [-6, 4],
            [17, -6],
            [6, -10],
            [-32, 16],
            [-5, 6],
            [-2, 14],
            [11, 6],
            [-8, 10],
            [16, 4],
            [48, -7],
            [26, -2],
            [-1, 11],
            [-7, 13],
            [-10, 13],
            [25, 5],
            [35, -1],
            [58, -6],
            [88, -12],
            [63, -12],
            [-26, 41],
            [-41, 55],
            [-8, 44],
            [24, 32],
            [-7, 34],
            [70, 15],
            [82, 13],
            [44, 15],
            [6, 22],
            [28, 11],
            [26, 14],
            [31, 7],
            [68, -2],
            [113, -17],
            [95, -13],
            [22, 5],
        ],
        [
            [23, -30],
            [11, -22],
            [15, -22],
            [22, -22],
            [14, -9],
            [7, -25],
            [-4, -23],
            [49, -62],
            [10, -11],
            [12, -12],
            [13, -16],
            [21, -21],
            [22, -17],
            [35, -44],
            [41, -50],
            [27, -12],
            [3, -37],
            [11, -38],
            [-3, 22],
            [4, 4],
            [0, 24],
            [8, 17],
            [8, 32],
            [10, 2],
            [5, 1],
            [-9, 17],
            [-7, 21],
            [-18, 54],
            [-3, 41],
            [12, -2],
            [12, 29],
            [7, 15],
            [-17, -8],
            [-13, 26],
            [-24, 29],
            [-24, 62],
            [-20, 78],
            [-3, 70],
            [-6, 56],
            [25, 24],
            [-18, -13],
            [-25, 11],
            [0, 34],
            [3, 36],
            [30, 73],
            [58, 27],
            [105, -15],
            [71, -5],
            [-40, 30],
            [-45, 25],
            [-48, 48],
            [-78, 100],
            [-56, 79],
            [65, 63],
            [10, 61],
            [133, -63],
            [-82, 85],
            [9, 4],
            [70, -27],
            [64, 0],
            [48, 21],
            [-4, 66],
            [5, 1],
            [-13, 43],
        ],
        [
            [-23, -54],
            [15, -38],
            [-6, -21],
            [-43, -14],
            [-21, -28],
            [-38, -19],
            [26, -46],
            [52, -92],
            [32, -40],
            [-87, 5],
            [-101, 17],
            [-131, 30],
            [-122, 26],
            [-113, 23],
            [-55, 4],
            [21, -35],
            [7, -35],
            [-44, 1],
            [-116, 23],
            [-128, 31],
            [-137, 31],
            [-117, 23],
            [-84, 10],
            [-37, -19],
            [20, -34],
            [-24, 4],
            [-39, 17],
            [-108, 29],
            [-102, 29],
            [-126, 29],
            [-99, 17],
            [-84, -8],
            [4, -23],
            [-31, 14],
            [-4, 20],
            [-34, 25],
            [-67, 28],
            [-56, 31],
            [-40, 25],
            [-26, -1],
            [62, -18],
            [97, 16],
            [10, 25],
            [30, 21],
            [-32, 34],
            [42, 32],
            [46, 33],
            [2, 8],
            [47, -13],
            [86, 13],
            [31, 21],
            [20, 18],
            [17, 21],
            [67, 29],
            [-37, 39],
            [-24, 11],
            [154, -78],
            [105, -21],
            [119, -18],
            [76, -8],
            [104, -16],
            [126, -12],
            [89, -14],
            [80, -55],
        ],
    ]),
};
