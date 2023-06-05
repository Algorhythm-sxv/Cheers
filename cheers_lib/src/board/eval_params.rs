#[cfg(feature = "eval-tracing")]
use bytemuck::{Pod, Zeroable};

use super::eval_types::*;

#[cfg_attr(feature = "eval-tracing", derive(Pod, Zeroable))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct EvalParams {
    pub piece_values: PieceValues,
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

pub static EVAL_PARAMS: EvalParams = EvalParams {
    piece_values: PieceValues([
        s!(93, 133),
        s!(415, 227),
        s!(432, 249),
        s!(595, 448),
        s!(1248, 835),
        s!(0, 0),
    ]),
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
            s!(-42, -21),
            s!(0, -31),
            s!(-18, -30),
            s!(-23, -29),
            s!(-5, -38),
            s!(24, -41),
            s!(52, -48),
            s!(-8, -49),
            s!(-39, -32),
            s!(-12, -35),
            s!(-4, -50),
            s!(-11, -38),
            s!(7, -45),
            s!(-14, -45),
            s!(35, -48),
            s!(-9, -49),
            s!(-46, -24),
            s!(-9, -33),
            s!(-12, -48),
            s!(8, -52),
            s!(7, -53),
            s!(-1, -53),
            s!(2, -45),
            s!(-42, -42),
            s!(-25, -4),
            s!(-3, -16),
            s!(-8, -28),
            s!(3, -39),
            s!(17, -49),
            s!(1, -41),
            s!(2, -28),
            s!(-29, -25),
            s!(-34, 62),
            s!(-12, 57),
            s!(9, 44),
            s!(2, 26),
            s!(23, 10),
            s!(56, 0),
            s!(-1, 33),
            s!(-28, 42),
            s!(43, 128),
            s!(31, 128),
            s!(20, 112),
            s!(52, 78),
            s!(79, 66),
            s!(59, 82),
            s!(-2, 124),
            s!(-47, 122),
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
            s!(-45, -49),
            s!(-5, -37),
            s!(-36, -13),
            s!(-15, -5),
            s!(-16, -12),
            s!(-1, -7),
            s!(-9, -37),
            s!(-68, -27),
            s!(-9, -29),
            s!(-16, -17),
            s!(10, -11),
            s!(17, 0),
            s!(17, -1),
            s!(28, -6),
            s!(9, -19),
            s!(4, -26),
            s!(-4, -19),
            s!(10, 1),
            s!(33, -3),
            s!(27, 20),
            s!(37, 16),
            s!(26, 2),
            s!(37, -13),
            s!(-8, -26),
            s!(3, -10),
            s!(16, 3),
            s!(31, 21),
            s!(28, 23),
            s!(39, 26),
            s!(39, 11),
            s!(22, -4),
            s!(-7, -5),
            s!(26, -13),
            s!(31, 14),
            s!(29, 18),
            s!(68, 25),
            s!(37, 24),
            s!(63, 15),
            s!(18, 12),
            s!(39, -17),
            s!(8, -23),
            s!(36, -13),
            s!(38, 15),
            s!(81, 9),
            s!(108, -14),
            s!(134, -10),
            s!(77, -22),
            s!(8, -17),
            s!(-56, -24),
            s!(-16, -12),
            s!(83, -22),
            s!(35, -7),
            s!(93, -30),
            s!(86, -30),
            s!(12, -26),
            s!(9, -46),
            s!(-249, -5),
            s!(-49, -46),
            s!(-35, -16),
            s!(-57, -8),
            s!(40, -32),
            s!(-115, -16),
            s!(-93, -27),
            s!(-139, -72),
        ],
        [
            s!(42, -36),
            s!(13, -15),
            s!(32, -33),
            s!(19, -7),
            s!(12, -10),
            s!(14, -18),
            s!(34, -32),
            s!(25, -25),
            s!(20, -13),
            s!(63, -25),
            s!(42, -14),
            s!(39, -3),
            s!(40, 0),
            s!(62, -13),
            s!(73, -17),
            s!(42, -29),
            s!(56, -24),
            s!(52, -5),
            s!(53, 0),
            s!(46, 7),
            s!(52, 8),
            s!(53, -1),
            s!(46, -17),
            s!(50, -25),
            s!(21, -12),
            s!(42, -3),
            s!(43, 8),
            s!(65, 1),
            s!(67, 2),
            s!(48, 9),
            s!(43, -12),
            s!(9, -15),
            s!(8, 1),
            s!(31, 8),
            s!(55, 2),
            s!(60, 10),
            s!(62, 8),
            s!(39, 0),
            s!(44, -4),
            s!(24, -13),
            s!(18, -2),
            s!(40, -3),
            s!(80, -4),
            s!(55, 5),
            s!(95, -10),
            s!(86, 1),
            s!(77, -8),
            s!(70, -18),
            s!(6, -20),
            s!(45, -8),
            s!(22, -5),
            s!(27, -13),
            s!(46, -6),
            s!(92, -23),
            s!(35, -5),
            s!(52, -41),
            s!(-9, -10),
            s!(-4, -25),
            s!(9, -21),
            s!(-24, -3),
            s!(-83, 1),
            s!(-7, -9),
            s!(25, -23),
            s!(-48, -1),
        ],
        [
            s!(-10, -3),
            s!(-6, 5),
            s!(9, 3),
            s!(11, 12),
            s!(16, -2),
            s!(1, -6),
            s!(-17, 0),
            s!(0, -34),
            s!(-36, 1),
            s!(-12, -1),
            s!(-14, 5),
            s!(-4, 2),
            s!(-1, -2),
            s!(0, -2),
            s!(22, -12),
            s!(-42, -2),
            s!(-29, -4),
            s!(-15, 0),
            s!(0, -2),
            s!(9, -4),
            s!(6, -3),
            s!(-1, -9),
            s!(31, -24),
            s!(12, -21),
            s!(-25, 3),
            s!(-25, 6),
            s!(-20, 10),
            s!(-3, 8),
            s!(3, 2),
            s!(-2, -1),
            s!(15, -13),
            s!(0, -16),
            s!(-22, 8),
            s!(0, 2),
            s!(23, 4),
            s!(36, -1),
            s!(16, 0),
            s!(26, -2),
            s!(53, -14),
            s!(45, -16),
            s!(7, 7),
            s!(17, 5),
            s!(19, 4),
            s!(48, -3),
            s!(62, -11),
            s!(71, -12),
            s!(96, -17),
            s!(66, -17),
            s!(18, 7),
            s!(7, 20),
            s!(44, 10),
            s!(79, -2),
            s!(43, -1),
            s!(98, -11),
            s!(106, -17),
            s!(74, -16),
            s!(58, 7),
            s!(69, 1),
            s!(82, 0),
            s!(79, -3),
            s!(118, -14),
            s!(146, -27),
            s!(103, -19),
            s!(51, -4),
        ],
        [
            s!(11, -32),
            s!(-5, -35),
            s!(5, -33),
            s!(22, -74),
            s!(0, -19),
            s!(-13, -36),
            s!(-37, -24),
            s!(0, -59),
            s!(-8, -16),
            s!(15, -15),
            s!(17, -24),
            s!(15, -5),
            s!(20, -18),
            s!(37, -59),
            s!(40, -66),
            s!(20, -46),
            s!(-4, -38),
            s!(15, -42),
            s!(5, 15),
            s!(11, -5),
            s!(8, 14),
            s!(16, 3),
            s!(18, 6),
            s!(7, -23),
            s!(-6, -13),
            s!(1, 5),
            s!(0, 10),
            s!(2, 39),
            s!(12, 25),
            s!(10, 13),
            s!(16, 10),
            s!(0, -5),
            s!(-20, -14),
            s!(-5, 14),
            s!(0, 12),
            s!(0, 38),
            s!(4, 58),
            s!(12, 54),
            s!(-3, 37),
            s!(24, 3),
            s!(-13, -24),
            s!(-6, -1),
            s!(13, 25),
            s!(35, 8),
            s!(48, 50),
            s!(89, -3),
            s!(123, -38),
            s!(71, -33),
            s!(-13, -3),
            s!(-23, 5),
            s!(-22, 30),
            s!(-50, 79),
            s!(-31, 59),
            s!(108, 26),
            s!(22, 39),
            s!(139, -92),
            s!(-42, 9),
            s!(-8, 13),
            s!(39, -2),
            s!(57, 5),
            s!(95, -20),
            s!(96, -16),
            s!(-15, 4),
            s!(-44, 47),
        ],
        [
            s!(-61, -22),
            s!(15, -24),
            s!(-11, -9),
            s!(-84, 3),
            s!(-15, -20),
            s!(-51, -1),
            s!(27, -30),
            s!(20, -52),
            s!(32, -28),
            s!(-24, 1),
            s!(-42, 18),
            s!(-95, 33),
            s!(-79, 32),
            s!(-47, 26),
            s!(12, 4),
            s!(13, -13),
            s!(7, -26),
            s!(17, -2),
            s!(-74, 26),
            s!(-90, 37),
            s!(-91, 40),
            s!(-68, 34),
            s!(-19, 16),
            s!(-41, 3),
            s!(37, -32),
            s!(23, -3),
            s!(6, 16),
            s!(-70, 34),
            s!(-60, 36),
            s!(-79, 39),
            s!(-40, 21),
            s!(-85, 11),
            s!(9, -21),
            s!(15, 2),
            s!(41, 11),
            s!(-4, 22),
            s!(-23, 24),
            s!(-14, 30),
            s!(4, 23),
            s!(-46, 15),
            s!(51, -15),
            s!(139, -1),
            s!(38, 11),
            s!(67, 6),
            s!(6, 19),
            s!(70, 22),
            s!(85, 25),
            s!(-16, 19),
            s!(49, -18),
            s!(118, -4),
            s!(79, -2),
            s!(53, 0),
            s!(55, 4),
            s!(115, 12),
            s!(9, 26),
            s!(-31, 16),
            s!(63, -53),
            s!(59, -22),
            s!(78, -22),
            s!(60, -16),
            s!(77, -15),
            s!(71, -3),
            s!(40, -3),
            s!(36, -29),
        ],
    ]),
};
