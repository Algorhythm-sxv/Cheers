use std::borrow::Borrow;
use std::borrow::Cow;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{prelude::*, BufWriter};
use std::{io::BufReader, path::PathBuf};

use argmin::prelude::*;
use argmin::solver::gradientdescent::SteepestDescent;
use argmin::solver::linesearch::MoreThuenteLineSearch;
use cheers_lib::types::ColorIndex;
use clap::Parser;
use pgn_reader::{BufferedReader, CastlingSide, RawHeader, Role, San, Skip, Visitor};
use rayon::prelude::*;

use cheers_lib::{
    chessgame::*,
    lookup_tables::LookupTables,
    moves::Move,
    transposition_table::{TranspositionTable, TT_DEFAULT_SIZE},
    types::PieceIndex,
    zobrist::initialise_zobrist_numbers,
};

enum GameResult {
    WhiteWin,
    BlackWin,
    Draw,
    Abandoned,
}
use GameResult::*;

impl ToString for GameResult {
    fn to_string(&self) -> String {
        match self {
            WhiteWin => String::from("1"),
            BlackWin => String::from("0"),
            Draw => String::from("0.5"),
            Abandoned => String::from("*"),
        }
    }
}

struct FENWriter {
    move_counter: usize,
    game: ChessGame,
    game_result: GameResult,
    fen_list: Vec<String>,
}

impl FENWriter {
    fn new() -> Self {
        LookupTables::generate_all(true);
        initialise_zobrist_numbers();
        let tt = TranspositionTable::new(TT_DEFAULT_SIZE);
        Self {
            move_counter: 0,
            game: ChessGame::new(tt),
            game_result: GameResult::Draw,
            fen_list: Vec::new(),
        }
    }
    fn reset(&mut self) {
        self.game.reset();
        self.game_result = Draw;
        self.fen_list.clear();
        self.move_counter = 0;
    }
}

impl Visitor for FENWriter {
    type Result = Vec<String>;

    fn header(&mut self, key: &[u8], value: RawHeader<'_>) {
        if key == b"Result" {
            self.game_result = match value.as_bytes() {
                b"1-0" => WhiteWin,
                b"0-1" => BlackWin,
                b"1/2-1/2" => Draw,
                b"*" => Abandoned,
                other => panic!("Invalid game result: {}", String::from_utf8_lossy(other)),
            }
        }
    }
    fn end_headers(&mut self) -> Skip {
        match self.game_result {
            Abandoned => Skip(true),
            _ => Skip(false),
        }
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true)
    }

    fn end_game(&mut self) -> Self::Result {
        // discard last 10 moves (to avoid easy mating positions)
        let list = self.fen_list[..(self.fen_list.len().saturating_sub(10))].to_vec();
        self.reset();
        list
    }

    fn san(&mut self, san_plus: pgn_reader::SanPlus) {
        let san = san_plus.san;
        if let Some(m) = san_to_move(san, &self.game) {
            self.game.make_move(m);

            self.move_counter += 1;
            // add moves out of opening and not close to mate
            if self.move_counter >= 10 {
                let score = 0; //self.game.search(Some(4), true).0;
                if score < 20000 {
                    self.fen_list
                        //.push(self.game.fen() + " score: " + &score.to_string());
                        .push(self.game.fen() + " result: " + &self.game_result.to_string())
                }
            }
        }
    }
}

fn san_to_move(san: San, game: &ChessGame) -> Option<Move> {
    let mut candidates = game.legal_moves();
    candidates.retain(|m| match san {
        San::Normal {
            role,
            file,
            rank,
            capture,
            to,
            promotion,
        } => {
            let correct_file = if let Some(f) = file {
                f as u8 == m.start() % 8
            } else {
                true
            };
            let correct_rank = if let Some(r) = rank {
                r as u8 == m.start() / 8
            } else {
                true
            };
            compare_roles(m.piece(), role)
                && m.target() == to as u8
                && m.capture() == capture
                && correct_file
                && correct_rank
                && promotion
                    .map(|p| compare_roles(m.promotion(), p))
                    .unwrap_or(true)
        }
        San::Castle(side) => {
            m.castling()
                && m.target() % 8
                    == match side {
                        CastlingSide::KingSide => 6,
                        CastlingSide::QueenSide => 2,
                    }
        }
        _ => false,
    });
    candidates.pop()
}

fn compare_roles(piece_index: PieceIndex, role: Role) -> bool {
    match piece_index {
        PieceIndex::Pawn => role == Role::Pawn,
        PieceIndex::Knight => role == Role::Knight,
        PieceIndex::Bishop => role == Role::Bishop,
        PieceIndex::Rook => role == Role::Rook,
        PieceIndex::Queen => role == Role::Queen,
        PieceIndex::King => role == Role::King,
        PieceIndex::NoPiece => unreachable!(),
    }
}

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
}

fn sigmoid(s: f64, k: f64) -> f64 {
    1.0 / (1.0 + (-k * s).exp())
}

fn calculate_error(data: &str, eval_params: &EvalParams, k: f64, count: usize) -> f64 {
    let data_index = data
        .split('\n')
        .take(count)
        .fold(0usize, |a, s| a + s.len() + 1);
    let data = data.split_at(data_index).0;

    let tt = TranspositionTable::new(0);
    let game = ChessGame::new(tt);
    let error = data
        .par_lines()
        .map(|line| {
            let mut split = line.split(" result: ");
            let position = split.next().unwrap();
            let result = split.next().unwrap().parse::<f64>().unwrap();

            let mut game = game.clone();
            game.set_from_fen(position).unwrap();
            let mut q = game.quiesce(i32::MIN + 1, i32::MAX - 1, Move::null(), *eval_params);
            if game.current_player() == ColorIndex::Black {
                q = -q
            }
            let s = sigmoid(q as f64, k);
            (result - s) * (result - s)
        })
        .sum::<f64>();
    error / (count as f64)
}

struct OptimizeSigmoidScale<'a> {
    data: Cow<'a, str>,
    eval_params: EvalParams,
    count: usize,
}

impl<'a> ArgminOp for OptimizeSigmoidScale<'a> {
    type Param = f64;
    type Output = f64;
    type Hessian = ();
    type Jacobian = ();
    type Float = f64;

    fn apply(&self, p: &Self::Param) -> Result<Self::Output, argmin::core::Error> {
        Ok(calculate_error(
            self.data.borrow(),
            &self.eval_params,
            *p,
            self.count,
        ))
    }

    fn gradient(&self, p: &Self::Param) -> Result<Self::Param, argmin::core::Error> {
        Ok(
            (calculate_error(self.data.borrow(), &self.eval_params, *p + 0.01, self.count)
                - calculate_error(self.data.borrow(), &self.eval_params, *p, self.count))
                / 0.01,
        )
    }
}

struct TexelTuning<'a> {
    data: Cow<'a, str>,
    k: f64,
    count: usize,
}

impl<'a> TexelTuning<'a> {
    fn local_search(&self, params: &mut Vec<i32>) {
        let mut current_score = calculate_error(
            self.data.borrow(),
            &EvalParams::from_params(params),
            self.k,
            self.count,
        );
        loop {
            let mut best_score = current_score;
            let mut best_delta = 0f64;
            let mut best_improvement = None;
            for i in 0..params.len() {
                // try p + 1, p - 1
                for d in [1i32, -2i32] {
                    params[i] += d;

                    // better scores are smaller so good deltas are positive
                    let score = calculate_error(
                        self.data.borrow(),
                        &EvalParams::from_params(params),
                        self.k,
                        self.count,
                    );
                    let delta = current_score - score;

                    params[i] -= d;
                    if delta > best_delta {
                        best_delta = delta;
                        best_improvement = Some((i, d.signum()));
                        best_score = score;
                        break;
                    }
                }
            }
            if let Some((i, d)) = best_improvement {
                params[i] += d;
                current_score = best_score;
                println!(
                    "New best params: {params:?}, changed index {i} by {d}, error: {current_score}"
                );
                let mut output_file = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open("best_parameters.txt")
                    .unwrap();
                output_file
                    .write_all(format!("{params:?}").as_bytes())
                    .unwrap();
            } else {
                let mut output_file = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open("best_parameters.txt")
                    .unwrap();
                output_file
                    .write_all(format!("{params:?}").as_bytes())
                    .unwrap();
                println!("Local minimum found, best params: {params:?}");
                break;
            }
        }
    }
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
    let mut count = 0;
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
            count += 1;
            print!("\rConverting PGN to fen list: {}", count);
            std::io::stdout().flush()?;
            for fen in fen_list {
                data_file.write_all(fen.as_bytes())?;
                data_file.write_all(b"\n")?;
            }
        }
        println!();
    }
    if !args.skip_tuning {
        initialise_zobrist_numbers();
        LookupTables::generate_all(true);

        let eval_params = EVAL_PARAMS;
        let mut params = eval_params.to_vec();
        params[5] = 0;
        let eval_params = EvalParams::from_params(&params);

        let k = 1.0;

        let mut data_file = OpenOptions::new().read(true).open(data_path)?;
        let mut data = String::new();
        data_file.read_to_string(&mut data)?;
        let len = data.lines().count();
        let data = data
            .lines()
            .step_by(len / args.count)
            .fold(String::new(), |a, b| a + b + "\n");

        let data = Cow::from(data);

        let cost = OptimizeSigmoidScale {
            data: data.clone(),
            eval_params,
            count: (args.count / 100).max(1),
        };
        let linesearch = MoreThuenteLineSearch::new();
        let solver = SteepestDescent::new(linesearch);

        let best_k = Executor::new(cost, solver, k)
            .add_observer(ArgminSlogLogger::term(), ObserverMode::Always)
            .max_iters(10)
            .run()?
            .state()
            .best_param;

        println!("best K: {best_k}");

        let cost = TexelTuning {
            data: data.clone(),
            k: best_k,
            count: args.count,
        };

        let mut best_params = eval_params.to_vec();
        cost.local_search(&mut best_params);
        println!("Best params: {best_params:?}");
    }
    Ok(())
}
