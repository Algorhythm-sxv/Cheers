mod bitboards;
mod lookup_tables;
mod moves;
mod types;
mod zobrist;
mod transposition_table;

use std::{
    error::Error,
    fs::File,
    io::{prelude::*, stdin},
    thread,
    time::Instant,
};

use crossbeam::channel::{unbounded, Sender};

use bitboards::BitBoards;
use lookup_tables::lookup_tables;
use moves::Move;
use transposition_table::{TranspositionTable, TT_DEFAULT_SIZE};

use crate::lookup_tables::LookupTables;

fn main() -> Result<(), Box<dyn Error>> {
    let tt = TranspositionTable::new(TT_DEFAULT_SIZE);
    let mut position = BitBoards::new(tt.clone());
    let (tx, rx) = unbounded::<Message>();
    for line in stdin().lock().lines() {
        let line = line?;

        let words = line.split(' ').collect::<Vec<_>>();


        match words.get(0) {
            Some(&"uci") => {
                println!("id name cheers");
                println!("id author Algorhythm");
                println!("uciok");
            }
            Some(&"quit") => break,
            Some(&"isready") => {
                let _ = zobrist::zobrist_numbers();
                let _ = lookup_tables();
                println!("readyok");
            }
            Some(&"position") => {
                let moves_index;
                match words.get(1) {
                    Some(&"fen") => {
                        let mut test_boards = BitBoards::new(tt.clone());
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
                let params = SearchParams {
                    position: position.clone(),
                    depth: None,
                    wtime: None,
                    btime: None,
                    infinite: false,
                };
                let tx = tx.clone();
                let _ = thread::spawn(move || engine_thread(params, tx).unwrap());
            }
            Some(&"magics") => {
                let start = Instant::now();
                LookupTables::generate_all();
                let end = Instant::now();
                println!("Generated magics in {:.3}s", (end - start).as_secs_f32());
                lookup_tables().print_magics();
            }
            Some(&"test") => {
                let mut test_suite = String::new();
                File::open("src/perftsuite.txt")?.read_to_string(&mut test_suite)?;
                for depth in 1..=6 {
                    for test in test_suite.split('\n') {
                        let mut test_params = test.split(';');

                        let test_fen = test_params.next().unwrap().to_string();
                        let mut boards = BitBoards::new(tt.clone());
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
            Some(&"perft") => {
                let depth = match words.get(1) {
                    None => 5,
                    Some(num) => num.parse::<usize>()?,
                };
                for i in 0..=depth {
                    println!("Perft {}: {}", i, position.perft(i))
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
    position: BitBoards,
    depth: Option<usize>,
    wtime: Option<usize>,
    btime: Option<usize>,
    infinite: bool,
}

enum Message {
    Result((i32, Move)),
}

fn engine_thread(search_params: SearchParams, tx: Sender<Message>) -> Result<(), Box<dyn Error>> {
    let boards = search_params.position;

    let (score, best_move) = boards.search();
    println!("info score cp {score}");
    println!("bestmove {}{}", best_move.coords(), match best_move.promotion() {
        types::PieceIndex::Knight => "n",
        types::PieceIndex::Bishop => "b",
        types::PieceIndex::Rook => "r",
        types::PieceIndex::Queen => "q",
        _ => "",
    },);

    tx.send(Message::Result((score, best_move)))?;
    Ok(())
}
