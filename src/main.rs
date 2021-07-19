use std::error::Error;
use std::io::{prelude::*, stdin};


mod bitboard;
mod lookup_tables;

use bitboard::*;
use lookup_tables::*;

struct Engine {
    bitboards: BitBoards,
    sliding_attack_table: Vec<u64>,
    rook_magics: Vec<MagicSquare>,
    bishop_magics: Vec<MagicSquare>,
}

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
                let bitboards = BitBoards::new(luts);
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
