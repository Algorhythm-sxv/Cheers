use cheers_lib::{
    chessgame::ChessGame,
    hash_tables::TranspositionTable,
    moves::Move,
    search::{
        EngineOptions, Search, ABORT_SEARCH, NODE_COUNT, NPS_COUNT, SEARCH_COMPLETE, TIME_ELAPSED,
    },
    types::ColorIndex,
};

use std::{
    error::Error,
    fs::File,
    io::{prelude::*, stdin},
    path::PathBuf,
    sync::{atomic::Ordering, Arc, RwLock},
    thread,
    time::Instant,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut position = ChessGame::new();
    let mut options = EngineOptions::default();

    let mut tt = Arc::new(RwLock::new(TranspositionTable::new(options.tt_size_mb)));

    if std::env::args().nth(1) == Some(String::from("bench")) {
        let bench_game = position.clone();
        let search = Search::new(bench_game)
            .max_depth(Some(12))
            .tt_size_mb(8)
            .output(false);
        let start = Instant::now();
        search.search();
        let end = Instant::now();
        let time = end - start;

        let nodes = NODE_COUNT.load(Ordering::Relaxed);
        let nps = (nodes as f64 / time.as_secs_f64()) as usize;
        println!("{nodes} nodes {nps} nps");
        return Ok(());
    }

    for line in stdin().lock().lines() {
        let line = line?;

        let words = line.split(' ').collect::<Vec<_>>();

        match words.get(0) {
            Some(&"uci") => {
                println!("id name cheers");
                println!("id author Algorhythm");
                println!("option name Hash type spin default 8 min 1 max 32768");
                println!("option name Threads type spin default 1 min 1 max 1");
                println!("uciok");
            }
            Some(&"ucinewgame") => {
                position.reset();
                tt = Arc::new(RwLock::new(TranspositionTable::new(options.tt_size_mb)));
            }
            Some(&"quit") => break,
            Some(&"isready") => {
                println!("readyok");
            }
            Some(&"position") => {
                let moves_index = match words.get(1) {
                    Some(&"fen") => {
                        let mut test_boards = ChessGame::new();
                        let fen = words[2..=7].join(" ");
                        if let Err(err) = test_boards.set_from_fen(fen.clone()) {
                            println!("Failed to set board with FEN {}: {}", fen, err)
                        }
                        // position is valid
                        position.set_from_fen(fen)?;
                        9
                    }
                    Some(&"startpos") => {
                        position.reset();
                        if let Some(word) = words.get(2) {
                            if word != &"moves" {
                                println!("Malformed UCI command: no \'moves\' in position command");
                                continue;
                            };
                        }
                        3
                    }
                    _ => unreachable!(),
                };
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
                    let nodes = words
                        .iter()
                        .enumerate()
                        .skip_while(|(_, &w)| w != "nodes")
                        .nth(1);
                    let nodes = match nodes {
                        Some((i, w)) => match w.parse::<usize>() {
                            Ok(n) => Some(n),
                            _ => {
                                println!("Invalid value for nodes: {}", words[i]);
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

                    let winc = words
                        .iter()
                        .enumerate()
                        .skip_while(|(_, &w)| w != "winc")
                        .nth(1);
                    let winc = match winc {
                        Some((i, w)) => match w.parse::<usize>() {
                            Ok(n) => Some(n),
                            _ => {
                                println!("Invalid value for winc: {}", words[i]);
                                continue;
                            }
                        },
                        None => None,
                    };

                    let binc = words
                        .iter()
                        .enumerate()
                        .skip_while(|(_, &w)| w != "binc")
                        .nth(1);
                    let binc = match binc {
                        Some((i, w)) => match w.parse::<usize>() {
                            Ok(n) => Some(n),
                            _ => {
                                println!("Invalid value for binc: {}", words[i]);
                                continue;
                            }
                        },
                        None => None,
                    };

                    let movetime = words
                        .iter()
                        .enumerate()
                        .skip_while(|(_, &w)| w != "movetime")
                        .nth(1);
                    let movetime = match movetime {
                        Some((i, w)) => match w.parse::<usize>() {
                            Ok(n) => Some(n),
                            _ => {
                                println!("Invalid value for movetime: {}", words[i]);
                                continue;
                            }
                        },
                        None => None,
                    };
                    let mut search = Search::new_with_tt(position.clone(), tt.clone())
                        .max_depth(depth)
                        .max_nodes(nodes)
                        .tt_size_mb(options.tt_size_mb)
                        .output(true)
                        .options(options);
                    match position.current_player() {
                        ColorIndex::White => {
                            search.max_time_ms = move_time(wtime, winc);
                        }
                        ColorIndex::Black => {
                            search.max_time_ms = move_time(btime, binc);
                        }
                    };
                    if let Some(movetime) = movetime {
                        search.max_time_ms = Some((movetime, movetime));
                    }

                    let _ = thread::spawn(move || engine_thread(search).unwrap());
                }
            }
            Some(&"stop") => ABORT_SEARCH.store(true, Ordering::Relaxed),
            Some(&"setoption") => {
                let option_name = words
                    .iter()
                    .position(|&w| w == "name")
                    .and_then(|i| words.get(i + 1).map(|w| w.to_lowercase()));

                if let Some(option) = option_name {
                    match option.as_ref() {
                        "hash" => {
                            let option_value = words
                                .iter()
                                .position(|&w| w == "value")
                                .and_then(|i| words.get(i + 1).map(|w| w.parse::<usize>().ok()))
                                .flatten();
                            if let Some(val) = option_value {
                                options.tt_size_mb = val
                            } else {
                                println!("Invalid value for hash table size");
                            }
                        }
                        "threads" => {}
                        other => {
                            println!("Unrecognised engine option: {other}")
                        }
                    }
                }
            }
            Some(&"test") => {
                let path = match words.get(1) {
                    Some(p) => match PathBuf::try_from(p) {
                        Ok(path) => path,
                        Err(_) => {
                            println!("Invalid perft test file path: {p}");
                            continue;
                        }
                    },
                    None => {
                        println!("No path given for perft test file");
                        continue;
                    }
                };
                let mut test_suite = String::new();
                File::open(path)?.read_to_string(&mut test_suite)?;
                for depth in 1..=6 {
                    for test in test_suite.split('\n') {
                        let mut test_params = test.split(';');

                        let test_fen = test_params.next().unwrap().to_string();
                        let mut boards = ChessGame::new();
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
            Some(&"fen") => {
                println!("{}", position.fen());
            }
            _ => println!("unknown command: {}", line),
        }
    }
    Ok(())
}

fn engine_thread(search: Search) -> Result<(), Box<dyn Error>> {
    ABORT_SEARCH.store(false, Ordering::Relaxed);
    TIME_ELAPSED.store(false, Ordering::Relaxed);
    SEARCH_COMPLETE.store(false, Ordering::Relaxed);
    NODE_COUNT.store(0, Ordering::Relaxed);
    NPS_COUNT.store(0, Ordering::Relaxed);

    let (_, pv) = search.search();

    println!("bestmove {}", pv.moves[0].coords());

    Ok(())
}

fn move_time(time_millis: Option<usize>, inc_millis: Option<usize>) -> Option<(usize, usize)> {
    let (time, inc) = match (time_millis, inc_millis) {
        (None, None) => return None,
        (t, i) => (t.unwrap_or(0), i.unwrap_or(0)),
    };
    if time < inc {
        Some((time / 20, time / 2))
    } else {
        Some((time / 20 + inc / 2, time / 2))
    }
}
