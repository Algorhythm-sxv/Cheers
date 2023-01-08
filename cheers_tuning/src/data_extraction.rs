#[derive(Clone, Copy, Debug)]
pub enum GameResult {
    WhiteWin,
    BlackWin,
    Draw,
    Abandoned,
}
impl GameResult {
    pub fn into_f64(self) -> f64 {
        match self {
            WhiteWin => 1.0,
            BlackWin => 0.0,
            Draw | Abandoned => 0.5,
        }
    }
    pub fn from_f64(n: f64) -> Self {
        if n.abs() < 0.01 {
            BlackWin
        } else if (n - 1.0).abs() < 0.01 {
            WhiteWin
        } else {
            Draw
        }
    }
}
use cheers_lib::{board::Board, moves::Move, types::Piece};
use pgn_reader::*;
use GameResult::*;

impl ToString for GameResult {
    fn to_string(&self) -> String {
        match self {
            WhiteWin => String::from("1"),
            BlackWin => String::from("0"),
            Draw => String::from("0.5"),
            Abandoned => String::from("*"),
        }
    }
}

pub struct FENWriter {
    move_counter: usize,
    game: Board,
    game_result: GameResult,
    fen_list: Vec<String>,
}

impl FENWriter {
    pub fn new() -> Self {
        Self {
            move_counter: 0,
            game: Board::new(),
            game_result: GameResult::Draw,
            fen_list: Vec::new(),
        }
    }
    fn reset(&mut self) {
        self.game = Board::new();
        self.game_result = Draw;
        self.fen_list.clear();
        self.move_counter = 0;
    }
}

impl Visitor for FENWriter {
    type Result = Vec<String>;

    fn header(&mut self, key: &[u8], value: RawHeader<'_>) {
        if key == b"Result" {
            self.game_result = match value.as_bytes() {
                b"1-0" => WhiteWin,
                b"0-1" => BlackWin,
                b"1/2-1/2" => Draw,
                b"*" => Abandoned,
                other => panic!("Invalid game result: {}", String::from_utf8_lossy(other)),
            }
        }
    }
    fn end_headers(&mut self) -> Skip {
        match self.game_result {
            Abandoned => Skip(true),
            _ => Skip(false),
        }
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true)
    }

    fn end_game(&mut self) -> Self::Result {
        // discard last 10 moves (to avoid easy mating positions)
        let list = self.fen_list[..(self.fen_list.len().saturating_sub(10))].to_vec();
        self.reset();
        list
    }

    fn san(&mut self, san_plus: pgn_reader::SanPlus) {
        let san = san_plus.san;
        if let Some(m) = san_to_move(san, &self.game) {
            self.game.make_move(m);

            self.move_counter += 1;
            // add moves out of opening and not close to mate
            if self.move_counter >= 10 {
                let score = 0; //self.game.search(Some(4), true).0;
                if score < 20000 {
                    self.fen_list
                        //.push(self.game.fen() + " score: " + &score.to_string());
                        .push(self.game.fen() + " result: " + &self.game_result.to_string())
                }
            }
        }
    }
}

fn san_to_move(san: San, game: &Board) -> Option<Move> {
    let mut candidates = game.legal_move_list();
    candidates.retain(|m| match san {
        San::Normal {
            role,
            file,
            rank,
            capture: _,
            to,
            promotion,
        } => {
            let correct_file = if let Some(f) = file {
                f as usize == m.from.file()
            } else {
                true
            };
            let correct_rank = if let Some(r) = rank {
                r as usize == m.from.rank()
            } else {
                true
            };
            compare_roles(m.piece, role)
                && m.to == (to as u8).into()
                && correct_file
                && correct_rank
                && promotion
                    .map(|p| compare_roles(m.promotion, p))
                    .unwrap_or(true)
        }
        San::Castle(side) => {
            m.to.file()
                == match side {
                    CastlingSide::KingSide => 6,
                    CastlingSide::QueenSide => 2,
                }
        }
        _ => false,
    });
    candidates.pop()
}

fn compare_roles(piece_index: Piece, role: Role) -> bool {
    match piece_index {
        Piece::Pawn => role == Role::Pawn,
        Piece::Knight => role == Role::Knight,
        Piece::Bishop => role == Role::Bishop,
        Piece::Rook => role == Role::Rook,
        Piece::Queen => role == Role::Queen,
        Piece::King => role == Role::King,
    }
}
