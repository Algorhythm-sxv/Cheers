use rayon::prelude::*;

use cheers_lib::{
    board::{
        evaluate::{EvalParams, EvalTrace},
        Board,
    },
    hash_tables::PawnHashTable,
};

use crate::types::GameResult;

#[derive(Clone, Copy)]
pub struct TuningTuple {
    index: u16,
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

#[allow(dead_code)]
pub fn mf_to_entry(mf: &str) -> TuningEntry {
    let mut split = mf.split('|');
    let fen = split.next().expect("Empty line in MF data");
    let _eval_text = split
        .next()
        .unwrap_or_else(|| panic!("Incomplete line in MF data: {mf}"));
    let result_text = split
        .next()
        .unwrap_or_else(|| panic!("Incomplete line in MF data: {mf}"));

    let game = Board::from_fen(fen).unwrap_or_else(|| panic!("Invalid FEN extracted from MF data: {mf}"));

    let result = result_text
        .parse::<f64>()
        .unwrap_or_else(|_| panic!("Invalid result in MF data: {mf}"));

    let mut pawn_hash_table = PawnHashTable::new();

    let (_, trace) = game.evaluate_impl::<EvalTrace>(&mut pawn_hash_table);

    let tuples = trace
        .to_array()
        .chunks_exact(2)
        .enumerate()
        .filter(|(_i, c)| c[0] != c[1])
        .map(|(i, c)| TuningTuple {
            index: 2 * i as u16,
            white_coeff: c[0],
            black_coeff: c[1],
        })
        .collect::<Vec<TuningTuple>>();

    let material: i16 = trace.knight_count.into_iter().sum::<i16>()
        + trace.bishop_count.into_iter().sum::<i16>()
        + 2 * trace.rook_count.into_iter().sum::<i16>()
        + 4 * trace.queen_count.into_iter().sum::<i16>();
    let phase = (256 * (24 - material)) / 24;

    TuningEntry {
        phase: phase as u16,
        result: GameResult::from_f64(result),
        tuples,
    }
}

#[allow(dead_code)]
pub fn book_to_entry(book: &str) -> TuningEntry {
    let mut split = book.split(" [");
    let fen = split.next().expect("Empty line in book");
    let result_text = split.next().expect("Result missing in book");
    let game = Board::from_fen(fen).unwrap_or_else(|| panic!("Invalid FEN extracted: {fen}"));
    let result = match result_text {
        "1.0]" => 1.0,
        "0.5]" => 0.5,
        "0.0]" => 0.0,
        _ => panic!("Invalid result extracted from book: {result_text}"),
    };

    let mut pawn_hash_table = PawnHashTable::new();

    let (_, trace) = game.evaluate_impl::<EvalTrace>(&mut pawn_hash_table);

    let tuples = trace
        .to_array()
        .chunks_exact(2)
        .enumerate()
        .filter(|(_i, c)| c[0] != c[1])
        .map(|(i, c)| TuningTuple {
            index: 2 * i as u16,
            white_coeff: c[0],
            black_coeff: c[1],
        })
        .collect::<Vec<TuningTuple>>();

    let material: i16 = trace.knight_count.into_iter().sum::<i16>()
        + trace.bishop_count.into_iter().sum::<i16>()
        + 2 * trace.rook_count.into_iter().sum::<i16>()
        + 4 * trace.queen_count.into_iter().sum::<i16>();
    let phase = (256 * (24 - material).max(0)) / 24;

    TuningEntry {
        phase: phase as u16,
        result: GameResult::from_f64(result),
        tuples,
    }
}

#[allow(dead_code)]
pub fn epd_to_entry(epd: &str) -> TuningEntry {
    let mut split = epd.split(" c9 ");
    let almost_fen = split.next().expect("Empty line in EPD");
    let result_text = split
        .next()
        .unwrap_or_else(|| panic!("Result missing in EPD: {epd}"));

    let mut fen = String::from(almost_fen);
    fen += " 0 1"; // add the move counters to the end of the FEN
    let game = Board::from_fen(&fen).unwrap_or_else(|| panic!("Invalid FEN extracted: {fen}"));

    let result = match result_text {
        "\"1-0\";" => 1.0,
        "\"1/2-1/2\";" => 0.5,
        "\"0-1\";" => 0.0,
        _ => panic!("Invalid result extracted from EPD: {result_text}"),
    };

    let mut pawn_hash_table = PawnHashTable::new();

    let (_, trace) = game.evaluate_impl::<EvalTrace>(&mut pawn_hash_table);

    let tuples = trace
        .to_array()
        .chunks_exact(2)
        .enumerate()
        .filter(|(_i, c)| c[0] != c[1])
        .map(|(i, c)| TuningTuple {
            index: 2 * i as u16,
            white_coeff: c[0],
            black_coeff: c[1],
        })
        .collect::<Vec<TuningTuple>>();

    let material: i16 = trace.knight_count.into_iter().sum::<i16>()
        + trace.bishop_count.into_iter().sum::<i16>()
        + 2 * trace.rook_count.into_iter().sum::<i16>()
        + 4 * trace.queen_count.into_iter().sum::<i16>();
    let phase = (256 * (24 - material).max(0)) / 24;

    TuningEntry {
        phase: phase as u16,
        result: GameResult::from_f64(result),
        tuples,
    }
}

pub fn linear_evaluation(entry: &TuningEntry, params: &[f64; EvalParams::LEN]) -> f64 {
    let mut mg = 0f64;
    let mut eg = 0f64;
    for tuple in entry.tuples.iter() {
        let mg_weight = params[tuple.index as usize];
        let eg_weight = params[tuple.index as usize + 1];

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
            (entry.result.into_f64() - sigmoid(eval, k)).powf(2.0)
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
                let s = sigmoid(eval, k);
                let base = (b.result.into_f64() - s) * s * (s - 1.0);

                for tuple in &b.tuples {
                    let i = tuple.index;
                    a[i as usize] += base
                        * (((256 - b.phase) as f64) / 256.0)
                        * (tuple.white_coeff - tuple.black_coeff) as f64;
                    a[i as usize + 1] += base
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
