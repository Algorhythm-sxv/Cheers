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

    pub passed_pawn_defended: EvalScore,
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

    pub passed_pawn_defended: [i16; 2],
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
        s!(93, 112),
        s!(411, 216),
        s!(431, 240),
        s!(579, 426),
        s!(1233, 774),
        s!(0, 0),
    ]),
    knight_mobility: [
        s!(-98, -84),
        s!(-52, -39),
        s!(-28, -12),
        s!(-24, 8),
        s!(-4, 0),
        s!(0, 12),
        s!(10, 9),
        s!(20, 9),
        s!(30, -1),
    ],
    bishop_mobility: [
        s!(-52, -52),
        s!(-59, -35),
        s!(-38, -34),
        s!(-22, -15),
        s!(-12, -8),
        s!(-2, 0),
        s!(2, 6),
        s!(7, 7),
        s!(12, 11),
        s!(15, 8),
        s!(23, 9),
        s!(39, -2),
        s!(44, 8),
        s!(45, -8),
    ],
    rook_mobility: [
        s!(-108, -59),
        s!(-77, -51),
        s!(-24, -28),
        s!(-16, -10),
        s!(-12, -6),
        s!(-5, -8),
        s!(-3, 0),
        s!(6, 0),
        s!(13, 3),
        s!(26, 1),
        s!(34, 6),
        s!(42, 7),
        s!(49, 11),
        s!(52, 12),
        s!(79, -2),
    ],
    queen_mobility: [
        s!(0, 0),
        s!(-100, -32),
        s!(-159, -27),
        s!(-68, -116),
        s!(-44, -78),
        s!(-35, -99),
        s!(-29, -68),
        s!(-27, -45),
        s!(-25, -11),
        s!(-18, -25),
        s!(-19, 8),
        s!(-14, 8),
        s!(-6, 18),
        s!(-4, 19),
        s!(0, 25),
        s!(2, 29),
        s!(-3, 44),
        s!(7, 37),
        s!(0, 50),
        s!(-2, 57),
        s!(12, 49),
        s!(18, 54),
        s!(7, 53),
        s!(68, 13),
        s!(96, 5),
        s!(38, 38),
        s!(145, -26),
        s!(182, -47),
    ],
    passed_pawn_defended: s!(30, 9),
    passed_pawn_table: [
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(0, 0),
        s!(-13, -9),
        s!(-25, 7),
        s!(-7, -6),
        s!(-22, -23),
        s!(-10, 1),
        s!(8, -8),
        s!(48, -8),
        s!(-11, 0),
        s!(-23, -1),
        s!(-30, 2),
        s!(-22, 2),
        s!(-49, 3),
        s!(-22, -1),
        s!(11, 1),
        s!(-7, 5),
        s!(0, -2),
        s!(-8, 19),
        s!(-25, 18),
        s!(-32, 20),
        s!(-37, 20),
        s!(-33, 18),
        s!(-49, 24),
        s!(-79, 38),
        s!(-22, 24),
        s!(-3, 41),
        s!(-6, 45),
        s!(-14, 36),
        s!(-28, 34),
        s!(-22, 29),
        s!(5, 29),
        s!(-20, 44),
        s!(-41, 43),
        s!(16, 96),
        s!(0, 72),
        s!(1, 47),
        s!(11, 17),
        s!(-8, 31),
        s!(-17, 69),
        s!(-62, 71),
        s!(-95, 107),
        s!(46, -1),
        s!(86, -13),
        s!(5, 62),
        s!(23, -1),
        s!(0, -20),
        s!(-8, 26),
        s!(-16, 95),
        s!(-61, 48),
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
            s!(-21, -23),
            s!(7, -30),
            s!(-8, -27),
            s!(1, -21),
            s!(9, -30),
            s!(31, -34),
            s!(55, -45),
            s!(2, -47),
            s!(-28, -28),
            s!(-7, -32),
            s!(-7, -40),
            s!(-5, -32),
            s!(13, -37),
            s!(-14, -36),
            s!(34, -44),
            s!(-6, -44),
            s!(-35, -24),
            s!(-15, -26),
            s!(-13, -42),
            s!(1, -46),
            s!(1, -44),
            s!(-1, -46),
            s!(-3, -39),
            s!(-36, -40),
            s!(-25, -10),
            s!(-16, -18),
            s!(-20, -27),
            s!(-7, -37),
            s!(-1, -41),
            s!(-11, -37),
            s!(-10, -27),
            s!(-25, -28),
            s!(-34, 7),
            s!(-19, 19),
            s!(-16, 25),
            s!(-28, 29),
            s!(-3, 7),
            s!(41, -21),
            s!(1, 4),
            s!(-7, -12),
            s!(-32, 149),
            s!(-27, 147),
            s!(-11, 70),
            s!(-15, 101),
            s!(24, 110),
            s!(37, 77),
            s!(-4, 52),
            s!(-25, 95),
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
            s!(-27, -22),
            s!(16, -14),
            s!(-22, -6),
            s!(-7, 0),
            s!(-1, -7),
            s!(-3, 0),
            s!(9, -18),
            s!(-42, 1),
            s!(10, -24),
            s!(-7, -11),
            s!(4, -16),
            s!(21, -8),
            s!(21, -8),
            s!(20, -9),
            s!(18, -14),
            s!(19, -19),
            s!(0, -15),
            s!(-3, 0),
            s!(5, -2),
            s!(0, 17),
            s!(13, 13),
            s!(11, -4),
            s!(23, -14),
            s!(0, -18),
            s!(11, -6),
            s!(3, 0),
            s!(7, 17),
            s!(13, 18),
            s!(16, 22),
            s!(24, 7),
            s!(12, -7),
            s!(6, 0),
            s!(26, -8),
            s!(16, 7),
            s!(5, 15),
            s!(46, 19),
            s!(18, 18),
            s!(27, 16),
            s!(8, 6),
            s!(46, -14),
            s!(0, -17),
            s!(12, -13),
            s!(1, 17),
            s!(43, 10),
            s!(66, -11),
            s!(95, -8),
            s!(49, -21),
            s!(3, -11),
            s!(-47, -12),
            s!(-24, -3),
            s!(71, -26),
            s!(21, -9),
            s!(80, -32),
            s!(65, -29),
            s!(16, -18),
            s!(14, -34),
            s!(-204, 18),
            s!(-38, -46),
            s!(-45, -10),
            s!(-60, -3),
            s!(27, -23),
            s!(-99, -10),
            s!(-113, -18),
            s!(-102, -44),
        ],
        [
            s!(59, -27),
            s!(27, -10),
            s!(46, -19),
            s!(30, -6),
            s!(26, -7),
            s!(20, -4),
            s!(48, -28),
            s!(49, -20),
            s!(32, -9),
            s!(57, -22),
            s!(38, -16),
            s!(39, -10),
            s!(37, -7),
            s!(62, -15),
            s!(68, -17),
            s!(49, -19),
            s!(55, -21),
            s!(44, -9),
            s!(43, -7),
            s!(34, 0),
            s!(43, 0),
            s!(43, -7),
            s!(33, -19),
            s!(46, -16),
            s!(26, -15),
            s!(25, -9),
            s!(28, 1),
            s!(46, -3),
            s!(48, -2),
            s!(28, 0),
            s!(28, -16),
            s!(23, -14),
            s!(-3, 2),
            s!(23, 0),
            s!(28, -3),
            s!(36, 5),
            s!(37, 2),
            s!(8, -3),
            s!(37, -13),
            s!(14, -11),
            s!(2, -1),
            s!(18, -6),
            s!(51, -9),
            s!(16, 4),
            s!(59, -11),
            s!(54, 0),
            s!(52, -10),
            s!(53, -14),
            s!(-6, -10),
            s!(16, -9),
            s!(0, -7),
            s!(4, -16),
            s!(18, -10),
            s!(59, -23),
            s!(9, -5),
            s!(36, -34),
            s!(-14, -8),
            s!(-36, -17),
            s!(-14, -15),
            s!(-39, -4),
            s!(-81, -1),
            s!(-15, -10),
            s!(-8, -16),
            s!(-45, 0),
        ],
        [
            s!(-9, 0),
            s!(-3, 0),
            s!(11, -4),
            s!(17, -1),
            s!(24, -12),
            s!(10, -6),
            s!(-4, -7),
            s!(12, -31),
            s!(-27, -1),
            s!(-15, -2),
            s!(-16, 2),
            s!(-1, -3),
            s!(2, -7),
            s!(4, -7),
            s!(29, -16),
            s!(-32, -6),
            s!(-30, -2),
            s!(-22, -1),
            s!(-12, -3),
            s!(5, -8),
            s!(3, -6),
            s!(2, -11),
            s!(22, -21),
            s!(10, -19),
            s!(-30, 6),
            s!(-34, 6),
            s!(-34, 10),
            s!(-18, 8),
            s!(-11, 4),
            s!(-11, 0),
            s!(8, -10),
            s!(-4, -11),
            s!(-33, 10),
            s!(-9, 2),
            s!(4, 5),
            s!(14, 0),
            s!(0, 1),
            s!(14, -1),
            s!(39, -10),
            s!(26, -10),
            s!(-6, 7),
            s!(0, 6),
            s!(-2, 5),
            s!(35, -3),
            s!(43, -10),
            s!(57, -11),
            s!(84, -16),
            s!(58, -16),
            s!(-11, 10),
            s!(-24, 23),
            s!(8, 13),
            s!(41, 0),
            s!(8, 2),
            s!(66, -8),
            s!(81, -12),
            s!(38, -8),
            s!(16, 12),
            s!(25, 5),
            s!(27, 7),
            s!(25, 2),
            s!(62, -8),
            s!(101, -20),
            s!(82, -17),
            s!(17, 0),
        ],
        [
            s!(25, -35),
            s!(21, -40),
            s!(26, -43),
            s!(34, -53),
            s!(26, -34),
            s!(9, -34),
            s!(-1, -33),
            s!(45, -67),
            s!(11, -17),
            s!(16, -22),
            s!(15, -24),
            s!(23, -26),
            s!(26, -33),
            s!(41, -57),
            s!(38, -57),
            s!(35, -32),
            s!(-2, -33),
            s!(13, -42),
            s!(0, 6),
            s!(9, -15),
            s!(6, 4),
            s!(11, -1),
            s!(13, 9),
            s!(6, -4),
            s!(-2, -12),
            s!(-3, 0),
            s!(-3, 0),
            s!(-9, 30),
            s!(2, 18),
            s!(5, 6),
            s!(10, 14),
            s!(2, 8),
            s!(-20, -10),
            s!(-6, 5),
            s!(-16, 6),
            s!(-16, 32),
            s!(-9, 50),
            s!(-6, 55),
            s!(-2, 37),
            s!(24, 14),
            s!(-14, -20),
            s!(-18, -3),
            s!(4, 15),
            s!(11, 10),
            s!(31, 45),
            s!(65, 3),
            s!(108, -32),
            s!(71, -16),
            s!(-19, 5),
            s!(-38, 7),
            s!(-37, 24),
            s!(-65, 75),
            s!(-50, 57),
            s!(79, 31),
            s!(3, 46),
            s!(127, -68),
            s!(-15, -9),
            s!(-24, 14),
            s!(25, -6),
            s!(28, 8),
            s!(84, -26),
            s!(78, -13),
            s!(-29, 12),
            s!(-20, 33),
        ],
        [
            s!(-75, -17),
            s!(6, -19),
            s!(-14, -4),
            s!(-83, 5),
            s!(-17, -11),
            s!(-42, 0),
            s!(22, -28),
            s!(7, -49),
            s!(15, -23),
            s!(-45, 6),
            s!(-59, 20),
            s!(-87, 31),
            s!(-76, 30),
            s!(-60, 28),
            s!(-5, 8),
            s!(-4, -10),
            s!(-2, -23),
            s!(2, 0),
            s!(-83, 27),
            s!(-93, 37),
            s!(-97, 40),
            s!(-72, 34),
            s!(-29, 16),
            s!(-47, 4),
            s!(29, -30),
            s!(16, -3),
            s!(0, 16),
            s!(-68, 32),
            s!(-61, 35),
            s!(-80, 37),
            s!(-43, 19),
            s!(-81, 10),
            s!(3, -21),
            s!(15, 1),
            s!(35, 11),
            s!(1, 21),
            s!(-20, 23),
            s!(-7, 28),
            s!(12, 19),
            s!(-41, 10),
            s!(49, -19),
            s!(142, -5),
            s!(43, 8),
            s!(71, 4),
            s!(10, 17),
            s!(78, 19),
            s!(87, 20),
            s!(-5, 11),
            s!(46, -24),
            s!(120, -8),
            s!(87, -4),
            s!(59, -2),
            s!(65, 1),
            s!(112, 9),
            s!(16, 21),
            s!(-15, 6),
            s!(117, -63),
            s!(67, -23),
            s!(110, -29),
            s!(79, -19),
            s!(102, -19),
            s!(100, -9),
            s!(58, -10),
            s!(62, -37),
        ],
    ]),
};
