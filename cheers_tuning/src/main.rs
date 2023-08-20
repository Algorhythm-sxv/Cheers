use std::error::Error;
use std::fs::OpenOptions;
use std::io::{prelude::*, stdout};
use std::path::PathBuf;

use cheers_lib::board::evaluate::{EvalParams, EVAL_PARAMS};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

use crate::calculate_error::{book_to_entry, calculate_error, calculate_gradient, TuningEntry};
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
    #[clap(short, long)]
    count: Option<usize>,
    /// maximum tuning iterations
    #[clap(short, long, default_value_t = 1_000_000)]
    max_iters: usize,
    /// learning rate step iterations
    #[clap(short, long, default_value_t = 1000)]
    rate_step_iters: usize,
    /// number of threads to use
    #[clap(short, long, default_value_t = 10)]
    threads: usize,
    /// initial learning rate
    #[clap(short = 'l', long, default_value_t = 0.1)]
    initial_lr: f64,
    /// ADAM first moment parameter
    #[clap(short = 'b', long, default_value_t = 0.9)]
    beta1: f64,
    /// ADAM second moment parameter
    #[clap(short = 'B', long, default_value_t = 0.999)]
    beta2: f64,
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
        .num_threads(args.threads)
        .build_global()
        .unwrap();

    let mut data_file = OpenOptions::new().read(true).open(data_path)?;
    let mut data_string = String::new();
    print!("Reading data to memory... ");
    data_file.read_to_string(&mut data_string)?;
    let len = data_string.lines().count();
    let data_string = data_string
        .lines()
        .step_by(if let Some(count) = args.count {
            len / count
        } else {
            1
        })
        .take(args.count.unwrap_or(len))
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
            book_to_entry(l)
        })
        .collect::<Vec<TuningEntry>>();
    entries_bar.finish();
    drop(data_string);

    println!("Optimising sigmoid K parameter...");
    let best_k = tune_k(&data, args.count.unwrap_or(len));
    println!("Best K: {best_k}");

    let mut output_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open("best_parameters.txt")?;
    let mut eval_params = EVAL_PARAMS.clone().to_array().map(|x| x as f64);

    let mut alpha = args.initial_lr;
    let mut first_moment = [0f64; EvalParams::LEN];
    let mut second_moment = [0f64; EvalParams::LEN];
    for iter in 0..args.max_iters {
        let gradient = calculate_gradient(&data, &eval_params, best_k);

        for ((x, &g), (m, v)) in eval_params
            .iter_mut()
            .zip(gradient.iter())
            .zip(first_moment.iter_mut().zip(second_moment.iter_mut()))
        {
            *m = args.beta1 * *m + (1.0 - args.beta1) * g;
            *v = args.beta2 * *v + (1.0 - args.beta2) * g * g;
            let mhat = *m / (1.0 - args.beta1.powi(iter as i32 + 1));
            let vhat = *v / (1.0 - args.beta2.powi(iter as i32 + 1));

            *x = *x - alpha * mhat / (vhat.sqrt() + 1e-8);
        }
        let error = calculate_error(&data, &eval_params, best_k, args.count.unwrap_or(len));
        print!("\rIter [{iter}] Error = [{error:.10}], Rate = [{alpha:.10}]");
        stdout().flush()?;

        if (iter + 1) % args.rate_step_iters == 0 {
            alpha /= 2.0;
        }
        // clear the file
        output_file.set_len(0)?;
        output_file.rewind()?;

        let params = EvalParams::from_array(eval_params.map(|x| x as i16));
        output_file.write_all(format!("{params:?}").as_bytes())?;
    }

    println!();
    Ok(())
}
