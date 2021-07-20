use std::error::Error;
use std::io::{prelude::*, stdin};

use rand::prelude::*;


mod bitboard;
mod lookup_tables;

use bitboard::*;
use lookup_tables::*;

fn main() -> Result<(), Box<dyn Error>> {
    for line_res in stdin().lock().lines() {
        let line = line_res?;

        match &line[..] {
            "uci" => {
                println!("id name ches");
                println!("id author Algorhythm");
                println!("uciok");
            }
            "quit" => {
                break;
            }
            "gen" => {
                let luts = LookupTables::generate_all();
                let mut bitboards = BitBoards::new(luts);
                let moves =  bitboards.generate_pseudolegal_moves(White);
                for i in moves.iter() {
                    println!("{} -> {}", square_to_coord(i.0), square_to_coord(i.1));
                }
                if let Some(choice) = moves.choose(&mut thread_rng()) {
                    println!("chose {} -> {}", square_to_coord(choice.0), square_to_coord(choice.1));                    
                    bitboards.make_move(choice.0 as usize, choice.1 as usize);
                }
            }
            _ => {
                println!("unknown command: {}", line)
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
