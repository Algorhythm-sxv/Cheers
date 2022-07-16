use cheers_lib::{
    chessgame::ChessGame,
    moves::Move,
    search::{Search, NODE_COUNT, NPS_COUNT, RUN_SEARCH},
    types::ColorIndex,
};

use std::{
    error::Error,
    fs::File,
    io::{prelude::*, stdin},
    path::PathBuf,
    sync::atomic::Ordering,
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Default)]
struct EngineOptions {
    pub tt_size_mb: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut position = ChessGame::new();
    let mut options = EngineOptions { tt_size_mb: 64 };

    if std::env::args().nth(1) == Some(String::from("bench")) {
        let bench_game = position.clone();
        let search = Search::new(bench_game)
            .max_depth(9)
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
                println!("option name Hash type spin default 64 min 1 max 32768");
                println!("uciok");
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

                    let mut search = Search::new(position.clone())
                        .tt_size_mb(options.tt_size_mb)
                        .output(true);
                    search.max_depth = depth;
                    search.max_time_ms = match position.current_player() {
                        ColorIndex::White => move_time(wtime, winc),
                        ColorIndex::Black => move_time(btime, binc),
                    };
                    RUN_SEARCH.store(true, Ordering::Relaxed);
                    NODE_COUNT.store(0, Ordering::Relaxed);
                    NPS_COUNT.store(0, Ordering::Relaxed);
                    let _ = thread::spawn(move || engine_thread(search).unwrap());
                }
            }
            Some(&"stop") => RUN_SEARCH.store(false, Ordering::Relaxed),
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
    let search_start = Instant::now();
    let max_time_ms = search.max_time_ms.map(|ms| {
        // limit the time of a search with 1 legal move
        if search.game.legal_moves().len() == 1 {
            ms.min(500)
        } else {
            ms
        }
    });
    println!("{max_time_ms:?}");
    // spawn another thread to do the actual searching
    thread::spawn(move || {
        let (score, pv) = search.search();
        println!("info score cp {score} pv {pv}");
        println!("bestmove {}", pv.moves[0].coords(),);
    });

    let mut nodes_report = Instant::now();
    while RUN_SEARCH.load(Ordering::Relaxed) {
        let now = Instant::now();
        let node_report_time = now.duration_since(nodes_report);
        if node_report_time > Duration::from_millis(500) {
            nodes_report = now;
            let nodes = NODE_COUNT.load(Ordering::Relaxed);
            let nps = (NPS_COUNT.swap(0, Ordering::Relaxed) as f32 / node_report_time.as_secs_f32())
                as usize;
            println!("info nodes {nodes} nps {nps}");
        }
        // terminate search after max time elapsed
        if let Some(max_time) = max_time_ms {
            if (now - search_start).as_millis() as usize > max_time {
                RUN_SEARCH.store(false, Ordering::Relaxed);
                break;
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

fn move_time(time_millis: Option<usize>, inc_millis: Option<usize>) -> Option<usize> {
    let (time, inc) = match (time_millis, inc_millis) {
        (None, None) => return None,
        (t, i) => (t.unwrap_or(0), i.unwrap_or(0)),
    };
    if time < inc {
        Some(time / 20)
    } else {
        Some(time / 20 + inc / 2)
    }
}
