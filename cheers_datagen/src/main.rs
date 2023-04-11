use std::{
    error::Error,
    fmt::Display,
    fs::OpenOptions,
    io::{stdout, BufRead, BufReader, BufWriter, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Sender},
        Arc, RwLock,
    },
    thread,
    time::Instant,
};

use cheers_lib::{
    board::{evaluate::CHECKMATE_SCORE, Board},
    hash_tables::TranspositionTable,
    moves::MoveList,
    search::Search,
    types::{Color, MainThread},
};
use clap::Parser;
use rand::prelude::*;

#[derive(Parser)]
struct Args {
    /// File to append data to
    #[arg(default_value_t = String::from("test.mf"))]
    output_file: String,

    /// Number of threads to use
    #[arg(short, long, default_value_t = 1)]
    threads: usize,

    /// Number of random plies at the start of each game
    #[arg(short, long, default_value_t = 10)]
    random_plies: usize,

    /// Maximum nodes to search per move
    #[arg(short, long, default_value_t = 5_000)]
    nodes: usize,

    /// Maximum search depth per move
    #[arg(short, long, default_value_t = 120)]
    depth: usize,

    /// Maximum number of data positions to generate
    #[arg(short, long, default_value_t = 10_000_000)]
    count: usize,
}

#[derive(Clone, Debug)]
struct PositionInfo {
    pub fen: String,
    pub eval: i16,
    pub hash: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum GameOutcome {
    WhiteWin,
    BlackWin,
    Draw,
}

impl Display for GameOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let wdl = match self {
            GameOutcome::WhiteWin => "1",
            GameOutcome::BlackWin => "0",
            GameOutcome::Draw => ".5",
        };
        writeln!(f, "{wdl}")
    }
}

static STOP_GENERATION: AtomicBool = AtomicBool::new(false);
fn generator_thread(
    max_nodes: usize,
    max_depth: usize,
    random_plies: usize,
    thread_id: usize,
    sender: Sender<(Vec<PositionInfo>, GameOutcome, usize)>,
) {
    let mut rng = thread_rng();
    let mut game_positions: Vec<PositionInfo> = Vec::new();
    let mut moves = MoveList::new();
    let tt = Arc::new(RwLock::new(TranspositionTable::new(32)));
    'game_start: loop {
        if STOP_GENERATION.load(Ordering::Relaxed) {
            return;
        }
        // set up a board
        // TODO: FRC startpos
        let mut board = Board::new();

        // randomize the exit color
        let extra = rng.gen_range(0..=1);
        // play random plies
        'outer: loop {
            for _ in 0..(random_plies + extra) {
                board.generate_legal_moves_into(&mut moves);
                let mv = match moves.inner().choose(&mut rng) {
                    // game end within start plies, try again
                    None => {
                        board = Board::new();
                        continue 'outer;
                    }
                    Some(smv) => smv.mv,
                };
                board.make_move(mv);
            }
            break;
        }

        let mut start_check = false;
        // play the board until game end
        let mut movenumber = random_plies / 2;
        let outcome;
        let mut win_adjudication_counter = 0;
        let mut draw_adjudication_counter = 0;
        game_positions.clear();
        loop {
            // detect some immediate repetitions
            if game_positions.len() >= 4 {
                if game_positions[game_positions.len() - 2].hash == board.hash()
                    || game_positions[game_positions.len() - 4].hash == board.hash()
                {
                    outcome = GameOutcome::Draw;
                    break;
                }
            }
            // detect stale/checkmate before search
            board.generate_legal_moves_into(&mut moves);
            if moves.len() == 0 {
                outcome = if board.in_check() {
                    if board.current_player() == Color::White {
                        GameOutcome::BlackWin
                    } else {
                        GameOutcome::WhiteWin
                    }
                } else {
                    GameOutcome::Draw
                };
                break;
            }

            // perform the search
            let search = Search::new_with_tt(board, tt.clone(), 0)
                .max_nodes(Some(max_nodes))
                .max_depth(Some(max_depth));
            let (score, pv) = search.search::<MainThread>(false);
            movenumber += (board.current_player() == Color::Black) as usize;
            // discard high-bias exits at game start
            if !start_check && score.abs() > 1000 {
                continue 'game_start;
            }
            start_check = true;

            // mate detected
            if CHECKMATE_SCORE - score.abs() < 128 {
                if board.current_player() == Color::White {
                    outcome = if score > 0 {
                        GameOutcome::WhiteWin
                    } else {
                        GameOutcome::BlackWin
                    };
                } else {
                    outcome = if score > 0 {
                        GameOutcome::BlackWin
                    } else {
                        GameOutcome::WhiteWin
                    };
                };
                break;
            }
            let score = if board.current_player() == Color::Black {
                -score
            } else {
                score
            };
            // draw adjudication
            if movenumber >= 40 - random_plies / 2 {
                if score.abs() < 20 {
                    draw_adjudication_counter += 1;
                } else {
                    draw_adjudication_counter = 0;
                }
            }
            // 20 move rule, drawn evals and likely material draws
            if board.halfmove_clock() >= 40
                || draw_adjudication_counter >= 8
                || (board.pawn_count(Color::White) == board.pawn_count(Color::Black)
                    && board.non_pawn_piece_count(Color::White) <= 2
                    && board.non_pawn_piece_count(Color::Black) <= 2)
            {
                outcome = GameOutcome::Draw;
                break;
            }

            // win adjudication
            if score.abs() >= 2000 {
                win_adjudication_counter += 1;
            } else {
                win_adjudication_counter = 0;
            }
            if win_adjudication_counter >= 4 {
                outcome = if score >= 0 {
                    GameOutcome::WhiteWin
                } else {
                    GameOutcome::BlackWin
                };
                break;
            }
            // play the best move
            if *pv[0].from == 0 && *pv[0].to == 0 {
                panic!("null move in {}", board.fen());
            }
            board.make_move(pv[0]);

            // discard 5 moves before/including checks
            if board.in_check() {
                game_positions.truncate(game_positions.len().saturating_sub(4));
                continue;
            // discard positions with less than 5 non-pawns on the board
            } else if board.non_pawn_piece_count(Color::White)
                + board.non_pawn_piece_count(Color::Black)
                <= 4
            {
                continue;
            } else {
                // add the position to the record
                game_positions.push(PositionInfo {
                    fen: board.fen(),
                    eval: score,
                    hash: board.hash(),
                });
            }
        }

        // send the game data to the main thread
        if game_positions.len() > 0 {
            sender
                .send((game_positions.clone(), outcome, thread_id))
                .expect("Failed to send game data!");
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let existing_data = OpenOptions::new().read(true).open(&args.output_file);
    let mut positions = if let Ok(ref file) = existing_data {
        let reader = BufReader::new(file);
        reader.lines().count()
    } else {
        0usize
    };
    // close the file if it exists
    std::mem::drop(existing_data);

    let mut output_file = BufWriter::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&args.output_file)?,
    );

    let start = Instant::now();
    // start the generator threads
    let (sender, receiver) = channel();
    let mut thread_handles = Vec::new();
    for i in 0..args.threads {
        let sender = sender.clone();
        thread_handles.push(thread::spawn(move || {
            generator_thread(args.nodes, args.depth, args.random_plies, i, sender);
        }));
    }

    let mut games = 0usize;
    let mut white_wins = 0usize;
    let mut black_wins = 0usize;
    let mut draws = 0usize;
    while positions < args.count {
        let (game_positions, outcome, thread_id) = receiver.recv()?;

        games += 1;
        positions += game_positions.len();
        match outcome {
            GameOutcome::WhiteWin => white_wins += 1,
            GameOutcome::BlackWin => black_wins += 1,
            GameOutcome::Draw => draws += 1,
        }

        // write positions to file
        for pos in game_positions.iter() {
            write!(output_file, "{}|{}|{}", pos.fen, pos.eval, outcome)?;
        }

        // clear the terminal line and report current stats
        let time = Instant::now().duration_since(start).as_millis() as usize;
        print!("\x1B[2K\r");
        print!(
            "Last thread: {thread_id:<3} Positions: {:<15} Positions/s: {:<15} Games: {:<15} Games/s: {:<15} White score: {}-{}-{}\r",
            positions,
            (positions * 1000) / time,
            games,
            (games as f32 * 1000.0) / time as f32,
            white_wins,
            black_wins,
            draws
        );
        stdout().flush()?;
        // for pos in game_positions.iter() {
        //     println!("{}|{}|{}", pos.fen, pos.eval, outcome);
        // }
    }
    STOP_GENERATION.store(true, Ordering::Relaxed);
    for thread in thread_handles {
        let _ = thread.join();
    }
    println!("");
    Ok(())
}
