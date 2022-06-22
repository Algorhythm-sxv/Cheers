use bytemuck::{Pod, Zeroable};

use super::eval_types::{PieceTables, PieceValues};

#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)]
pub struct EvalParams {
    pub piece_values: PieceValues,

    pub knight_mobility: [[i32; 2]; 9],
    pub knight_behind_pawn: [i32; 2],
    pub knight_king_distance: [[i32; 2]; 4],
    pub knight_outpost: [[i32; 2]; 2],

    pub bishop_mobility: [[i32; 2]; 14],
    pub bishop_behind_pawn: [i32; 2],
    pub bishop_king_distance: [[i32; 2]; 4],
    pub bishop_outpost: [[i32; 2]; 2],
    pub bishop_pair: [i32; 2],
    pub bishop_long_diagonal: [i32; 2],

    pub rook_mobility: [[i32; 2]; 15],
    pub rook_open_file: [[i32; 2]; 2],
    pub rook_on_seventh: [i32; 2],

    pub queen_mobility: [[i32; 2]; 28],
    pub queen_discovery_risk: [i32; 2],

    pub king_defenders: [[i32; 2]; 9],

    pub passed_pawn: [i32; 2],
    pub double_pawn: [i32; 2],
    pub isolated_pawn: [[i32; 2]; 8],
    // pub backward_pawn: [[i32; 2]; 8],
    pub piece_tables: PieceTables,
}

impl EvalParams {
    pub const LEN: usize = std::mem::size_of::<Self>() / std::mem::size_of::<i32>();
    pub fn to_array(&self) -> [i32; Self::LEN] {
        bytemuck::cast::<EvalParams, [i32; Self::LEN]>(*self)
    }
    pub fn as_array(&self) -> &[i32; Self::LEN] {
        bytemuck::cast_ref::<EvalParams, [i32; Self::LEN]>(self)
    }
    pub fn from_array(params: [i32; Self::LEN]) -> Self {
        bytemuck::cast::<[i32; Self::LEN], EvalParams>(params)
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct EvalTrace {
    pub pawn_count: [i32; 2],
    pub knight_count: [i32; 2],
    pub bishop_count: [i32; 2],
    pub rook_count: [i32; 2],
    pub queen_count: [i32; 2],
    // pads to the length of PieceValues
    pub king_count: [i32; 2],

    pub knight_mobility: [[i32; 2]; 9],
    pub knights_behind_pawns: [i32; 2],
    pub knight_king_distance: [[i32; 2]; 4],
    pub knight_outposts: [[i32; 2]; 2],

    pub bishop_mobility: [[i32; 2]; 14],
    pub bishops_behind_pawns: [i32; 2],
    pub bishop_king_distance: [[i32; 2]; 4],
    pub bishop_outposts: [[i32; 2]; 2],
    pub bishop_pair: [i32; 2],
    pub bishop_long_diagonals: [i32; 2],

    pub rook_mobility: [[i32; 2]; 15],
    pub rook_open_files: [[i32; 2]; 2],
    pub rooks_on_seventh: [i32; 2],

    pub queen_mobility: [[i32; 2]; 28],
    pub queen_discovery_risks: [i32; 2],

    pub king_defenders: [[i32; 2]; 9],

    pub passed_pawns: [i32; 2],
    pub double_pawns: [i32; 2],
    pub isolated_pawns: [[i32; 2]; 8],
    // pub backward_pawns: [[i32; 2]; 8],
    pub pawn_placement: [[i32; 2]; 64],
    pub knight_placement: [[i32; 2]; 64],
    pub bishop_placement: [[i32; 2]; 64],
    pub rook_placement: [[i32; 2]; 64],
    pub queen_placement: [[i32; 2]; 64],
    pub king_placement: [[i32; 2]; 64],

    pub turn: i32,
}

impl EvalTrace {
    pub const LEN: usize = std::mem::size_of::<Self>() / std::mem::size_of::<i32>();
    pub fn new() -> Self {
        bytemuck::cast::<[i32; Self::LEN], Self>([0i32; Self::LEN])
    }
    pub fn to_array(&self) -> [i32; Self::LEN] {
        bytemuck::cast::<Self, [i32; Self::LEN]>(*self)
    }
}

impl Default for EvalTrace {
    fn default() -> Self {
        Self::new()
    }
}

pub const CHECKMATE_SCORE: i32 = 20000;
pub const DRAW_SCORE: i32 = 0;

pub const EVAL_PARAMS: EvalParams = EvalParams {
    piece_values: PieceValues([
        [54, 133],
        [241, 366],
        [269, 342],
        [348, 591],
        [762, 1019],
        [20000, 20000],
    ]),
    knight_mobility: [
        [-135, -144],
        [-72, -66],
        [-46, -25],
        [-41, 12],
        [-27, 21],
        [-20, 43],
        [-9, 51],
        [1, 57],
        [13, 42],
    ],
    knight_behind_pawn: [8, 12],
    knight_king_distance: [[-15, -2], [-10, -16], [-22, -21], [-32, -46]],
    knight_outpost: [[3, 6], [0, 21]],
    bishop_mobility: [
        [-95, -106],
        [-62, -75],
        [-47, -46],
        [-42, -5],
        [-32, 4],
        [-21, 18],
        [-12, 36],
        [-6, 37],
        [-5, 47],
        [0, 45],
        [3, 51],
        [31, 24],
        [60, 24],
        [32, 27],
    ],
    bishop_behind_pawn: [13, 21],
    bishop_king_distance: [[-5, 0], [-13, -10], [-13, -14], [-62, 10]],
    bishop_outpost: [[-7, 7], [0, 3]],
    bishop_pair: [-6, 85],
    bishop_long_diagonal: [0, -6],
    rook_mobility: [
        [-262, -178],
        [-105, 24],
        [-70, -1],
        [-70, 21],
        [-66, 30],
        [-64, 33],
        [-60, 43],
        [-56, 48],
        [-50, 53],
        [-49, 62],
        [-45, 63],
        [-39, 67],
        [-46, 70],
        [-42, 66],
        [-72, 87],
    ],
    rook_open_file: [[10, 15], [29, 12]],
    rook_on_seventh: [-19, 1],
    queen_mobility: [
        [-46, -6],
        [-137, -48],
        [10, -115],
        [-59, 53],
        [-38, -108],
        [-43, -31],
        [-39, 7],
        [-41, 7],
        [-39, 19],
        [-37, 34],
        [-34, 50],
        [-28, 52],
        [-25, 63],
        [-22, 72],
        [-16, 68],
        [-18, 77],
        [-8, 73],
        [-3, 59],
        [-23, 80],
        [-10, 70],
        [5, 46],
        [42, 8],
        [30, 27],
        [65, -7],
        [113, -36],
        [-79, 114],
        [30, -39],
        [-18, -93],
    ],
    queen_discovery_risk: [-11, 3],
    king_defenders: [[0; 2]; 9],
    passed_pawn: [-19, 15],
    double_pawn: [-42, -95],
    isolated_pawn: [
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
        [0, 0],
    ],
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
            [-17, 31],
            [-10, 26],
            [-17, 18],
            [1, 22],
            [5, 42],
            [18, 31],
            [23, 22],
            [0, 1],
            [-31, 24],
            [-14, 31],
            [-31, 14],
            [-12, 18],
            [3, 28],
            [-6, 26],
            [14, 16],
            [1, -4],
            [-28, 37],
            [-2, 30],
            [-10, 4],
            [-10, 8],
            [-2, 14],
            [0, 13],
            [3, 25],
            [3, 10],
            [-20, 67],
            [-8, 51],
            [-13, 33],
            [-2, 22],
            [16, 25],
            [25, 28],
            [8, 50],
            [8, 39],
            [-12, 125],
            [5, 115],
            [59, 86],
            [48, 69],
            [40, 86],
            [48, 67],
            [38, 77],
            [31, 91],
            [0, 226],
            [110, 177],
            [105, 177],
            [115, 132],
            [83, 155],
            [32, 190],
            [5, 177],
            [-73, 227],
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
            [-60, 17],
            [-18, -67],
            [-52, -2],
            [-48, -7],
            [-58, -14],
            [-57, -15],
            [-23, -17],
            [-77, -14],
            [-43, 12],
            [-53, 26],
            [-24, -37],
            [-23, -28],
            [-18, -31],
            [-19, -12],
            [-54, 0],
            [-38, -9],
            [-1, -33],
            [-29, -27],
            [-35, -29],
            [-29, -9],
            [-25, 0],
            [-28, -45],
            [-14, -31],
            [-10, -25],
            [-4, 12],
            [-11, -17],
            [1, 1],
            [-22, 13],
            [-16, 12],
            [4, -20],
            [34, -22],
            [-6, -13],
            [35, 10],
            [3, -10],
            [5, 16],
            [50, -15],
            [7, 1],
            [20, 13],
            [19, -5],
            [39, 13],
            [-28, 2],
            [0, -6],
            [38, -5],
            [51, 0],
            [69, -17],
            [158, -40],
            [66, -14],
            [48, -13],
            [46, -13],
            [23, 10],
            [64, -6],
            [109, -33],
            [118, -13],
            [79, -17],
            [69, -13],
            [67, -29],
            [26, -7],
            [-18, 28],
            [19, 21],
            [18, 41],
            [34, 3],
            [-116, 55],
            [-87, -8],
            [44, -28],
        ],
        [
            [11, 14],
            [21, -14],
            [-8, 0],
            [-28, -16],
            [-40, -8],
            [-19, -5],
            [5, -14],
            [-23, 18],
            [1, -32],
            [-1, -10],
            [4, -26],
            [-28, -2],
            [-25, 6],
            [5, -25],
            [-5, -3],
            [6, 0],
            [-1, -1],
            [7, -9],
            [-18, 10],
            [-12, 3],
            [-12, 13],
            [-25, -5],
            [-10, -21],
            [13, -32],
            [9, 2],
            [-2, -6],
            [-11, -2],
            [9, 17],
            [0, 6],
            [-12, -9],
            [-36, -6],
            [19, -16],
            [11, 5],
            [10, 7],
            [1, -2],
            [37, 7],
            [26, -4],
            [23, -3],
            [0, 0],
            [-41, 43],
            [34, 10],
            [40, 0],
            [34, 19],
            [23, 3],
            [61, -32],
            [103, 11],
            [65, -13],
            [31, 13],
            [-3, 15],
            [14, 9],
            [8, -1],
            [-7, 4],
            [39, -22],
            [59, -22],
            [0, -1],
            [44, 25],
            [96, 43],
            [-28, 59],
            [7, 20],
            [-108, 44],
            [38, 9],
            [30, 21],
            [4, -20],
            [55, 23],
        ],
        [
            [-42, 18],
            [-39, 3],
            [-39, 11],
            [-29, 5],
            [-24, 0],
            [-25, 6],
            [-18, 0],
            [-35, 0],
            [-57, 15],
            [-65, -8],
            [-39, 3],
            [-40, -12],
            [-27, -8],
            [-29, 12],
            [-30, -12],
            [-55, 12],
            [-39, 12],
            [-51, 8],
            [-58, 13],
            [-62, 12],
            [-7, -28],
            [-30, -14],
            [4, -11],
            [-47, 5],
            [-55, 31],
            [-40, 17],
            [-55, 31],
            [-48, 26],
            [-37, 15],
            [-41, 18],
            [-6, -5],
            [-20, 8],
            [-54, 49],
            [-21, 34],
            [-7, 32],
            [3, 20],
            [-5, 23],
            [20, 3],
            [31, 7],
            [-25, 36],
            [-53, 39],
            [-7, 37],
            [11, 42],
            [16, 19],
            [36, 11],
            [49, 20],
            [86, -1],
            [0, 35],
            [-26, 47],
            [-20, 57],
            [-1, 58],
            [32, 45],
            [28, 33],
            [54, 23],
            [54, 35],
            [-3, 34],
            [-12, 28],
            [-21, 34],
            [-53, 49],
            [7, 48],
            [-17, 33],
            [-13, 40],
            [0, 57],
            [33, 30],
        ],
        [
            [-31, 15],
            [-35, -11],
            [-42, 15],
            [-25, 13],
            [-13, -19],
            [-40, -38],
            [63, -141],
            [58, -140],
            [-64, 73],
            [-28, -4],
            [-32, 18],
            [-18, 2],
            [-21, -2],
            [0, -23],
            [-20, -53],
            [4, -41],
            [-25, 17],
            [-30, 36],
            [-31, 44],
            [-35, 27],
            [-41, 53],
            [-23, 38],
            [-7, 3],
            [10, -31],
            [-10, -17],
            [-37, 23],
            [-40, 63],
            [-43, 113],
            [-31, 66],
            [-23, 59],
            [1, 13],
            [0, 37],
            [-1, -29],
            [-37, 36],
            [-1, 41],
            [-41, 104],
            [-8, 84],
            [-1, 101],
            [4, 69],
            [0, 68],
            [-34, 18],
            [-15, 19],
            [7, 40],
            [-20, 48],
            [39, 47],
            [85, 52],
            [84, 40],
            [38, 67],
            [-13, 29],
            [-27, 27],
            [-16, 52],
            [-11, 63],
            [-22, 62],
            [52, 29],
            [-17, 99],
            [61, 25],
            [9, -14],
            [13, -16],
            [52, 0],
            [99, -43],
            [80, 8],
            [106, -62],
            [256, -165],
            [86, -54],
        ],
        [
            [23, -87],
            [62, -57],
            [35, -49],
            [-44, -32],
            [16, -60],
            [-22, -44],
            [40, -67],
            [37, -103],
            [63, -28],
            [-1, -11],
            [13, -18],
            [-37, -7],
            [-45, -1],
            [-35, -15],
            [9, -32],
            [18, -62],
            [-22, 5],
            [-20, -3],
            [-25, 11],
            [-49, 17],
            [-57, 18],
            [-83, 11],
            [-41, -7],
            [-39, -44],
            [-39, -2],
            [0, 14],
            [-67, 42],
            [-85, 54],
            [-143, 55],
            [-101, 44],
            [-66, 14],
            [-43, -9],
            [-37, 24],
            [0, 28],
            [-61, 57],
            [-112, 72],
            [-107, 62],
            [-39, 51],
            [-146, 48],
            [-28, -1],
            [66, -9],
            [-39, 61],
            [190, 40],
            [29, 46],
            [-14, 61],
            [-27, 59],
            [-106, 52],
            [-54, 11],
            [149, -57],
            [-37, 51],
            [65, 60],
            [-44, 69],
            [-77, 60],
            [-193, 95],
            [-77, 82],
            [-112, 20],
            [65, -120],
            [103, -37],
            [104, -9],
            [-5, 20],
            [-85, 10],
            [-6, 31],
            [97, 6],
            [4, -76],
        ],
    ]),
};