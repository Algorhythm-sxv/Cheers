use std::error::Error;
use std::fs::OpenOptions;
use std::io::{prelude::*, stdout};
use std::path::PathBuf;

use cheers_lib::board::evaluate::{EvalParams, EVAL_PARAMS};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

use crate::calculate_error::{calculate_error, calculate_gradient, epd_to_entry, TuningEntry};
use crate::k_tuning::tune_k;

mod calculate_error;
mod k_tuning;
mod types;

#[derive(Parser, Debug)]
struct Args {
    /// file to read EPD data
    #[clap(short, long)]
    data: Option<PathBuf>,
    /// number of positions to analyse
    #[clap(short, long, default_value_t = 1_000_000)]
    count: usize,
    /// maximum tuning iterations
    #[clap(short, long, default_value_t = 1_000_000)]
    max_iters: usize,
    /// learning rate step iterations
    #[clap(short, long, default_value_t = 1000)]
    rate_step_iters: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let data_path = if let Some(path) = args.data {
        println!("Reading EPD data from {}", path.to_string_lossy());
        path
    } else {
        println!("Reading EPD data from ./data.epd");
        PathBuf::from("./data.epd")
    };
    let mut data_options = OpenOptions::new();
    data_options.read(true);

    println!("Commencing tuning");

    ThreadPoolBuilder::new()
        .num_threads(6)
        .build_global()
        .unwrap();

    let mut data_file = OpenOptions::new().read(true).open(data_path)?;
    let mut data_string = String::new();
    print!("Reading data to memory... ");
    data_file.read_to_string(&mut data_string)?;
    let len = data_string.lines().count();
    let data_string = data_string
        .lines()
        .step_by(len / args.count)
        .take(args.count)
        .fold(String::new(), |a, b| a + b + "\n");

    println!("done");

    println!("Converting data to tuning entries...");
    let entries_bar = ProgressBar::new(data_string.lines().count() as u64);
    entries_bar.set_draw_rate(20);
    entries_bar.set_style(ProgressStyle::default_bar().template(
        "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
    ));
    let data = data_string
        .par_lines()
        .map(|l| {
            entries_bar.clone().inc(1);
            epd_to_entry(l)
        })
        .collect::<Vec<TuningEntry>>();
    entries_bar.finish();
    drop(data_string);

    println!("Optimising sigmoid K parameter...");
    let best_k = tune_k(&data, args.count);
    println!("Best K: {best_k}");

    let mut output_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open("best_parameters.txt")?;
    let mut eval_params = EVAL_PARAMS.clone().to_array().map(|x| x as f64);

    let mut adagrad = [0f64; EvalParams::LEN];
    let mut rate = 1024.0;
    let drop_rate = 2.0;

    for iter in 0..args.max_iters {
        let gradient = calculate_gradient(&data, &eval_params, best_k);

        for i in (0..adagrad.len()).step_by(2) {
            adagrad[i] += (2.0 * gradient[i] / args.count as f64).powf(2.0);
            adagrad[i + 1] += (2.0 * gradient[i + 1] / args.count as f64).powf(2.0);

            eval_params[i] += (best_k / (200.0 * args.count as f64))
                * gradient[i]
                * (rate / (1e-8 + adagrad[i]).sqrt());
            eval_params[i + 1] += (best_k / (200.0 * args.count as f64))
                * gradient[i + 1]
                * (rate / (1e-8 + adagrad[i + 1]).sqrt());
        }
        let error = calculate_error(&data, &eval_params, best_k, args.count);
        if iter != 0 && iter % args.rate_step_iters == 0 {
            rate /= drop_rate;
        }
        print!("\rIter [{iter}] Error = [{error:.10}], Rate = [{rate:.10}]");
        stdout().flush()?;

        // clear the file
        output_file.set_len(0)?;
        output_file.rewind()?;

        let params = EvalParams::from_array(eval_params.map(|x| x as i16));
        output_file.write_all(format!("{params:?}").as_bytes())?;
    }
    Ok(())
}
