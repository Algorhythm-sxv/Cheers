#![allow(dead_code)]

use std::error::Error;
use std::io::{prelude::*, stdin};
use std::sync::mpsc::*;
use std::thread;

use rand::prelude::*;

mod bitboard;
mod lookup_tables;
mod types;
mod utils;

use bitboard::*;
use lookup_tables::*;
use types::*;

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
    use EngineMessage::*;

    let _luts = LookupTables::generate_all();
    let mut bitboards = BitBoards::new();

    while let Ok(msg) = rx.recv() {
        match msg {
            Move(next_move) => {
                bitboards.make_move(&next_move);
            }
            Start => {
                let moves = bitboards.generate_legal_moves();

                if let Some(choice) = moves.choose(&mut thread_rng()) {
                    bitboards.make_move(choice);
                    tx.send(Move(*choice))?;
                }
            }
            Stop => break,
            Reset => {
                bitboards.reset();
            }
            Fen(fen) => bitboards.set_from_fen(fen),
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
                println!("readyok");
            }
            Some(&"position") => {
                let moves_index;
                match words.get(1) {
                    Some(&"fen") => {
                        tx.send(EngineMessage::Fen(words[2].to_string()))?;
                        moves_index = 4
                    }
                    Some(&"startpos") => {
                        tx.send(EngineMessage::Reset)?;
                        if let Some(word) = words.get(2) {
                            if word != &"moves" {
                                println!("Malformed UCI command: no \'moves\' in position command");
                                continue
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
                        println!("bestmove {}", move_.to_algebraic_notation())
                    }
                    _ => (),
                }
            }
            Some(&"gen") => {
                let mut bitboards = BitBoards::new();
                let _ = LookupTables::generate_all();

                bitboards.make_move(&Move::new(12, 28, None));
                bitboards.make_move(&Move::new(52, 36, None));
                bitboards.make_move(&Move::new(5, 24, None));
                bitboards.make_move(&Move::new(51, 35, None));
                bitboards.make_move(&Move::new(6, 21, None));
                bitboards.make_move(&Move::new(50, 34, None));
                bitboards.make_move(&Move::new(4, 6, None));
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
