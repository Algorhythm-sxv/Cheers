#[derive(Clone, Copy, Debug)]
pub enum GameResult {
    WhiteWin,
    BlackWin,
    Draw,
}
impl GameResult {
    pub fn into_f64(self) -> f64 {
        match self {
            WhiteWin => 1.0,
            BlackWin => 0.0,
            Draw => 0.5,
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
use GameResult::*;

impl ToString for GameResult {
    fn to_string(&self) -> String {
        match self {
            WhiteWin => String::from("1"),
            BlackWin => String::from("0"),
            Draw => String::from("0.5"),
        }
    }
}
