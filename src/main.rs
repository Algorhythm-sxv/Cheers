mod bitboards;
mod lookup_tables;
mod moves;
mod types;
mod zobrist;

use std::{
    error::Error,
    fs::File,
    io::{prelude::*, stdin},
    time::Instant,
};

use bitboards::BitBoards;
use lookup_tables::lookup_tables;

use crate::lookup_tables::LookupTables;

fn main() -> Result<(), Box<dyn Error>> {
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
                println!("readyok");
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
                        let mut boards = BitBoards::new();
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
                let mut boards = BitBoards::new();
                boards.set_from_fen(
                    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                )?;
                let depth = match words.get(1) {
                    None => 5,
                    Some(num) => num.parse::<usize>()?,
                };
                for i in 0..=depth {
                    println!("Perft {}: {}", i, boards.perft(i))
                }
            }
            Some(&"divide") => {
                let depth = match words.get(1) {
                    None => 5,
                    Some(num) => num.parse::<usize>()?,
                };
                let mut boards = BitBoards::new();
                boards.set_from_fen(
                    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                )?;
                boards.divide(depth);
            }
            _ => println!("unknown command: {}", line),
        }
    }
    Ok(())
}
