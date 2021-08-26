use std::error::Error;
use std::io::{prelude::*, stdin};
use std::sync::mpsc::*;
use std::thread;

mod bitboard;
mod evaluate;
mod lookup_tables;
mod piece_tables;
mod search;
mod transposition_table;
mod types;
mod utils;
mod zobrist;

use bitboard::*;
use lookup_tables::*;
use types::*;

use crate::zobrist::{zobrist_hash, zobrist_numbers};

enum EngineMessage {
    Move(Move),
    Moves(Vec<Move>),
    Reset,
    Fen(String),
    Start,
    Stop,
}

fn engine_thread(
    tx: Sender<EngineMessage>,
    rx: Receiver<EngineMessage>,
) -> Result<(), Box<dyn Error>> {
    use rayon::ThreadPoolBuilder;
    use EngineMessage::*;

    let _luts = LookupTables::generate_all();
    // TODO: configurable hash table size
    // 1<<23 entries corresponds with ~140MB
    let mut bitboards = BitBoards::new((1 << 23) + 9);

    ThreadPoolBuilder::new().num_threads(0).build_global()?;
    while let Ok(msg) = rx.recv() {
        match msg {
            Move(next_move) => {
                bitboards.make_move(&next_move);
            }
            Start => {
                // let moves = bitboards.generate_legal_moves();

                // if let Some(choice) = moves.choose(&mut thread_rng()) {
                //     bitboards.make_move(choice);
                //     tx.send(Move(*choice))?;
                // }
                let (_score, best_move) = bitboards.toplevel_search(i32::MIN + 1, i32::MAX - 1, 8);

                tx.send(EngineMessage::Move(best_move))?;
            }
            Stop => break,
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
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let (tx, thread_rx) = channel();
    let (thread_tx, rx) = channel();

    thread::spawn(|| {
        engine_thread(thread_tx, thread_rx).unwrap();
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

                tx.send(EngineMessage::Start)?;
                let msg = rx.recv()?;
                match msg {
                    EngineMessage::Move(move_) => {
                        println!("bestmove {}", move_.to_algebraic_notation());
                    }
                    _ => (),
                }
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
