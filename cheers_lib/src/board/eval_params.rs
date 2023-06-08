#[cfg(feature = "eval-tracing")]
use bytemuck::{Pod, Zeroable};

use super::eval_types::*;

#[cfg_attr(feature = "eval-tracing", derive(Pod, Zeroable))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct EvalParams {
    pub piece_values: PieceValues,

    pub knight_mobility: [EvalScore; 9],

    pub bishop_mobility: [EvalScore; 14],

    pub rook_mobility: [EvalScore; 15],

    pub queen_mobility: [EvalScore; 28],

    pub pawn_connected: [EvalScore; 3],

    pub passed_pawn_table: [EvalScore; 64],

    pub piece_tables: PieceTables,
}

#[cfg(feature = "eval-tracing")]
impl EvalParams {
    pub const LEN: usize = std::mem::size_of::<Self>() / std::mem::size_of::<i16>();
    pub fn to_array(&mut self) -> [i16; Self::LEN] {
        let array = &mut bytemuck::cast::<EvalParams, [i16; Self::LEN]>(*self);
        array
            .chunks_exact_mut(2)
            .for_each(|p| EvalScore::convert(p));
        *array
    }
    pub fn as_array(&self) -> &[i16; Self::LEN] {
        bytemuck::cast_ref::<EvalParams, [i16; Self::LEN]>(self)
    }
    pub fn from_array(mut params: [i16; Self::LEN]) -> Self {
        params.chunks_exact_mut(2).for_each(|p| {
            let score = EvalScore::new(p[0], p[1]);
            p[1] = (score.inner() >> 16) as i16;
            p[0] = score.inner() as i16;
        });
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
    pub bishop_mobility: [[i16; 2]; 14],
    pub rook_mobility: [[i16; 2]; 15],
    pub queen_mobility: [[i16; 2]; 28],

    pub pawn_connected: [[i16; 2]; 3],

    pub passed_pawn_placement: [[i16; 2]; 64],

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

// static assert that eval params and trace are the same length (plus 1 for turn in trace)
#[cfg(feature = "eval-tracing")]
const _PARAMS_TRACE_LEN_EQ: () = if EvalParams::LEN + 1 == EvalTrace::LEN {
    ()
} else {
    panic!("Eval parameters and trace are not equal length!")
};

pub const CHECKMATE_SCORE: i16 = 30000;
pub const DRAW_SCORE: i16 = 0;

pub static EVAL_PARAMS: EvalParams = EvalParams {
    piece_values: PieceValues([
        s!(94, 115),
        s!(412, 217),
        s!(434, 240),
        s!(584, 427),
        s!(1239, 779),
        s!(0, 0),
    ]),
    knight_mobility: [
        s!(-100, -77),
        s!(-52, -39),
        s!(-26, -13),
        s!(-22, 7),
        s!(-3, 0),
        s!(1, 12),
        s!(10, 9),
        s!(21, 9),
        s!(33, -2),
    ],
    bishop_mobility: [
        s!(-46, -48),
        s!(-57, -31),
        s!(-36, -31),
        s!(-20, -14),
        s!(-11, -7),
        s!(-3, 0),
        s!(2, 6),
        s!(7, 7),
        s!(13, 11),
        s!(16, 8),
        s!(26, 8),
        s!(44, -3),
        s!(47, 7),
        s!(52, -9),
    ],
    rook_mobility: [
        s!(-101, -56),
        s!(-79, -50),
        s!(-24, -27),
        s!(-16, -10),
        s!(-13, -6),
        s!(-5, -7),
        s!(-3, 1),
        s!(7, 0),
        s!(13, 4),
        s!(26, 2),
        s!(35, 6),
        s!(43, 7),
        s!(50, 11),
        s!(53, 12),
        s!(82, -2),
    ],
    queen_mobility: [
        s!(0, 0),
        s!(-91, -29),
        s!(-154, -26),
        s!(-69, -113),
        s!(-44, -77),
        s!(-36, -99),
        s!(-29, -65),
        s!(-27, -45),
        s!(-26, -8),
        s!(-19, -22),
        s!(-19, 10),
        s!(-14, 9),
        s!(-7, 18),
        s!(-5, 18),
        s!(-1, 25),
        s!(1, 29),
        s!(-3, 43),
        s!(7, 37),
        s!(-1, 50),
        s!(-3, 56),
        s!(11, 48),
        s!(18, 53),
        s!(5, 53),
        s!(71, 10),
        s!(97, 3),
        s!(41, 34),
        s!(140, -25),
        s!(180, -47),
    ],
    pawn_connected: [s!(-7, -10), s!(5, 5), s!(13, 12)],
    passed_pawn_table: [
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(-10, -1),
        s!(-19, 15),
        s!(-5, -2),
        s!(-19, -16),
        s!(-7, 5),
        s!(10, -1),
        s!(53, 0),
        s!(-7, 5),
        s!(-5, 7),
        s!(-4, 14),
        s!(-6, 8),
        s!(-36, 8),
        s!(0, 4),
        s!(24, 8),
        s!(18, 17),
        s!(18, 5),
        s!(0, 29),
        s!(-12, 29),
        s!(-24, 29),
        s!(-25, 28),
        s!(-25, 25),
        s!(-37, 33),
        s!(-69, 49),
        s!(-8, 33),
        s!(0, 51),
        s!(2, 56),
        s!(-5, 42),
        s!(-21, 40),
        s!(-14, 35),
        s!(12, 36),
        s!(-15, 55),
        s!(-37, 51),
        s!(16, 106),
        s!(5, 81),
        s!(3, 54),
        s!(12, 23),
        s!(-7, 36),
        s!(-12, 76),
        s!(-59, 79),
        s!(-92, 113),
        s!(43, 1),
        s!(81, -10),
        s!(5, 68),
        s!(23, 0),
        s!(0, -16),
        s!(-12, 31),
        s!(-15, 94),
        s!(-54, 50),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
    ],
    piece_tables: PieceTables([
        [
            s!(0, 0),
            s!(0, 0),
            s!(0, 0),
            s!(0, 0),
            s!(0, 0),
            s!(0, 0),
            s!(0, 0),
            s!(0, 0),
            s!(-16, -24),
            s!(8, -30),
            s!(-4, -24),
            s!(2, -21),
            s!(12, -28),
            s!(34, -33),
            s!(58, -46),
            s!(5, -45),
            s!(-31, -32),
            s!(-19, -38),
            s!(-13, -39),
            s!(-12, -31),
            s!(1, -37),
            s!(-16, -37),
            s!(21, -51),
            s!(-10, -45),
            s!(-32, -27),
            s!(-16, -29),
            s!(-10, -43),
            s!(0, -47),
            s!(3, -44),
            s!(-2, -48),
            s!(-3, -43),
            s!(-36, -42),
            s!(-18, -12),
            s!(-14, -21),
            s!(-16, -26),
            s!(-3, -36),
            s!(3, -39),
            s!(-4, -36),
            s!(-6, -30),
            s!(-19, -28),
            s!(-27, 7),
            s!(-15, 19),
            s!(-7, 27),
            s!(-21, 32),
            s!(5, 10),
            s!(50, -20),
            s!(10, 4),
            s!(0, -10),
            s!(-21, 154),
            s!(-18, 153),
            s!(-4, 73),
            s!(-7, 108),
            s!(36, 114),
            s!(48, 80),
            s!(0, 62),
            s!(-23, 102),
            s!(0, 0),
            s!(0, 0),
            s!(0, 0),
            s!(0, 0),
            s!(0, 0),
            s!(0, 0),
            s!(0, 0),
            s!(0, 0),
        ],
        [
            s!(-29, -22),
            s!(14, -16),
            s!(-21, -7),
            s!(-6, 0),
            s!(0, -7),
            s!(-2, 0),
            s!(8, -19),
            s!(-41, 0),
            s!(9, -25),
            s!(-6, -12),
            s!(4, -15),
            s!(20, -7),
            s!(20, -8),
            s!(18, -9),
            s!(20, -15),
            s!(19, -20),
            s!(-1, -15),
            s!(-3, -1),
            s!(5, -1),
            s!(0, 18),
            s!(13, 13),
            s!(12, -3),
            s!(24, -15),
            s!(-1, -18),
            s!(10, -7),
            s!(3, 0),
            s!(7, 18),
            s!(12, 20),
            s!(17, 22),
            s!(25, 7),
            s!(12, -7),
            s!(6, 1),
            s!(26, -8),
            s!(17, 7),
            s!(6, 16),
            s!(48, 19),
            s!(19, 18),
            s!(29, 17),
            s!(9, 6),
            s!(46, -15),
            s!(-1, -18),
            s!(13, -13),
            s!(2, 18),
            s!(43, 10),
            s!(66, -11),
            s!(97, -8),
            s!(51, -21),
            s!(4, -12),
            s!(-47, -12),
            s!(-25, -3),
            s!(69, -26),
            s!(22, -9),
            s!(80, -31),
            s!(66, -30),
            s!(20, -19),
            s!(15, -34),
            s!(-208, 20),
            s!(-34, -46),
            s!(-45, -10),
            s!(-63, -1),
            s!(24, -22),
            s!(-101, -9),
            s!(-112, -19),
            s!(-101, -44),
        ],
        [
            s!(57, -27),
            s!(27, -11),
            s!(45, -20),
            s!(32, -6),
            s!(26, -7),
            s!(20, -4),
            s!(49, -29),
            s!(50, -21),
            s!(31, -10),
            s!(57, -22),
            s!(38, -16),
            s!(39, -9),
            s!(38, -6),
            s!(60, -15),
            s!(70, -18),
            s!(46, -20),
            s!(53, -21),
            s!(44, -8),
            s!(43, -6),
            s!(34, 2),
            s!(42, 2),
            s!(44, -6),
            s!(33, -18),
            s!(46, -17),
            s!(26, -15),
            s!(23, -8),
            s!(29, 2),
            s!(43, -1),
            s!(47, -1),
            s!(27, 1),
            s!(28, -15),
            s!(21, -14),
            s!(-2, 1),
            s!(23, 0),
            s!(27, -1),
            s!(36, 5),
            s!(36, 2),
            s!(8, -1),
            s!(38, -12),
            s!(16, -11),
            s!(3, -1),
            s!(17, -6),
            s!(52, -9),
            s!(15, 4),
            s!(61, -10),
            s!(54, 0),
            s!(52, -9),
            s!(54, -13),
            s!(-7, -9),
            s!(17, -9),
            s!(-1, -7),
            s!(5, -15),
            s!(16, -9),
            s!(63, -23),
            s!(8, -5),
            s!(39, -35),
            s!(-13, -10),
            s!(-38, -18),
            s!(-11, -15),
            s!(-44, -3),
            s!(-87, 0),
            s!(-17, -10),
            s!(-10, -16),
            s!(-46, 0),
        ],
        [
            s!(-9, 0),
            s!(-3, 0),
            s!(11, -4),
            s!(17, -1),
            s!(25, -12),
            s!(10, -7),
            s!(-4, -7),
            s!(12, -32),
            s!(-26, -2),
            s!(-15, -3),
            s!(-16, 2),
            s!(-1, -4),
            s!(2, -7),
            s!(2, -7),
            s!(28, -16),
            s!(-33, -6),
            s!(-31, -2),
            s!(-21, -1),
            s!(-12, -2),
            s!(4, -7),
            s!(2, -5),
            s!(1, -11),
            s!(24, -22),
            s!(10, -19),
            s!(-32, 6),
            s!(-37, 6),
            s!(-34, 10),
            s!(-19, 8),
            s!(-11, 5),
            s!(-14, 0),
            s!(8, -10),
            s!(-5, -11),
            s!(-32, 10),
            s!(-6, 2),
            s!(5, 5),
            s!(16, -1),
            s!(2, 0),
            s!(15, -1),
            s!(41, -11),
            s!(25, -9),
            s!(-5, 7),
            s!(0, 6),
            s!(-2, 4),
            s!(34, -4),
            s!(43, -10),
            s!(58, -12),
            s!(87, -17),
            s!(59, -16),
            s!(-12, 10),
            s!(-24, 22),
            s!(6, 13),
            s!(40, 0),
            s!(7, 2),
            s!(66, -8),
            s!(82, -13),
            s!(37, -8),
            s!(15, 12),
            s!(26, 5),
            s!(29, 6),
            s!(22, 3),
            s!(60, -7),
            s!(101, -20),
            s!(87, -18),
            s!(13, 1),
        ],
        [
            s!(26, -37),
            s!(21, -41),
            s!(26, -43),
            s!(33, -53),
            s!(26, -34),
            s!(10, -37),
            s!(0, -35),
            s!(45, -68),
            s!(12, -18),
            s!(16, -22),
            s!(16, -24),
            s!(22, -26),
            s!(25, -31),
            s!(39, -57),
            s!(38, -57),
            s!(34, -33),
            s!(-2, -35),
            s!(12, -41),
            s!(0, 5),
            s!(9, -14),
            s!(5, 5),
            s!(12, -2),
            s!(12, 9),
            s!(6, -4),
            s!(-2, -14),
            s!(-5, 0),
            s!(-2, 0),
            s!(-10, 30),
            s!(1, 19),
            s!(5, 8),
            s!(10, 16),
            s!(1, 10),
            s!(-19, -12),
            s!(-6, 6),
            s!(-15, 6),
            s!(-15, 33),
            s!(-11, 52),
            s!(-6, 57),
            s!(-3, 39),
            s!(25, 15),
            s!(-14, -20),
            s!(-19, -3),
            s!(7, 15),
            s!(11, 10),
            s!(35, 45),
            s!(64, 5),
            s!(112, -33),
            s!(71, -15),
            s!(-20, 4),
            s!(-37, 6),
            s!(-39, 25),
            s!(-66, 76),
            s!(-50, 57),
            s!(81, 29),
            s!(3, 46),
            s!(131, -71),
            s!(-15, -7),
            s!(-20, 11),
            s!(25, -6),
            s!(29, 8),
            s!(84, -24),
            s!(80, -13),
            s!(-29, 12),
            s!(-21, 34),
        ],
        [
            s!(-73, -18),
            s!(7, -18),
            s!(-13, -4),
            s!(-84, 5),
            s!(-17, -11),
            s!(-42, 0),
            s!(23, -28),
            s!(7, -49),
            s!(16, -23),
            s!(-45, 6),
            s!(-60, 21),
            s!(-88, 31),
            s!(-77, 30),
            s!(-60, 28),
            s!(-2, 7),
            s!(1, -11),
            s!(-3, -23),
            s!(1, 0),
            s!(-83, 27),
            s!(-94, 37),
            s!(-99, 41),
            s!(-72, 35),
            s!(-32, 17),
            s!(-48, 5),
            s!(26, -30),
            s!(12, -3),
            s!(0, 16),
            s!(-69, 33),
            s!(-61, 36),
            s!(-82, 38),
            s!(-42, 20),
            s!(-83, 10),
            s!(3, -21),
            s!(14, 1),
            s!(35, 11),
            s!(1, 21),
            s!(-21, 24),
            s!(-8, 29),
            s!(13, 20),
            s!(-43, 11),
            s!(51, -20),
            s!(142, -5),
            s!(44, 8),
            s!(73, 4),
            s!(10, 17),
            s!(77, 20),
            s!(86, 21),
            s!(-7, 12),
            s!(47, -24),
            s!(120, -8),
            s!(84, -4),
            s!(60, -2),
            s!(66, 1),
            s!(112, 10),
            s!(17, 21),
            s!(-16, 7),
            s!(118, -64),
            s!(67, -23),
            s!(110, -29),
            s!(80, -19),
            s!(103, -19),
            s!(98, -9),
            s!(57, -10),
            s!(61, -36),
        ],
    ]),
};
