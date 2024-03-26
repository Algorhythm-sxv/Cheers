use cheers_bitboards::{BitBoard, Square};
use pyrrhic_rs::EngineAdapter;

use super::{
    lookup_bishop, lookup_king, lookup_knight, lookup_queen, lookup_rook, Black, Board, White,
};

#[derive(Clone)]
pub struct MovegenAdapter;

impl EngineAdapter for MovegenAdapter {
    fn pawn_attacks(color: pyrrhic_rs::Color, square: u64) -> u64 {
        match color {
            pyrrhic_rs::Color::White => Board::pawn_attack::<White>(Square::from(square as u8)).0,
            pyrrhic_rs::Color::Black => Board::pawn_attack::<Black>(Square::from(square as u8)).0,
        }
    }

    fn knight_attacks(square: u64) -> u64 {
        lookup_knight(Square::from(square as u8)).0
    }

    fn bishop_attacks(square: u64, occupied: u64) -> u64 {
        lookup_bishop(Square::from(square as u8), BitBoard(occupied)).0
    }

    fn rook_attacks(square: u64, occupied: u64) -> u64 {
        lookup_rook(Square::from(square as u8), BitBoard(occupied)).0
    }

    fn queen_attacks(square: u64, occupied: u64) -> u64 {
        lookup_queen(Square::from(square as u8), BitBoard(occupied)).0
    }

    fn king_attacks(square: u64) -> u64 {
        lookup_king(Square::from(square as u8)).0
    }
}
