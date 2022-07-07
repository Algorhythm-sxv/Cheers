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
use cheers_lib::{
    chessgame::ChessGame,
    moves::Move,
    transposition_table::{TranspositionTable, TT_DEFAULT_SIZE},
    types::PieceIndex,
};
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
    game: ChessGame,
    game_result: GameResult,
    fen_list: Vec<String>,
}

impl FENWriter {
    pub fn new() -> Self {
        let tt = TranspositionTable::new(TT_DEFAULT_SIZE);
        Self {
            move_counter: 0,
            game: ChessGame::new(tt),
            game_result: GameResult::Draw,
            fen_list: Vec::new(),
        }
    }
    fn reset(&mut self) {
        self.game.reset();
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

fn san_to_move(san: San, game: &ChessGame) -> Option<Move> {
    let mut candidates = game.legal_moves();
    candidates.retain(|m| match san {
        San::Normal {
            role,
            file,
            rank,
            capture,
            to,
            promotion,
        } => {
            let correct_file = if let Some(f) = file {
                f as u8 == m.start() % 8
            } else {
                true
            };
            let correct_rank = if let Some(r) = rank {
                r as u8 == m.start() / 8
            } else {
                true
            };
            compare_roles(m.piece(), role)
                && m.target() == to as u8
                && m.capture() == capture
                && correct_file
                && correct_rank
                && promotion
                    .map(|p| compare_roles(m.promotion(), p))
                    .unwrap_or(true)
        }
        San::Castle(side) => {
            m.castling()
                && m.target() % 8
                    == match side {
                        CastlingSide::KingSide => 6,
                        CastlingSide::QueenSide => 2,
                    }
        }
        _ => false,
    });
    candidates.pop()
}

fn compare_roles(piece_index: PieceIndex, role: Role) -> bool {
    match piece_index {
        PieceIndex::Pawn => role == Role::Pawn,
        PieceIndex::Knight => role == Role::Knight,
        PieceIndex::Bishop => role == Role::Bishop,
        PieceIndex::Rook => role == Role::Rook,
        PieceIndex::Queen => role == Role::Queen,
        PieceIndex::King => role == Role::King,
        PieceIndex::NoPiece => unreachable!(),
    }
}
