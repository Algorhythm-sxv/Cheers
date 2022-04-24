use cheers_lib::*;

use std::{
    error::Error,
    fs::File,
    io::{prelude::*, stdin},
    sync::atomic::Ordering,
    thread,
    time::{Duration, Instant},
};

use chessgame::{ChessGame, NODE_COUNT, NPS_COUNT, RUN_SEARCH};
use moves::Move;
use transposition_table::{TranspositionTable, TT_DEFAULT_SIZE};

use crate::lookup_tables::*;

fn main() -> Result<(), Box<dyn Error>> {
    LookupTables::generate_all(true);
    zobrist::initialise_zobrist_numbers();

    let tt = TranspositionTable::new(TT_DEFAULT_SIZE);
    let mut position = ChessGame::new(tt.clone());

    for line in stdin().lock().lines() {
        let line = line?;

        let words = line.split(' ').collect::<Vec<_>>();

        match words.get(0) {
            Some(&"uci") => {
                println!("id name cheers");
                println!("id author Algorhythm");
                println!("option name Hash type spin default 64");
                println!("uciok");
            }
            Some(&"quit") => break,
            Some(&"isready") => {
                println!("readyok");
            }
            Some(&"position") => {
                let moves_index;
                match words.get(1) {
                    Some(&"fen") => {
                        let mut test_boards = ChessGame::new(tt.clone());
                        let fen = words[2..=7].join(" ");
                        if let Err(err) = test_boards.set_from_fen(fen.clone()) {
                            println!("Failed to set board with FEN {}: {}", fen, err)
                        }
                        // position is valid
                        position.set_from_fen(fen)?;
                        moves_index = 9
                    }
                    Some(&"startpos") => {
                        position.reset();
                        if let Some(word) = words.get(2) {
                            if word != &"moves" {
                                println!("Malformed UCI command: no \'moves\' in position command");
                                continue;
                            };
                        }
                        moves_index = 3
                    }
                    _ => unreachable!(),
                }
                if words.get(moves_index).is_some() {
                    words[moves_index..].iter().for_each(|xy| {
                        let move_ = Move::from_pair(&position, xy);
                        position.make_move(move_)
                    });
                }
            }
            Some(&"go") => {
                if words.get(1) == Some(&"perft") {
                    let depth = match words.get(2) {
                        None => 5,
                        Some(num) => num.parse::<usize>()?,
                    };
                    let start = Instant::now();
                    let nodes = position.perft(depth);
                    let end = Instant::now();
                    let time = (end - start).as_secs_f32();
                    let nps = nodes as f32 / time;
                    println!("Perft({depth}): {nodes}\t\t{time}s\t\t{nps:.1}nps");
                } else {
                    let infinite = words.iter().any(|&w| w == "infinite");
                    let depth = words
                        .iter()
                        .enumerate()
                        .skip_while(|(_, &w)| w != "depth")
                        .nth(1);
                    let depth = match depth {
                        Some((i, w)) => match w.parse::<usize>() {
                            Ok(n) => Some(n),
                            _ => {
                                println!("Invalid value for depth: {}", words[i]);
                                continue;
                            }
                        },
                        None => None,
                    };
                    let wtime = words
                        .iter()
                        .enumerate()
                        .skip_while(|(_, &w)| w != "wtime")
                        .nth(1);
                    let wtime = match wtime {
                        Some((i, w)) => match w.parse::<usize>() {
                            Ok(n) => Some(n),
                            _ => {
                                println!("Invalid value for wtime: {}", words[i]);
                                continue;
                            }
                        },
                        None => None,
                    };
                    let btime = words
                        .iter()
                        .enumerate()
                        .skip_while(|(_, &w)| w != "btime")
                        .nth(1);
                    let btime = match btime {
                        Some((i, w)) => match w.parse::<usize>() {
                            Ok(n) => Some(n),
                            _ => {
                                println!("Invalid value for btime: {}", words[i]);
                                continue;
                            }
                        },
                        None => None,
                    };
                    let params = SearchParams {
                        position: position.clone(),
                        depth,
                        wtime,
                        btime,
                        infinite,
                    };
                    RUN_SEARCH.store(true, Ordering::Relaxed);
                    NODE_COUNT.store(0, Ordering::Relaxed);
                    NPS_COUNT.store(0, Ordering::Relaxed);
                    let _ = thread::spawn(move || engine_thread(params).unwrap());
                }
            }
            Some(&"stop") => RUN_SEARCH.store(false, Ordering::Relaxed),
            Some(&"setoption") => {
                let option_name = words
                    .iter()
                    .position(|&w| w == "name")
                    .map(|i| words.get(i + 1).map(|w| w.to_lowercase()))
                    .flatten();

                if let Some(option) = option_name {
                    match option.as_ref() {
                        "hash" => {
                            let option_value = words
                                .iter()
                                .position(|&w| w == "value")
                                .map(|i| words.get(i + 1).map(|w| w.parse::<usize>().ok()))
                                .flatten()
                                .flatten();
                            if let Some(val) = option_value {
                                tt.set_size(val);
                            } else {
                                println!("Invalid value for hash table size");
                            }
                        }
                        other => {
                            println!("Unrecognised engine option: {other}")
                        }
                    }
                }
            }
            Some(&"magics") => {
                let start = Instant::now();
                LookupTables::generate_all(false);
                let end = Instant::now();
                println!("Generated magics in {:.3}s", (end - start).as_secs_f32());
                unsafe { LOOKUP_TABLES.print_magics() };
            }
            Some(&"test") => {
                let mut test_suite = String::new();
                File::open("src/perftsuite.txt")?.read_to_string(&mut test_suite)?;
                for depth in 1..=6 {
                    for test in test_suite.split('\n') {
                        let mut test_params = test.split(';');

                        let test_fen = test_params.next().unwrap().to_string();
                        let mut boards = ChessGame::new(tt.clone());
                        boards.set_from_fen(test_fen.clone())?;
                        let answer = test_params
                            .nth(depth - 1)
                            .unwrap()
                            .split(' ')
                            .nth(1)
                            .unwrap()
                            .trim()
                            .parse::<usize>()
                            .unwrap();

                        if boards.perft(depth) != answer {
                            println!("Test for fen {} failed at depth {}", test_fen, depth);
                        } else if depth > 5 {
                            println!(
                                "Perft {} completed successfully for FEN {}",
                                depth, test_fen
                            );
                        }
                    }
                    println!("Perft tests completed at depth {}", depth);
                }
            }
            Some(&"divide") => {
                let depth = match words.get(1) {
                    None => 5,
                    Some(num) => num.parse::<usize>()?,
                };
                position.divide(depth);
            }
            _ => println!("unknown command: {}", line),
        }
    }
    Ok(())
}

struct SearchParams {
    position: ChessGame,
    depth: Option<usize>,
    wtime: Option<usize>,
    btime: Option<usize>,
    infinite: bool,
}

fn engine_thread(search_params: SearchParams) -> Result<(), Box<dyn Error>> {
    let boards = search_params.position;
    let current_player = boards.current_player();
    let max_depth = search_params.depth;
    // spawn another thread to do the actual searching
    thread::spawn(move || {
        let (score, best_move) = boards.search(max_depth);
        println!("info score cp {score}");
        println!("bestmove {}", best_move.coords(),);
    });

    let search_start = Instant::now();
    let mut nodes_report = Instant::now();
    while chessgame::RUN_SEARCH.load(Ordering::Relaxed) {
        let node_report_time = Instant::now().duration_since(nodes_report);
        if node_report_time > Duration::from_millis(500) {
            nodes_report = Instant::now();
            let nodes = NODE_COUNT.load(Ordering::Relaxed);
            let nps = (NPS_COUNT.swap(0, Ordering::Relaxed) as f32 / node_report_time.as_secs_f32())
                as usize;
            println!("info nodes {nodes} nps {nps}");
        }
        if !search_params.infinite {
            let search_time = match current_player {
                types::ColorIndex::White => search_params.wtime.map(move_time),
                types::ColorIndex::Black => search_params.btime.map(move_time),
            };

            if let Some(d) = search_time {
                if Instant::now().duration_since(search_start) > d {
                    RUN_SEARCH.store(false, Ordering::Relaxed);
                }
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn move_time(msec: usize) -> Duration {
    Duration::from_millis(msec as u64 / 15)
}
