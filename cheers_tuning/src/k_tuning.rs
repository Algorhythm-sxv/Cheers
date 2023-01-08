
use cheers_lib::board::evaluate::EVAL_PARAMS;

use crate::calculate_error::{calculate_error, TuningEntry};

pub fn tune_k(data: &[TuningEntry], count: usize) -> f64 {
    let mut start = 0.0;
    let mut end = 10.0;
    let mut step = 1.0;

    let params = EVAL_PARAMS.as_array().map(|x| x as f64);

    let mut best = calculate_error(data, &params, start, count);

    // 10 decimal digits of precision
    for i in 0..10 {
        let mut current = start - step;
        while current < end {
            current += step;
            let error = calculate_error(data, &params, current, count);
            if error < best || (error - best).abs() < 1e-10 {
                best = error;
                start = current;
            }
        }
        println!(
            "Step = [{:<11.places$}], K = [{:<11.places$}], E = [{best:.10}]",
            step,
            start,
            places = i
        );
        end = start + step;
        start -= step;
        step /= 10.0;
    }
    start
}
