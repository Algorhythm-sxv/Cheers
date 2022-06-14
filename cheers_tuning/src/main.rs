use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{prelude::*, stdout, BufWriter};
use std::{io::BufReader, path::PathBuf};

use cheers_lib::chessgame::{EvalParams, EVAL_PARAMS};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use pgn_reader::BufferedReader;
use rayon::prelude::*;

use cheers_lib::{lookup_tables::LookupTables, zobrist::initialise_zobrist_numbers};

use crate::calculate_error::{calculate_error, calculate_gradient, data_to_entry, TuningEntry};
use crate::data_extraction::FENWriter;
use crate::k_tuning::tune_k;

mod calculate_error;
mod data_extraction;
mod k_tuning;
mod parameter_tuning;

#[derive(Parser, Debug)]
struct Args {
    /// PGN file to extract FEN strings from
    #[clap(short, long)]
    extract: Option<PathBuf>,
    /// file to read/write FEN data
    #[clap(short, long)]
    data: Option<PathBuf>,
    /// skip tuning
    #[clap(short, long)]
    skip_tuning: bool,
    /// number of positions to analyse
    #[clap(short, long, default_value_t = 1_000_000)]
    count: usize,
    /// maximum tuning iterations
    #[clap(short, long, default_value_t = 1_000_000)]
    max_iters: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let data_path = if let Some(path) = args.data {
        println!("Reading FEN data from {}", path.to_string_lossy());
        path
    } else {
        println!("Reading/writing FEN data to ./data.fen");
        PathBuf::from("./data.fen")
    };
    let mut pgn_count = 0;
    let mut data_options = OpenOptions::new();
    data_options.read(true);
    if args.extract.is_some() {
        data_options.write(true).truncate(true).create(true);
    }

    let mut data_file = data_options.open(data_path.clone())?;
    if let Some(path) = args.extract {
        let mut fenwriter = FENWriter::new();
        let mut data_file = BufWriter::new(&mut data_file);
        let mut reader = BufferedReader::new(BufReader::new(File::open(path)?));
        while let Some(fen_list) = reader.read_game(&mut fenwriter)? {
            pgn_count += 1;
            print!("\rConverting PGN to fen list: {}", pgn_count);
            std::io::stdout().flush()?;
            for fen in fen_list {
                data_file.write_all(fen.as_bytes())?;
                data_file.write_all(b"\n")?;
            }
        }
        println!();
    }
    if !args.skip_tuning {
        println!("Commencing tuning");
        initialise_zobrist_numbers();
        LookupTables::generate_all(true);

        let mut data_file = OpenOptions::new().read(true).open(data_path)?;
        let mut data = String::new();
        print!("Reading data to memory... ");
        data_file.read_to_string(&mut data)?;
        let len = data.lines().count();
        let data = data
            .lines()
            .step_by(len / args.count)
            .fold(String::new(), |a, b| a + b + "\n");

        println!("done");

        println!("Converting data to tuning entries...");
        let entries_bar = ProgressBar::new(data.lines().count() as u64);
        entries_bar.set_draw_rate(20);
        entries_bar.set_style(ProgressStyle::default_bar().template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
        ));
        let data = data
            .par_lines()
            .map(|l| {
                entries_bar.clone().inc(1);
                data_to_entry(l)
            })
            .collect::<Vec<TuningEntry>>();
        entries_bar.finish();

        println!("Optimising sigmoid K parameter...");
        let best_k = tune_k(&data, args.count);
        println!("Best K: {best_k}");

        let mut output_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open("best_parameters.txt")?;
        let mut eval_params = EVAL_PARAMS.to_array().map(|x| x as f64);

        let mut adagrad = [0f64; EvalParams::LEN];
        let mut rate = 0.1;
        let drop_rate = 1.0;
        let rate_step = 250;

        for iter in 0..args.max_iters {
            let params = EvalParams::from_array(eval_params.map(|x| x as i32));
            let gradient = calculate_gradient(&data, &params, best_k);

            for i in (0..adagrad.len()).step_by(2) {
                adagrad[i] += ((best_k / 200.0) * gradient[i]).powf(2.0);
                adagrad[i + 1] += ((best_k / 200.0) * gradient[i + 1]).powf(2.0);

                eval_params[i] += (best_k / 200.0) * gradient[i] * (rate / (1e-8 + adagrad[i]));
                eval_params[i + 1] +=
                    (best_k / 200.0) * gradient[i + 1] * (rate / (1e-8 + adagrad[i + 1]));
            }
            let error = calculate_error(&data, &params, best_k, args.count);
            if iter % rate_step == 0 {
                rate /= drop_rate;
            }
            print!("\rIter [{iter}] Error = [{error:.10}], Rate = [{rate:.10}]");
            stdout().flush()?;

            // clear the file
            output_file.set_len(0)?;
            output_file.rewind()?;
            output_file.write_all(format!("{params:#?}").as_bytes())?;
        }
    }
    Ok(())
}
