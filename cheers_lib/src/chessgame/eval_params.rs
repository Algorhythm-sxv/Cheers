use bytemuck::{Pod, Zeroable};

use crate::types::PieceIndex;

#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)]
pub struct EvalParams {
    pub piece_values: PieceValues,
    pub knight_mobility: [[i32; 2]; 9],
    pub bishop_mobility: [[i32; 2]; 14],
    pub rook_mobility: [[i32; 2]; 15],
    pub queen_mobility: [[i32; 2]; 28],
    pub passed_pawn_bonus: [i32; 2],
    pub double_pawn_penalty: [i32; 2],
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
    pub bishop_mobility: [[i32; 2]; 14],
    pub rook_mobility: [[i32; 2]; 15],
    pub queen_mobility: [[i32; 2]; 28],

    pub passed_pawns: [i32; 2],
    pub double_pawns: [i32; 2],

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

pub const EVAL_PARAMS: EvalParams = EvalParams {
    piece_values: PIECE_VALUES,
    knight_mobility: KNIGHT_MOBILITY,
    bishop_mobility: BISHOP_MOBILITY,
    rook_mobility: ROOK_MOBILITY,
    queen_mobility: QUEEN_MOBILITY,
    passed_pawn_bonus: [PASSED_PAWN_BONUS; 2],
    double_pawn_penalty: [DOUBLE_PAWN_PENALTY; 2],
    piece_tables: PIECE_TABLES,
};

#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)]
pub struct PieceValues(pub [[i32; 2]; 6]);

impl std::ops::Index<(GamePhase, PieceIndex)> for PieceValues {
    type Output = i32;
    fn index(&self, index: (GamePhase, PieceIndex)) -> &Self::Output {
        &self.0[index.1 as usize][index.0 as usize]
    }
}

pub const PIECE_VALUES: PieceValues = PieceValues([
    [100, 100], // pawn value
    [320, 320], // knight value
    [350, 350], // bishop value
    [500, 500], // rook value
    [900, 900], // queen value
    [20000, 20000], // king value (for MVV-LVA)
]);

pub const CHECKMATE_SCORE: i32 = 20000;
pub const DRAW_SCORE: i32 = 0;
pub const PAWN_SHIELD_1: i32 = 0;
pub const PAWN_SHIELD_2: i32 = 0;
pub const PASSED_PAWN_BONUS: i32 = 0;
pub const DOUBLE_PAWN_PENALTY: i32 = 0;

pub const KNIGHT_MOBILITY: [[i32; 2]; 9] = [[0; 2]; 9];
pub const BISHOP_MOBILITY: [[i32; 2]; 14] = [[0; 2]; 14];
pub const ROOK_MOBILITY: [[i32; 2]; 15] = [[0; 2]; 15];
pub const QUEEN_MOBILITY: [[i32; 2]; 28] = [[0; 2]; 28];

#[rustfmt::skip]
mod tables {
    use super::PieceTables;

    pub const PAWN_TABLE: [[i32; 2]; 64] = [
        [  0,   0], [  0,   0], [  0,   0], [  0,   0], [  0,   0], [  0,   0], [  0,   0], [  0,   0],
        [-35,  13], [ -1,   8], [-20,   8], [-23,  10], [-15,  13], [ 24,   0], [ 38,   2], [-22,  -7],
        [-26,   4], [ -4,   7], [ -4,  -6], [-10,   1], [  3,   0], [  3,  -5], [ 33,  -1], [-12,  -8],
        [-27,  13], [ -2,   9], [ -5,  -3], [ 12,  -7], [ 17,  -7], [  6,  -8], [ 10,   3], [-25,  -1],
        [-14,  32], [ 13,  24], [  6,  13], [ 21,   5], [ 23,  -2], [ 12,   4], [ 17,  17], [-23,  17],
        [ -6,  94], [  7, 100], [ 26,  85], [ 31,  67], [ 65,  56], [ 56,  53], [ 25,  82], [-20,  84],
        [ 98, 178], [134, 173], [ 61, 158], [ 95, 134], [ 68, 147], [126, 165], [ 34, 187], [-11, 187],
        [  0,   0], [  0,   0], [  0,   0], [  0,   0], [  0,   0], [  0,   0], [  0,   0], [  0,   0],
    ];

        pub const KNIGHT_TABLE: [[i32; 2]; 64] = [
            [-105, -29], [-21, -51], [-58, -23], [-33, -15], [-17, -22], [-28, -18], [-19, -50], [-23, -64],
            [ -29, -42], [-53, -20], [-12, -10], [ -3,  -5], [ -1,  -2], [ 18, -20], [-14, -23], [-19, -44],
            [ -23, -23], [ -9,  -3], [ 12,  -1], [ 10,  15], [ 19,  10], [ 17,  -3], [ 25, -20], [-16, -22],
            [ -13, -18], [  4,  -6], [ 16,  16], [ 13,  25], [ 28,  16], [ 19,  17], [ 21,   4], [ -8, -18],
            [  -9, -17], [ 17,   3], [ 19,  22], [ 53,  22], [ 37,  22], [ 69,  11], [ 18,   8], [ 22, -18],
            [ -47, -24], [ 60, -20], [ 37,  10], [ 65,   9], [ 84,  -1], [129,  -9], [ 73, -19], [ 44, -41],
            [ -73, -25], [-41,  -8], [ 72, -25], [ 36,  -2], [ 23,  -9], [ 62, -25], [  7, -24], [-17, -52],
            [-167, -58], [-89, -38], [-34, -13], [-49, -28], [ 61, -31], [-97, -27], [-15, -63], [-107, 99],
        ];

            pub const BISHOP_TABLE: [[i32; 2]; 64] = [
                [-33, -23], [ -3,  -9], [-14, -23], [-21,  -5], [-13,  -9], [-12, -16], [-39,  -5], [-21, -17],
                [  4, -14], [ 15, -18], [ 16,  -7], [  0,  -1], [  7,   4], [ 21,  -9], [ 33, -15], [  1, -27],
                [  0, -12], [ 15,  -3], [ 15,   8], [ 15,  10], [ 14,  13], [ 27,   3], [ 18,  -7], [ 10, -15],
                [ -6,  -6], [ 13,   3], [ 13,  13], [ 26,  19], [ 34,   7], [ 12,  10], [ 10,  -3], [  4,  -9],
                [ -4,  -3], [  5,   9], [ 19,  12], [ 50,   9], [ 37,  14], [ 37,  10], [  7,   3], [ -2,  -2],
                [-16,   2], [ 37,  -8], [ 43,   0], [ 40,  -1], [ 35,  -2], [ 50,   6], [ 37,   0], [ -2,   4],
                [-26,  -8], [ 16,  -4], [-18,   7], [-13, -12], [ 30,  -3], [ 59, -13], [ 18,  -4], [-47, -14],
                [-29, -14], [  4, -21], [-82, -11], [-37,  -8], [-25,  -7], [-42,  -9], [  7, -17], [ -8, -24],
            ];

                pub const ROOK_TABLE: [[i32; 2]; 64] = [
                    [-19,  -9], [-13,   2], [  1,   3], [ 17,  -1], [ 16,  -5], [  7, -13], [-37,   4], [-26, -20],
                    [-44,  -6], [-16,  -6], [-20,   0], [ -9,   2], [ -1,  -9], [ 11,  -9], [ -6, -11], [-71,  -3],
                    [-45,  -4], [-25,   0], [-16,  -5], [-17,  -1], [  3,  -7], [  0, -12], [ -5,  -8], [-33, -16],
                    [-36,   3], [-26,   5], [-12,   8], [ -1,   4], [  9,  -5], [ -7,  -6], [  6,  -8], [-23, -11],
                    [-24,   4], [-11,   3], [  7,  13], [ 26,   1], [ 24,   2], [ 35,   1], [ -8,  -1], [-20,   2],
                    [ -5,   7], [ 19,   7], [ 26,   7], [ 36,   5], [ 17,   4], [ 45,  -3], [ 61,  -5], [ 16,  -3],
                    [ 27,  11], [ 32,  13], [ 58,  13], [ 62,  11], [ 80,  -3], [ 67,   3], [ 26,   8], [ 44,   3],
                    [ 32,  13], [ 42,  10], [ 32,  18], [ 51,  15], [ 63,  12], [  9,  12], [ 31,   8], [ 43,   5],
                ];

                    pub const QUEEN_TABLE: [[i32; 2]; 64] = [
                        [ -1, -33], [-18, -28], [ -9, -22], [ 10, -43], [-15,  -5], [-25, -32], [-31, -20], [-50, -41],
                        [-35, -22], [ -8, -23], [ 11, -30], [  2, -16], [  8, -16], [ 15, -23], [ -3, -36], [  1, -32],
                        [-14, -16], [  2, -27], [-11,  15], [ -2,   6], [ -5,   9], [  2,  17], [ 14,  10], [  5,   5],
                        [ -9, -18], [-26,  28], [ -9,  19], [-10,  47], [ -2,  31], [ -4,  34], [  3,  39], [ -3,  23],
                        [-27,   3], [-27,  22], [-16,  24], [-16,  45], [ -1,  57], [ 17,  40], [ -2,  57], [  1,  36],
                        [-13,  20], [-17,   6], [  7,   9], [  8,  49], [ 29,  47], [ 56,  35], [ 47,  19], [ 57,   9],
                        [-24, -17], [-39,  20], [ -5,  32], [  1,  41], [-16,  58], [ 57,  25], [ 28,  30], [ 54,   0],
                        [-28,  -9], [  0,  22], [ 29,  22], [ 12,  27], [ 59,  27], [ 44,  19], [ 43,  10], [ 45,  20],
                    ];

                        pub const KING_TABLE: [[i32; 2]; 64] = [
                            [-15, -53], [ 36, -34], [ 12, -21], [-54, -11], [  8, -28], [-28, -14], [ 24, -24], [ 14, -43],
                            [  1, -27], [  7, -11], [ -8,   4], [-64,  13], [-43,  14], [-16,   4], [  9,  -5], [  8, -17],
                            [-14, -19], [-14,  -3], [-22,  11], [-46,  21], [-44,  23], [-30,  16], [-15,   7], [-27,  -9],
                            [-49, -18], [ -1,  -4], [-27,  21], [-39,  24], [-46,  27], [-44,  23], [-33,   9], [-51, -11],
                            [-17,  -8], [-20,  22], [-12,  24], [-27,  27], [-30,  26], [-25,  33], [-14,  26], [-36,   3],
                            [ -9,  10], [ 24,  17], [  2,  23], [-16,  15], [-20,  20], [  6,  45], [ 22,  44], [-22,  13],
                            [ 29, -12], [ -1,  17], [-20,  14], [-7,   17], [ -8,  17], [ -4,  38], [-38,  23], [-29,  11],
                            [-65, -74], [ 23, -35], [ 16, -18], [-15, -18], [-56, -11], [-34,  15], [  2,   4], [ 13, -17],
                        ];

                            pub const PIECE_TABLES: PieceTables = PieceTables([
                                PAWN_TABLE,
                                KNIGHT_TABLE,
                                BISHOP_TABLE,
                                ROOK_TABLE,
                                QUEEN_TABLE,
                                KING_TABLE,
                            ]);
}

pub use self::tables::*;

#[derive(Clone, Copy, Debug)]
pub enum GamePhase {
    Midgame = 0,
    Endgame = 1,
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct PieceTables([[[i32; 2]; 64]; 6]);
impl std::ops::Index<(GamePhase, PieceIndex, u8)> for PieceTables {
    type Output = i32;
    fn index(&self, index: (GamePhase, PieceIndex, u8)) -> &Self::Output {
        &self.0[index.1 as usize][index.2 as usize][index.0 as usize]
    }
}

impl Default for PieceTables {
    fn default() -> Self {
        PIECE_TABLES
    }
}
