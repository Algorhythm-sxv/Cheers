use std::error::Error;
use std::io::{prelude::*, stdin};
use std::sync::{atomic::Ordering::*, mpsc::*};
use std::thread;
use std::time::{Duration, Instant};

mod bitboard;
mod evaluate;
mod lookup_tables;
mod piece_tables;
mod search;
mod time_management;
mod transposition_table;
mod types;
mod utils;
mod zobrist;

use bitboard::*;
use lookup_tables::*;
use search::{NODE_COUNT, NPS_COUNT, RUN_SEARCH};
use time_management::time_for_move;
use types::*;

use crate::zobrist::{zobrist_hash, zobrist_numbers};

enum EngineMessage {
    Moves(Vec<Move>),
    Reset,
    Fen(String),
    // wtime, btime, infinite, depth
    Start(usize, usize, bool, Option<usize>),
    Stop,
    Quit,
}

fn engine_thread(rx: Receiver<EngineMessage>) -> Result<(), Box<dyn Error>> {
    use rayon::ThreadPoolBuilder;
    use EngineMessage::*;

    let _luts = LookupTables::generate_all();
    // TODO: configurable hash table size
    // 1<<23 entries corresponds with ~140MB
    let mut bitboards = BitBoards::new((1 << 23) + 9);
    let mut max_depth = None;

    let mut best_move = Move::null();
    let mut best_depth = 0;
    let mut start_time = Instant::now();
    let mut last_nps_time = Instant::now();
    let mut max_elapsed_time = 0;
    let mut infinite_search = false;

    ThreadPoolBuilder::new().num_threads(0).build_global()?;
    let (move_tx, move_rx) = sync_channel(30);
    loop {
        if let Ok(msg) = rx.recv_timeout(Duration::from_millis(100)) {
            match msg {
                Start(wtime, btime, infinite, depth) => {
                    if !infinite {
                        infinite_search = false;
                        max_elapsed_time = time_for_move(match bitboards.current_player {
                            White => wtime,
                            Black => btime,
                        });
                    } else {
                        infinite_search = true;
                    }
                    max_depth = depth;

                    best_move = Move::null();
                    best_depth = 0;
                    start_time = Instant::now();
                    last_nps_time = Instant::now();
                    NODE_COUNT.store(0, SeqCst);
                    NPS_COUNT.store(0, SeqCst);

                    bitboards.toplevel_search(i32::MIN + 1, i32::MAX - 1, move_tx.clone());
                }
                Stop => {
                    println!(
                        "bestmove {}\nFound after {:.3}s",
                        best_move.to_algebraic_notation(),
                        (Instant::now() - start_time).as_secs_f32()
                    );
                    RUN_SEARCH.store(false, Relaxed);
                    // clear the channel
                    while let Ok(_) = move_rx.recv_timeout(Duration::from_millis(100)) {}
                }
                Quit => break,
                Reset => {
                    bitboards.reset();
                }
                Fen(fen) => bitboards.set_from_fen(fen).unwrap(),
                Moves(moves) => {
                    for move_ in &moves {
                        bitboards.make_move(move_);
                    }
                }
            }
        }
        while let Ok((score, move_, depth)) = move_rx.recv_timeout(Duration::from_millis(100)) {
            if RUN_SEARCH.load(Relaxed) {
                println!(
                    "info depth {} score cp {}",
                    depth,
                    score * (-1 * ((bitboards.current_player as i32 + depth as i32) % 2))
                );
                if depth > best_depth {
                    best_depth = depth;
                    // best_score = score;
                    best_move = move_;
                }
                if Some(depth) == max_depth {
                    println!(
                        "bestmove {}\nFound after {:.3}s",
                        best_move.to_algebraic_notation(),
                        (Instant::now() - start_time).as_secs_f32()
                    );
                    RUN_SEARCH.store(false, Relaxed);
                    // clear the channel
                    while let Ok(_) = move_rx.recv_timeout(Duration::from_millis(100)) {}
                }
            }
        }
        // report nodes and nps
        if RUN_SEARCH.load(Relaxed) {
            println!("info nodes {}", NODE_COUNT.load(Relaxed));
            println!(
                "info nps {}",
                (NPS_COUNT.swap(0, Relaxed) as f32 - (Instant::now() - last_nps_time).as_secs_f32())
                    as usize
            );
            last_nps_time = Instant::now();
        }

        // stop search after too long
        if !infinite_search
            && RUN_SEARCH.load(Relaxed)
            && (Instant::now() - start_time).as_millis() as usize >= max_elapsed_time
        {
            println!(
                "bestmove {}\nFound after {:.3}s",
                best_move.to_algebraic_notation(),
                (Instant::now() - start_time).as_secs_f32()
            );
            RUN_SEARCH.store(false, Relaxed);
            // clear the channel of incomplete results
            while let Ok(_) = move_rx.recv_timeout(Duration::from_millis(100)) {}
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let (tx, thread_rx) = channel();
    let (_thread_tx, rx) = channel::<EngineMessage>();

    thread::spawn(|| {
        engine_thread(thread_rx).unwrap();
    });

    for line_res in stdin().lock().lines() {
        let line = line_res?;

        let words = line.split(' ').collect::<Vec<_>>();

        match words.get(0) {
            Some(&"uci") => {
                println!("id name ches");
                println!("id author Algorhythm");
                println!("uciok");
            }
            Some(&"quit") => {
                tx.send(EngineMessage::Quit)?;
                break;
            }
            Some(&"isready") => {
                // generate lookup tables and zobrist numbers
                let _ = LookupTables::generate_all();
                let _ = zobrist_numbers();
                println!("readyok");
            }
            Some(&"position") => {
                let moves_index;
                match words.get(1) {
                    Some(&"fen") => {
                        let mut test_boards = BitBoards::new(0);
                        test_boards.set_from_fen(words[2..=7].join(" "))?;

                        tx.send(EngineMessage::Fen(words[2..=7].join(" ")))?;
                        moves_index = 9
                    }
                    Some(&"startpos") => {
                        tx.send(EngineMessage::Reset)?;
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
                if let Some(_) = words.get(moves_index) {
                    tx.send(EngineMessage::Moves(
                        words[moves_index..]
                            .iter()
                            .map(|xy| parse_move_pair(xy))
                            .collect(),
                    ))?;
                }
            }
            Some(&"go") => {
                // clear the channel buffer
                while let Ok(_) = rx.try_recv() {}
                if words.iter().any(|&c| c == "infinite") {
                    tx.send(EngineMessage::Start(0, 0, true, None))?;
                } else {
                    let wtime = words.iter().skip_while(|&&c| c != "wtime").nth(1);
                    let btime = words.iter().skip_while(|&&c| c != "btime").nth(1);

                    if wtime.is_some() && btime.is_some() {
                        tx.send(EngineMessage::Start(
                            wtime.unwrap().parse::<usize>().unwrap(),
                            btime.unwrap().parse::<usize>().unwrap(),
                            false,
                            None,
                        ))?;
                    } else if let Some(depth) = words.iter().skip_while(|&&c| c != "depth").nth(1) {
                        tx.send(EngineMessage::Start(
                            0,
                            0,
                            true,
                            Some(depth.parse::<usize>()?),
                        ))?;
                    } else {
                        println!("Incomplete 'go' command, expected one of 'infinite', 'wtime <millis> btime <millis>' or 'depth <depth>'");
                    }
                }
            }
            Some(&"stop") => {
                tx.send(EngineMessage::Stop)?;
            }
            Some(&"perft") => {
                let depth = words
                    .get(1)
                    .ok_or("No depth specified for perft!")?
                    .parse::<usize>()?;
                let nodes = BitBoards::perft(
                    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 0"
                        .to_string(),
                    depth,
                )?;
                println!("Depth {}, {} nodes", depth, nodes);
            }
            Some(&"test") => {
                let mut boards = BitBoards::new(0);
                assert_eq!(boards.position_hash, zobrist_hash(&boards));
                boards.make_move(&parse_move_pair("b1c3"));
                boards.make_move(&parse_move_pair("b8c6"));
                boards.make_move(&parse_move_pair("g1f3"));
                boards.make_move(&parse_move_pair("g8f6"));
                boards.make_move(&parse_move_pair("e2e3"));
                boards.make_move(&parse_move_pair("c6b4"));
                boards.make_move(&parse_move_pair("f1e2"));
                boards.make_move(&parse_move_pair("b4c2"));
                assert_eq!(boards.position_hash, zobrist_hash(&boards));
                boards.make_move(&parse_move_pair("c3a4"));
                assert_eq!(boards.position_hash, zobrist_hash(&boards));
            }
            _ => {
                eprintln!("unknown command: {}", line)
            }
        }
    }
    Ok(())
}

// get 'uci'
// send 'id'-s
// send 'option'-s
// send 'uciok' before timeout

// get 'isready'
// send 'readyok'

// get 'go': start calculating
// get specifier for calculation
// get 'stop': stop calculating
// send 'bestmove'

// get 'quit': exit program
