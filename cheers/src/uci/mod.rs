use std::{fmt::Display, str::FromStr};

use cheers_lib::{
    board::Board,
    moves::Move,
    options::*,
    types::{Color, Piece},
    Square,
};

#[macro_use]
mod macros;

uci_options![
    Hash(Spin<usize> { default: 32, min: 1, max: 32768 }),
    Threads(Spin<usize> { default: 1, min: 1, max: 256 }),
    UCI_Chess960(Check { default: false }),
    SyzygyPath(OptionString { default: "<empty>" }),
    NmpDepth(Spin<i8> { default: NMP_DEPTH, min: 1, max: 10 }),
    NmpConstReduction(Spin<i8> { default: NMP_CONST_REDUCTION, min: 1, max: 10 }),
    NmpLinearDivisor(Spin<i8> { default: NMP_LINEAR_DIVISOR, min: 1, max: 10 }),
    SeePruningDepth(Spin<i8> { default: SEE_PRUNING_DEPTH, min: 1, max: 10 }),
    SeeCaptureMargin(Spin<i16> { default: SEE_CAPTURE_MARGIN, min: -200, max: 200 }),
    SeeQuietMargin(Spin<i16> { default: SEE_QUIET_MARGIN, min: -100, max: 100 }),
    PvsFulldepth(Spin<i8> { default: PVS_FULLDEPTH, min: 1, max: 5 }),
    DeltaPruningMargin(Spin<i16> { default: DELTA_PRUNING_MARGIN, min: 0, max: 300 }),
    FpMargin1(Spin<i16> { default: FP_MARGIN_1, min: 0, max: 300 }),
    FpMargin2(Spin<i16> { default: FP_MARGIN_2, min: 0, max: 700 }),
    FpMargin3(Spin<i16> { default: FP_MARGIN_3, min: 500, max: 1000 }),
    RfpDepth(Spin<i8> { default: RFP_DEPTH, min: 0, max: 15 }),
    RfpMargin(Spin<i16> { default: RFP_MARGIN, min: 0, max: 300 }),
    RfpImprovingMargin(Spin<i16> { default: RFP_IMPROVING_MARGIN, min: -100, max: 100 }),
    LmpDepth(Spin<i8> { default: LMP_DEPTH, min: 0, max: 10 }),
    HistoryLmrDivisor(Spin<i16> { default: HISTORY_LMR_DIVISOR, min: 0, max: 8192 }),
    IirDepth(Spin<i8> { default: IIR_DEPTH, min: 2, max: 10 }),
];

pub enum UciCommand {
    Uci,
    IsReady,
    SetOption(UciOption),
    UciNewGame,
    Position {
        fen: Option<String>,
        moves: Vec<Move>,
    },
    Go {
        wtime: Option<isize>,
        btime: Option<isize>,
        winc: Option<isize>,
        binc: Option<isize>,
        movestogo: Option<usize>,
        depth: Option<usize>,
        nodes: Option<usize>,
        movetime: Option<usize>,
        infinite: bool,
        perft: Option<usize>,
    },
    Fen,
    Stop,
    Quit,
}

type StrValidResult<T> = Result<T, UciParseError>;

pub trait ValidateOption {
    type Output: FromStr;
    fn validate<S: AsRef<str>>(&self, data: S) -> StrValidResult<Self::Output>;
    fn details(&self) -> String;
}

pub struct Check {
    default: bool,
}

impl ValidateOption for Check {
    type Output = bool;
    fn validate<S: AsRef<str>>(&self, data: S) -> StrValidResult<Self::Output> {
        data.as_ref().parse::<bool>().map_err(|_| {
            UciParseError::Other(format!(
                "Invalid value for check option: '{}'. Acceptable values are: 'true', 'false'",
                data.as_ref()
            ))
        })
    }
    fn details(&self) -> String {
        format!("type check default {}", self.default)
    }
}

pub struct Spin<T> {
    default: T,
    min: T,
    max: T,
}

impl<T: FromStr + PartialOrd + Display> ValidateOption for Spin<T> {
    type Output = T;
    fn validate<S: AsRef<str>>(&self, data: S) -> StrValidResult<Self::Output> {
        let value = data.as_ref().parse::<T>().map_err(|_| {
            UciParseError::Other(format!("Invalid value for spin option: {}", data.as_ref()))
        })?;

        if value >= self.min && value <= self.max {
            Ok(value)
        } else {
            Err(UciParseError::Other(format!(
                "Value for spin option out of range [{}, {}]",
                self.min, self.max
            )))
        }
    }

    fn details(&self) -> String {
        format!(
            "type spin default {} min {} max {}",
            self.default, self.min, self.max
        )
    }
}

#[allow(dead_code)]
pub struct Combo {
    vars: &'static [&'static str],
    default: &'static str,
}

impl ValidateOption for Combo {
    type Output = String;

    fn validate<S: AsRef<str>>(&self, data: S) -> StrValidResult<Self::Output> {
        Ok(data.as_ref().to_owned())
    }

    fn details(&self) -> String {
        format!("type combo default {}", self.default)
            + self
                .vars
                .iter()
                .map(|&v| " var ".to_string() + v)
                .collect::<String>()
                .trim_end()
    }
}

pub struct OptionString {
    default: &'static str,
}

impl ValidateOption for OptionString {
    type Output = String;

    fn validate<S: AsRef<str>>(&self, data: S) -> StrValidResult<Self::Output> {
        Ok(data.as_ref().to_owned())
    }

    fn details(&self) -> String {
        format!("type string default {}", self.default)
    }
}

#[derive(Debug)]
pub enum UciParseError {
    Empty,
    Other(String),
}

impl Display for UciParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UciParseError::Empty => "",
                UciParseError::Other(reason) => reason,
            }
        )
    }
}
impl std::error::Error for UciParseError {}

macro_rules! parse_uci_go_value {
    ($words: ident, $name:ident, $type:ty) => {
        let $name = {
            if let Some(p) = $words.iter().position(|&w| w == stringify!($name)) {
                match $words.get(p + 1) {
                    Some(n) => Some(n.parse::<$type>().map_err(|_| {
                        UciParseError::Other(format!(
                            concat!(
                                "Invalid value for ",
                                stringify!($name),
                                " in UCI go command: {}"
                            ),
                            n
                        ))
                    })?),
                    None => {
                        return Err(UciParseError::Other(format!(concat!(
                            "Missing token in UCI go command: no value specified for ",
                            stringify!($name)
                        ))))
                    }
                }
            } else {
                None
            }
        };
    };
}

pub fn parse_uci_command<T: AsRef<str>>(cmd: T) -> Result<UciCommand, UciParseError> {
    let words = cmd.as_ref().split(' ').collect::<Vec<&str>>();

    use UciCommand::*;

    match words.first() {
        Some(word) => {
            match word.to_lowercase().as_str() {
                "uci" => Ok(Uci),
                "isready" => Ok(IsReady),
                "setoption" => {
                    let name = match words.get(1).copied() {
                        Some("name") => words
                            .get(2)
                            .copied()
                            .unwrap_or("Missing token in UCI setoption command: no name specified"),
                        Some(other) => {
                            return Err(UciParseError::Other(format!(
                            "Unexpected token in UCI setoption command: expected 'name', found {}",
                            other
                        )))
                        }
                        None => {
                            return Err(UciParseError::Other(
                                "Missing token in UCI setoption command: 'name' not found"
                                    .to_string(),
                            ))
                        }
                    };
                    let value = match words.get(3).copied() {
                        Some("value") => words.get(4).copied().unwrap_or(
                            "Missing token in UCI setoption command: no value specified",
                        ),
                        Some(other) => {
                            return Err(UciParseError::Other(format!(
                            "Unexpected token in UCI setoption command: expected 'value', found {}",
                            other
                        )))
                        }
                        None => {
                            return Err(UciParseError::Other(
                                "Missing token in UCI setoption command: 'value' not found"
                                    .to_string(),
                            ))
                        }
                    };
                    UciOption::parse(name, value).map(UciCommand::SetOption)
                }
                "ucinewgame" => Ok(UciNewGame),
                "position" => {
                    let mut test = Board::new();
                    let (startpos, fen) = match words.get(1) {
                        Some(&"startpos") => (true, None),
                        Some(&"fen") => {
                            match words.get(2..=7) {
                                Some(fen) => {
                                    let fen = fen.join(" ");
                                    let new = Board::from_fen(&fen);
                                    match new {
                                        None => return Err(UciParseError::Other(format!("Invalid FEN string in UCI position command: {}", fen))),
                                        Some(b) => test = b,
                                    }
                                    (false, Some(fen))
                                }
                                None => {
                                    return Err(UciParseError::Other("Incomplete or missing FEN string in UCI position command".to_string()))
                                }
                            }

                        },
                        Some(p) => return Err(UciParseError::Other(format!(
                                    "Invalid argument in UCI position command: {}\n\t \
                                    Valid arguments are: 'startpos', 'fen [FEN]'", p))),
                        None => return Err(
                            UciParseError::Other("Missing arguments in UCI position command, expected 'startpos' or 'fen'".to_string()))
                    };
                    let moves_index = if startpos { 2 } else { 8 };
                    let moves = match words.get(moves_index) {
                        Some(&"moves") => match words.get((moves_index + 1)..) {
                            Some(moves) => {
                                let mut checked_moves = Vec::new();
                                for move_string in moves {
                                    // convert regular castling moves
                                    let move_string = if *move_string == "e1g1"
                                        && test.current_player() == Color::White
                                        && test.piece_on(Square::E1) == Some(Piece::King)
                                    {
                                        let kingside = test.castling_rights()[0][0]
                                            .first_square()
                                            .file_letter();
                                        move_string.replace('g', kingside)
                                    } else if *move_string == "e8g8"
                                        && test.current_player() == Color::Black
                                        && test.piece_on(Square::E8) == Some(Piece::King)
                                    {
                                        let kingside = test.castling_rights()[1][0]
                                            .first_square()
                                            .file_letter();
                                        move_string.replace('g', kingside)
                                    } else if *move_string == "e1c1"
                                        && test.current_player() == Color::White
                                        && test.piece_on(Square::E1) == Some(Piece::King)
                                    {
                                        let queenside = test.castling_rights()[0][1]
                                            .first_square()
                                            .file_letter();
                                        move_string.to_string().replace('c', queenside)
                                    } else if *move_string == "e8c8"
                                        && test.current_player() == Color::Black
                                        && test.piece_on(Square::E8) == Some(Piece::King)
                                    {
                                        let queenside = test.castling_rights()[1][1]
                                            .first_square()
                                            .file_letter();
                                        move_string.replace('c', queenside)
                                    } else {
                                        move_string.to_string()
                                    };
                                    if test
                                        .legal_move_list()
                                        .iter()
                                        .map(|m| m.coords_960())
                                        .any(|m| m == move_string)
                                    {
                                        let checked_move = Move::from_pair(&test, move_string);
                                        test.make_move(checked_move);
                                        checked_moves.push(checked_move);
                                    } else {
                                        return Err(UciParseError::Other(format!(
                                            "Illegal move in UCI position command: {}",
                                            move_string
                                        )));
                                    }
                                }
                                return Ok(Position {
                                    fen,
                                    moves: checked_moves,
                                });
                            }
                            None => {
                                return Err(UciParseError::Other(
                                    "Missing move list in UCI position command".to_string(),
                                ))
                            }
                        },
                        Some(other) => {
                            return Err(UciParseError::Other(format!(
                                "Expected 'moves' in UCI position command, found {}",
                                other
                            )))
                        }
                        None => Vec::new(),
                    };

                    Ok(Position { fen, moves })
                }
                "go" => {
                    parse_uci_go_value!(words, wtime, isize);
                    parse_uci_go_value!(words, btime, isize);
                    parse_uci_go_value!(words, winc, isize);
                    parse_uci_go_value!(words, binc, isize);
                    parse_uci_go_value!(words, movestogo, usize);
                    parse_uci_go_value!(words, depth, usize);
                    parse_uci_go_value!(words, nodes, usize);
                    parse_uci_go_value!(words, movetime, usize);

                    parse_uci_go_value!(words, perft, usize);

                    let infinite = words.iter().find(|&&s| s == "infinite");
                    if infinite.is_some()
                        && (wtime.is_some()
                            || btime.is_some()
                            || winc.is_some()
                            || binc.is_some()
                            || movestogo.is_some()
                            || depth.is_some()
                            || nodes.is_some()
                            || movetime.is_some())
                    {
                        return Err(UciParseError::Other("Error in UCI go command: 'infinite' specified along with other search directives".to_string()));
                    }

                    if perft.is_some()
                        && (wtime.is_some()
                            || btime.is_some()
                            || winc.is_some()
                            || binc.is_some()
                            || movestogo.is_some()
                            || depth.is_some()
                            || nodes.is_some()
                            || movetime.is_some()
                            || infinite.is_some())
                    {
                        return Err(UciParseError::Other("Error in UCI go command: 'perft' specified along with other directives".to_string()));
                    }

                    Ok(Go {
                        wtime,
                        btime,
                        winc,
                        binc,
                        movestogo,
                        depth,
                        nodes,
                        movetime,
                        infinite: infinite.is_some(),
                        perft,
                    })
                }
                "fen" => Ok(Fen),
                "stop" => Ok(Stop),
                "quit" => Ok(Quit),
                other => Err(UciParseError::Other(format!(
                    "Unknown UCI command: {}",
                    other
                ))),
            }
        }
        None => Err(UciParseError::Empty),
    }
}
