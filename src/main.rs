#![allow(dead_code)]

use std::error::Error;
use std::io::{prelude::*, stdin};
use std::sync::mpsc::*;
use std::thread;

use rand::prelude::*;

mod bitboard;
mod lookup_tables;

use bitboard::*;
use lookup_tables::*;

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

    let luts = LookupTables::generate_all();
    let mut bitboards = BitBoards::new();

    while let Ok(msg) = rx.recv() {
        match msg {
            Move(next_move) => {
                bitboards.make_move(&next_move);
            }
            Start => {
                let moves = bitboards.generate_pseudolegal_moves();
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
        let _ = engine_thread(thread_tx, thread_rx);
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
                let moves = bitboards.generate_pseudolegal_moves();
                for i in moves.iter() {
                    println!(
                        "{} -> {}",
                        square_to_coord(i.start),
                        square_to_coord(i.target)
                    );
                }
                println!("number: {}", moves.len());
                if let Some(choice) = moves.choose(&mut thread_rng()) {
                    println!(
                        "chose {} -> {}",
                        square_to_coord(choice.start),
                        square_to_coord(choice.target)
                    );
                    bitboards.make_move(choice);
                }
                let moves = bitboards.generate_pseudolegal_moves();
                for i in moves.iter() {
                    println!(
                        "{} -> {}",
                        square_to_coord(i.start),
                        square_to_coord(i.target)
                    );
                }
                println!("number: {}", moves.len());
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
