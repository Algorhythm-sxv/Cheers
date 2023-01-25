use rayon::prelude::*;

use cheers_lib::{
    board::{
        evaluate::{EvalParams, EvalTrace, EVAL_PARAMS},
        Board,
    },
    hash_tables::TranspositionTable,
    moves::Move,
    search::Search,
};

use crate::data_extraction::GameResult;

#[derive(Clone, Copy)]
pub struct TuningTuple {
    index: usize,
    white_coeff: i16,
    black_coeff: i16,
}

#[derive(Clone)]
pub struct TuningEntry {
    phase: u16,
    //static_score: i16,
    //score: i16,
    // turn: ColorIndex,
    result: GameResult,
    tuples: Vec<TuningTuple>,
}
fn sigmoid(s: f64, k: f64) -> f64 {
    1.0 / (1.0 + (-k * s / 400.0).exp())
}

pub fn data_to_entry(line: &str) -> TuningEntry {
    let mut split = line.split(" result: ");
    let position = split.next().unwrap();
    let result = split.next().unwrap().parse::<f64>().unwrap();

    let game = Board::from_fen(position).unwrap();

    let mut search = Search::new(game);
    let tt_placeholder = TranspositionTable::new(0);

    let (_, trace) = search.quiesce_impl::<EvalTrace>(
        &game.clone(),
        i16::MIN + 1,
        i16::MAX - 1,
        0,
        Move::null(),
        &EVAL_PARAMS,
        &tt_placeholder,
    );
    // if game.current_player() == ColorIndex::Black {
    //     score = -score;
    // }
    let tuples = trace
        .to_array()
        .chunks_exact(2)
        .enumerate()
        .filter(|(_i, c)| c[0] != c[1])
        .map(|(i, c)| TuningTuple {
            index: 2 * i,
            white_coeff: c[0],
            black_coeff: c[1],
        })
        .collect::<Vec<TuningTuple>>();
    let material: i16 = trace.knight_count.into_iter().sum::<i16>()
        + trace.bishop_count.into_iter().sum::<i16>()
        + 2 * trace.rook_count.into_iter().sum::<i16>()
        + 4 * trace.queen_count.into_iter().sum::<i16>();
    let phase = (256 * (24 - material)) / 24;

    // let mut static_score = game.evaluate::<()>().0;

    // let turn = game.current_player();
    // if turn == ColorIndex::Black {
    //     static_score = -static_score;
    //     score = -score
    // }

    TuningEntry {
        phase: phase as u16,
        //static_score: static_score as i16,
        //score: score as i16,
        // turn,
        result: GameResult::from_f64(result),
        tuples,
    }
}

pub fn linear_evaluation(entry: &TuningEntry, params: &[f64; EvalParams::LEN]) -> f64 {
    let mut mg = 0f64;
    let mut eg = 0f64;
    for tuple in entry.tuples.iter() {
        let mg_weight = params[tuple.index];
        let eg_weight = params[tuple.index + 1];

        mg += mg_weight * (tuple.white_coeff - tuple.black_coeff) as f64;
        eg += eg_weight * (tuple.white_coeff - tuple.black_coeff) as f64;
    }
    ((256.0 - entry.phase as f64) * mg + entry.phase as f64 * eg) / 256.0
}

pub fn calculate_error(
    data: &[TuningEntry],
    eval_params: &[f64; EvalParams::LEN],
    k: f64,
    count: usize,
) -> f64 {
    data.par_iter()
        .take(count)
        .map(|entry| {
            let eval = linear_evaluation(entry, eval_params);
            (entry.result.into_f64() - sigmoid(eval as f64, k)).powf(2.0)
        })
        .sum::<f64>()
        / count as f64
}

//pub fn calculate_error_static(data: &[TuningEntry], k: f64, count: usize) -> f64 {
//    data.par_iter()
//        .take(count)
//        .map(|e| (e.result.into_f64() - sigmoid(e.static_score as f64, k)).powf(2.0))
//        .sum::<f64>()
//        / count as f64
//}

pub fn calculate_gradient(
    data: &[TuningEntry],
    eval_params: &[f64; EvalParams::LEN],
    k: f64,
) -> [f64; EvalParams::LEN] {
    let par_score = data
        .par_iter()
        .fold(
            || Box::new([0f64; EvalParams::LEN]),
            |mut a: Box<[f64; EvalParams::LEN]>, b: &TuningEntry| {
                let eval = linear_evaluation(b, eval_params);
                let s = sigmoid(eval as f64, k);
                let base = (b.result.into_f64() - s) * s * (1.0 - s);

                for tuple in &b.tuples {
                    let i = tuple.index;
                    a[i] += base
                        * (((256 - b.phase) as f64) / 256.0)
                        * (tuple.white_coeff - tuple.black_coeff) as f64;
                    a[i + 1] += base
                        * (b.phase as f64 / 256.0)
                        * (tuple.white_coeff - tuple.black_coeff) as f64;
                }
                a
            },
        )
        .reduce(
            || Box::new([0f64; EvalParams::LEN]),
            |mut a, b| {
                for i in 0..a.len() {
                    a[i] += b[i]
                }
                a
            },
        );

    *par_score
}

// #[cfg(test)]
// mod tests {
// #[test]
// fn test_linear_eval() {
// let tuples = trace
// .to_array()
// .chunks_exact(2)
// .enumerate()
// .filter(|(_i, c)| c[0] != c[1])
// .map(|(i, c)| TuningTuple {
// index: 2 * i,
// white_coeff: c[0],
// black_coeff: c[1],
// })
// .collect::<Vec<TuningTuple>>();
//
// let material: i32 = trace.knight_count.into_iter().sum::<i32>()
// + trace.bishop_count.into_iter().sum::<i32>()
// + 2 * trace.rook_count.into_iter().sum::<i32>()
// + 4 * trace.queen_count.into_iter().sum::<i32>();
// let phase = (256 * (24 - material)) / 24;
//
// let eval_params = EVAL_PARAMS.to_array();
// let mut mg = 0;
// let mut eg = 0;
// for tuple in tuples.iter() {
// let mg_weight = eval_params[tuple.index];
// let eg_weight = eval_params[tuple.index + 1];
//
// mg += mg_weight * (tuple.white_coeff - tuple.black_coeff);
// eg += eg_weight * (tuple.white_coeff - tuple.black_coeff);
// }
// let eval = ((256 - phase as i32) * mg + phase as i32 * eg) / 256;
//
// assert!(eval == score);
// }
// }
