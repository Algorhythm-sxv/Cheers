use cheers_lib::{
    chessgame::ChessGame,
    hash_tables::TranspositionTable,
    search::{
        EngineOptions, Search, ABORT_SEARCH, NODE_COUNT, NPS_COUNT, SEARCH_COMPLETE, TIME_ELAPSED,
    },
    types::ColorIndex,
};

use std::{
    error::Error,
    io::{prelude::*, stdin},
    sync::{atomic::Ordering, Arc, RwLock},
    thread,
    time::Instant,
};

mod uci;

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
        let cmd = match uci::parse_uci_command(line?) {
            Ok(cmd) => cmd,
            Err(uci::UciParseError::Empty) => {
                continue;
            }
            Err(uci::UciParseError::Other(e)) => {
                println!("{e}");
                continue;
            }
        };

        match cmd {
            uci::UciCommand::Uci => {
                println!("id name Cheers");
                println!("id author Algorhythm");
                uci::print_uci_options();
                println!("uciok");
            }
            uci::UciCommand::IsReady => println!("readyok"),
            uci::UciCommand::SetOption(opt) => match opt {
                uci::UciOption::Hash(mb) => {
                    options.tt_size_mb = mb;
                    tt.write().unwrap().set_size(mb);
                }
                _ => {}
            },
            uci::UciCommand::UciNewGame => {
                position.reset();
                tt = Arc::new(RwLock::new(TranspositionTable::new(options.tt_size_mb)));
            }
            uci::UciCommand::Position { fen, moves } => {
                match fen {
                    Some(fen) => position.set_from_fen(fen).unwrap(),
                    None => position.reset(),
                }
                for m in moves {
                    position.make_move(m);
                }
            }
            uci::UciCommand::Go {
                wtime,
                btime,
                winc,
                binc,
                movestogo,
                depth,
                nodes,
                movetime,
                infinite,
                perft,
            } => {
                if let Some(depth) = perft {
                    position.divide(depth);
                    continue;
                }
                let movetime = if infinite {
                    None
                } else {
                    match movetime {
                        Some(time) => Some((time, time)),
                        None => match movestogo {
                            Some(n) => {
                                if position.current_player() == ColorIndex::White {
                                    Some((wtime.unwrap() / n, wtime.unwrap() / n))
                                } else {
                                    Some((btime.unwrap() / n, btime.unwrap() / n))
                                }
                            }
                            None => {
                                if position.current_player() == ColorIndex::White {
                                    move_time(wtime, winc)
                                } else {
                                    move_time(btime, binc)
                                }
                            }
                        },
                    }
                };

                let mut search = Search::new_with_tt(position.clone(), tt.clone())
                    .tt_size_mb(options.tt_size_mb)
                    .max_nodes(nodes)
                    .max_depth(depth)
                    .options(options)
                    .output(true);
                search.max_time_ms = movetime;

                let _ = thread::spawn(move || engine_thread(search).unwrap());
            }
            uci::UciCommand::Stop => ABORT_SEARCH.store(true, Ordering::Relaxed),
            uci::UciCommand::Quit => {
                ABORT_SEARCH.store(true, Ordering::Relaxed);
                break;
            }
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
