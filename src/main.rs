mod lookup_tables;
mod types;
mod bitboards;
mod moves;

use std::{
    error::Error,
    io::{prelude::*, stdin},
    time::Instant,
};

use bitboards::BitBoards;
use lookup_tables::lookup_tables;

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
                lookup_tables();
                let end = Instant::now();
                println!("Generated magics in {:.3}s", (end - start).as_secs_f32());
            }
            Some(&"test") => {
                let mut boards = BitBoards::default();
                boards.set_from_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10".to_string())?;
                let moves = boards.legal_moves();
                for move_ in &moves {
                    println!("{}", move_);
                }
                println!("{}", moves.len());
            }
            _ => println!("unknown command: {}", line),
        }
    }
    Ok(())
}
