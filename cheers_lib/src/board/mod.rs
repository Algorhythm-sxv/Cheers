pub mod eval_params;
pub mod eval_types;
pub mod evaluate;
pub mod movegen;
pub mod see;

use std::time::Instant;

use crate::lookup_tables::*;
use crate::moves::*;
use crate::types::*;
use crate::zobrist::*;
use cheers_bitboards::*;

use Piece::*;

macro_rules! select_colored_pieces {
    ($self: ident, $piece:ident, $piece_board:ident, $color_board:ident) => {
        let ($piece_board, $color_board) = if T::WHITE {
            (
                match $piece {
                    Pawn => &mut $self.white_pawns,
                    Knight => &mut $self.white_knights,
                    Bishop => &mut $self.white_bishops,
                    Rook => &mut $self.white_rooks,
                    Queen => &mut $self.white_queens,
                    King => &mut $self.white_king,
                },
                &mut $self.white_pieces,
            )
        } else {
            (
                match $piece {
                    Pawn => &mut $self.black_pawns,
                    Knight => &mut $self.black_knights,
                    Bishop => &mut $self.black_bishops,
                    Rook => &mut $self.black_rooks,
                    Queen => &mut $self.black_queens,
                    King => &mut $self.black_king,
                },
                &mut $self.black_pieces,
            )
        };
    };
}

#[derive(Copy, Clone, Debug)]
pub struct Board {
    white_pawns: BitBoard,
    black_pawns: BitBoard,
    white_knights: BitBoard,
    black_knights: BitBoard,
    white_bishops: BitBoard,
    black_bishops: BitBoard,
    white_rooks: BitBoard,
    black_rooks: BitBoard,
    white_queens: BitBoard,
    black_queens: BitBoard,
    white_king: BitBoard,
    black_king: BitBoard,
    white_pieces: BitBoard,
    black_pieces: BitBoard,
    occupied: BitBoard,
    castling_rights: [[BitBoard; 2]; 2],
    check_mask: BitBoard,
    pub diagonal_pin_mask: BitBoard,
    pub orthogonal_pin_mask: BitBoard,
    ep_mask: BitBoard,
    black_to_move: bool,
    halfmove_clock: u8,
    hash: u64,
    pawn_hash: u64,
}

impl Board {
    pub fn new() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub fn perft(&self, depth: usize) {
        let mut nodes = 0;
        if depth == 0 {
            println!("Nodes: 1");
        }

        let start = Instant::now();
        self.generate_legal_moves(|mvs| {
            for mv in mvs {
                let mut new = *self;
                new.make_move(mv);
                let subnodes = new._perft(depth - 1);
                nodes += subnodes;
                println!("{mv}: {subnodes}");
            }
        });
        let end = Instant::now();

        println!(
            "Nodes: {nodes}\t\tNPS: {}",
            (nodes as f64 / (end - start).as_secs_f64()) as usize
        )
    }

    fn _perft(&self, depth: usize) -> usize {
        if depth == 1 {
            let mut nodes = 0;
            self.generate_legal_moves(|mvs| {
                nodes += mvs.len();
            });
            return nodes;
        } else if depth == 0 {
            return 1;
        }

        let mut nodes = 0;

        self.generate_legal_moves(|mvs| {
            for mv in mvs {
                let mut new = *self;
                new.make_move(mv);
                nodes += new._perft(depth - 1);
            }
        });
        nodes
    }

    #[inline(always)]
    pub fn current_player(&self) -> usize {
        self.black_to_move as usize
    }

    #[inline(always)]
    pub fn in_check(&self) -> bool {
        self.check_mask != FULL_BOARD
    }

    #[inline(always)]
    pub fn halfmove_clock(&self) -> u8 {
        self.halfmove_clock
    }

    #[inline(always)]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    #[inline(always)]
    pub fn castling_rights(&self) -> &[[BitBoard; 2]; 2] {
        &self.castling_rights
    }

    #[inline(always)]
    pub fn is_capture(&self, mv: Move) -> bool {
        if self.black_to_move {
            (mv.to.bitboard() & self.white_pieces).is_not_empty()
                || (mv.piece == Pawn && mv.to.bitboard() == self.ep_mask)
        } else {
            (mv.to.bitboard() & self.black_pieces).is_not_empty()
                || (mv.piece == Pawn && mv.to.bitboard() == self.ep_mask)
        }
    }

    #[inline(always)]
    pub fn has_non_pawn_material(&self, color: usize) -> bool {
        let material = if color == 0 {
            self.white_knights | self.white_bishops | self.white_rooks | self.white_queens
        } else {
            self.black_knights | self.black_bishops | self.black_rooks | self.black_queens
        };

        material.is_not_empty()
    }

    #[inline(always)]
    fn all_attacks_on(&self, target: Square, mask: BitBoard) -> BitBoard {
        let knights = self.white_knights | self.black_knights;
        let bishops =
            self.white_bishops | self.black_bishops | self.white_queens | self.black_queens;
        let rooks = self.white_rooks | self.black_rooks | self.white_queens | self.black_queens;
        let kings = self.white_king | self.black_king;

        (self.pawn_attack::<White>(target) & self.black_pawns)
            | (self.pawn_attack::<Black>(target) & self.white_pawns)
            | (lookup_knight(target) & knights)
            | (lookup_bishop(target, mask) & bishops)
            | (lookup_rook(target, mask) & rooks)
            | (lookup_king(target) & kings)
    }

    #[inline(always)]
    pub fn forward<T: TypeColor>(&self, mask: BitBoard) -> BitBoard {
        if T::WHITE {
            mask << 8
        } else {
            mask >> 8
        }
    }

    #[inline(always)]
    pub fn ep_target<T: TypeColor>(&self) -> BitBoard {
        if T::WHITE {
            self.ep_mask >> 8
        } else {
            self.ep_mask << 8
        }
    }

    #[inline(always)]
    pub fn pawn_push_span<T: TypeColor>(square: Square) -> BitBoard {
        let start = square.bitboard();

        let mut board = BitBoard::empty();
        if T::WHITE {
            board |= start << 8 | start << 16;
            board |= board << 16;
            board |= board << 32;
        } else {
            board |= start >> 8 | start >> 16;
            board |= board >> 16;
            board |= board >> 32;
        };

        board
    }

    #[inline(always)]
    pub fn pawn_push_spans<T: TypeColor>(pawns: BitBoard) -> BitBoard {
        let mut board = BitBoard::empty();
        if T::WHITE {
            board |= pawns << 8 | pawns << 16;
            board |= pawns << 16;
            board |= pawns << 32;
        } else {
            board |= pawns >> 8 | pawns >> 16;
            board |= pawns >> 16;
            board |= pawns >> 32;
        }
        board
    }

    #[inline(always)]
    pub fn pawn_front_spans<T: TypeColor>(pawns: BitBoard) -> BitBoard {
        let mut spans = pawns;
        if T::WHITE {
            spans |= spans << 8;
            spans |= spans << 16;
            spans |= spans << 32;
        } else {
            spans |= spans >> 8;
            spans |= spans >> 16;
            spans |= spans >> 32;
        }
        spans
    }

    #[inline(always)]
    pub fn pawn_attack_spans<T: TypeColor>(&self) -> BitBoard {
        let mut spans = self.pawn_attacks::<T>();
        if T::WHITE {
            spans |= spans << 8;
            spans |= spans << 16;
            spans |= spans << 32;
        } else {
            spans |= spans >> 8;
            spans |= spans >> 16;
            spans |= spans >> 32;
        };

        spans
    }

    #[inline(always)]
    pub fn pawn_pushes<T: TypeColor>(&self, square: Square) -> BitBoard {
        let empty = self.occupied.inverse();
        let mask = square.bitboard();
        if T::WHITE {
            let single = (mask << 8) & empty;
            let double = (((mask & SECOND_RANK) << 8) & empty) << 8;
            single | (double & empty)
        } else {
            let single = (mask >> 8) & empty;
            let double = (((mask & SEVENTH_RANK) >> 8) & empty) >> 8;
            single | (double & empty)
        }
    }
    #[inline(always)]
    pub fn pawn_attack<T: TypeColor>(&self, square: Square) -> BitBoard {
        let board = square.bitboard();
        if T::WHITE {
            ((board & A_FILE.inverse()) << 7) | ((board & H_FILE.inverse()) << 9)
        } else {
            ((board & A_FILE.inverse()) >> 9) | ((board & H_FILE.inverse()) >> 7)
        }
    }

    #[inline(always)]
    pub fn pawn_attacks<T: TypeColor>(&self) -> BitBoard {
        if T::WHITE {
            ((self.white_pawns & A_FILE.inverse()) << 7)
                | ((self.white_pawns & H_FILE.inverse()) << 9)
        } else {
            ((self.black_pawns & A_FILE.inverse()) >> 9)
                | ((self.black_pawns & H_FILE.inverse()) >> 7)
        }
    }

    #[inline(always)]
    pub fn knight_attacks<T: TypeColor>(&self) -> BitBoard {
        let knights = if T::WHITE {
            self.white_knights
        } else {
            self.black_knights
        };
        let mut attacks = BitBoard::empty();
        for knight in knights {
            attacks |= lookup_knight(knight)
        }
        attacks
    }

    #[inline(always)]
    pub fn diagonal_attacks<T: TypeColor>(&self, mask: BitBoard) -> BitBoard {
        let sliders = if T::WHITE {
            self.white_bishops | self.white_queens
        } else {
            self.black_bishops | self.black_queens
        };
        let mut attacks = BitBoard::empty();
        for slider in sliders {
            attacks |= lookup_bishop(slider, mask)
        }
        attacks
    }

    #[inline(always)]
    pub fn orthogonal_attacks<T: TypeColor>(&self, mask: BitBoard) -> BitBoard {
        let sliders = if T::WHITE {
            self.white_rooks | self.white_queens
        } else {
            self.black_rooks | self.black_queens
        };
        let mut attacks = BitBoard::empty();
        for slider in sliders {
            attacks |= lookup_rook(slider, mask)
        }
        attacks
    }

    #[inline(always)]
    pub fn all_enemy_attacks<T: TypeColor>(&self, mask: BitBoard) -> BitBoard {
        if T::WHITE {
            self.pawn_attacks::<Black>()
                | self.knight_attacks::<Black>()
                | self.diagonal_attacks::<Black>(mask)
                | self.orthogonal_attacks::<Black>(mask)
                | lookup_king(self.black_king.first_square())
        } else {
            self.pawn_attacks::<White>()
                | self.knight_attacks::<White>()
                | self.diagonal_attacks::<White>(mask)
                | self.orthogonal_attacks::<White>(mask)
                | lookup_king(self.white_king.first_square())
        }
    }

    #[inline(always)]
    pub fn king_safe_squares<T: TypeColor>(&self) -> BitBoard {
        let king = if T::WHITE {
            self.white_king
        } else {
            self.black_king
        };
        self.all_enemy_attacks::<T>(self.occupied ^ king).inverse()
    }

    #[inline(always)]
    fn discovered_attacks<T: TypeColor>(&self, square: Square) -> BitBoard {
        let rook_attacks = lookup_rook(square, self.occupied);
        let bishop_attacks = lookup_bishop(square, self.occupied);

        let (bishops, rooks) = if T::WHITE {
            (
                self.black_bishops & bishop_attacks.inverse(),
                self.black_rooks & rook_attacks.inverse(),
            )
        } else {
            (
                self.white_bishops & bishop_attacks.inverse(),
                self.white_rooks & rook_attacks.inverse(),
            )
        };

        (rooks & lookup_rook(square, self.occupied & rook_attacks.inverse()))
            | (bishops & lookup_bishop(square, self.occupied & bishop_attacks.inverse()))
    }

    #[inline(always)]
    pub fn xor_piece<T: TypeColor>(&mut self, piece: Piece, square: Square) {
        select_colored_pieces!(self, piece, board, pieces);
        let mask = square.bitboard();
        *board ^= mask;
        *pieces ^= mask;
        self.occupied ^= mask;
        self.hash ^= zobrist_piece::<T>(piece, square);
    }

    #[inline(always)]
    pub fn move_piece<T: TypeColor>(&mut self, piece: Piece, start: Square, target: Square) {
        select_colored_pieces!(self, piece, board, pieces);
        let start_mask = start.bitboard();
        let target_mask = target.bitboard();
        let xor_mask = start_mask ^ target_mask;

        *board ^= xor_mask;
        *pieces ^= xor_mask;

        self.occupied ^= start_mask;
        self.hash ^= zobrist_piece::<T>(piece, start);
        self.occupied |= target_mask;
        self.hash ^= zobrist_piece::<T>(piece, target);
    }

    #[inline(always)]
    pub fn color<T: TypeColor>(&self) -> BitBoard {
        if T::WHITE {
            self.white_pieces
        } else {
            self.black_pieces
        }
    }

    #[inline(always)]
    pub fn piece_on(&self, square: Square) -> Option<Piece> {
        let mask = square.bitboard();
        if (self.occupied & mask).is_empty() {
            None
        } else if (self.white_pieces & mask).is_not_empty() {
            if ((self.white_pawns | self.white_knights | self.white_bishops) & mask).is_not_empty()
            {
                if (self.white_pawns & mask).is_not_empty() {
                    Some(Pawn)
                } else if (self.white_knights & mask).is_not_empty() {
                    Some(Knight)
                } else {
                    Some(Bishop)
                }
            } else if (self.white_rooks & mask).is_not_empty() {
                Some(Rook)
            } else if (self.white_queens & mask).is_not_empty() {
                Some(Queen)
            } else {
                Some(King)
            }
        } else {
            if ((self.black_pawns | self.black_knights | self.black_bishops) & mask).is_not_empty()
            {
                if (self.black_pawns & mask).is_not_empty() {
                    Some(Pawn)
                } else if (self.black_knights & mask).is_not_empty() {
                    Some(Knight)
                } else {
                    Some(Bishop)
                }
            } else if (self.black_rooks & mask).is_not_empty() {
                Some(Rook)
            } else if (self.black_queens & mask).is_not_empty() {
                Some(Queen)
            } else {
                Some(King)
            }
        }
    }

    pub fn is_pseudolegal(&self, mv: Move) -> bool {
        if self.black_to_move {
            self._is_pseudolegal::<Black>(mv)
        } else {
            self._is_pseudolegal::<White>(mv)
        }
    }

    fn _is_pseudolegal<T: TypeColor>(&self, mv: Move) -> bool {
        // null moves are never legal
        if mv.is_null() {
            return false;
        }

        // moving wrong piece
        if Some(mv.piece) != self.piece_on(mv.from) {
            return false;
        }

        let (pieces, enemy_pieces) = if T::WHITE {
            (self.white_pieces, self.black_pieces)
        } else {
            (self.black_pieces, self.white_pieces)
        };

        let from = mv.from.bitboard();
        let to = mv.to.bitboard();

        // moving from a square without a friendly piece
        if (from & pieces).is_empty() {
            return false;
        }

        // capturing a friendly piece (while not castling)
        if mv.piece != King && (to & pieces).is_not_empty() {
            return false;
        }

        // piece special cases
        match mv.piece {
            King => {
                // castling
                let rights = if T::WHITE {
                    self.castling_rights[0]
                } else {
                    self.castling_rights[1]
                };
                if (to & (rights[0] | rights[1])).is_not_empty() {
                    // get for full castling legality
                    let queenside = (to | rights[1]).is_not_empty();
                    return self.castling_legal::<T>(queenside);
                }
            }
            Pawn => {
                // erroneous promotions
                if mv.promotion != Pawn && (to & (FIRST_RANK | EIGHTH_RANK)).is_empty() {
                    return false;
                }
                // pushes
                if matches!((mv.to).abs_diff(*mv.from), 8 | 16) {
                    return (self.pawn_pushes::<T>(mv.from) & to).is_not_empty();
                } else {
                    // captures
                    return (self.pawn_attack::<T>(mv.from) & to & (enemy_pieces | self.ep_mask))
                        .is_not_empty();
                }
            }
            _ => {}
        }

        let piece_attacks = match mv.piece {
            Pawn => unreachable!(),
            Knight => lookup_knight(mv.from),
            Bishop => lookup_bishop(mv.from, self.occupied),
            Rook => lookup_rook(mv.from, self.occupied),
            Queen => lookup_queen(mv.from, self.occupied),
            King => lookup_king(mv.from),
        };

        (to & piece_attacks & pieces.inverse()).is_not_empty()
    }

    #[inline(always)]
    pub fn illegal_position(&self) -> bool {
        if self.black_to_move {
            self.king_attacked::<White>()
        } else {
            self.king_attacked::<Black>()
        }
    }

    #[inline(always)]
    pub fn king_attacked<T: TypeColor>(&self) -> bool {
        let king = if T::WHITE {
            self.white_king
        } else {
            self.black_king
        };

        (king & self.all_enemy_attacks::<T>(self.occupied)).is_not_empty()
    }

    pub fn material_draw(&self) -> bool {
        // do not report any positions with pawns as material draws
        if (self.white_pawns | self.black_pawns).is_not_empty() {
            return false;
        }

        // KNvK
        if (self.white_knights.count_ones() == 1 && self.black_pieces.count_ones() == 1)
            || (self.black_knights.count_ones() == 1 && self.white_pieces.count_ones() == 1)
        {
            return true;
        }

        // KBvK
        if (self.white_bishops.count_ones() == 1 && self.black_pieces.count_ones() == 1)
            || (self.black_bishops.count_ones() == 1 && self.white_pieces.count_ones() == 1)
        {
            return true;
        }

        // Extended material draws: these positions can win/lose with imperfect
        // play or timeout, but we can just call them draws

        if self.white_pieces.count_ones() == 2 && self.black_pieces.count_ones() == 2 {
            // KNvKN or KBvKN or KBvKB
            if (self.white_knights | self.white_bishops).count_ones() == 1
                && (self.black_knights | self.black_bishops).count_ones() == 1
            {
                return true;
            }
            // KRvKR
            if self.white_rooks.count_ones() == 1 && self.black_rooks.count_ones() == 1 {
                return true;
            }
            // KQvKQ
            if self.white_queens.count_ones() == 1 && self.black_queens.count_ones() == 1 {
                return true;
            }
        }

        false
    }

    pub fn make_move(&mut self, mv: Move) {
        if self.black_to_move {
            self.make_move_for::<Black>(mv);
        } else {
            self.make_move_for::<White>(mv);
        }
    }

    fn make_move_for<T: TypeColor>(&mut self, mv: Move) {
        let piece = mv.piece;
        let start_mask = mv.from.bitboard();
        let target_mask = mv.to.bitboard();
        let capture = self.piece_on(mv.to);
        let (color, other_color) = (self.black_to_move as usize, !self.black_to_move as usize);
        let other_king = if T::WHITE {
            self.black_king.first_square()
        } else {
            self.white_king.first_square()
        };

        // castling is encoded as king captures friendly rook
        let castling = (self.color::<T>() & mv.to.bitboard()).is_not_empty();

        self.diagonal_pin_mask = BitBoard::empty();
        self.orthogonal_pin_mask = BitBoard::empty();
        self.check_mask = FULL_BOARD;

        self.halfmove_clock += 1;

        if castling {
            // select the target squares for king and rook
            let (king_target, rook_target) = if T::WHITE {
                if mv.from.file() < mv.to.file() {
                    (Square::G1, Square::F1)
                } else {
                    (Square::C1, Square::D1)
                }
            } else {
                if mv.from.file() < mv.to.file() {
                    (Square::G8, Square::F8)
                } else {
                    (Square::C8, Square::D8)
                }
            };

            // move the king and the rook
            self.move_piece::<T>(King, mv.from, king_target);
            self.move_piece::<T>(Rook, mv.to, rook_target);

            // clear castling rights
            self.hash ^= zobrist_castling(self.castling_rights);
            self.castling_rights[color] = [BitBoard::empty(); 2];
            self.hash ^= zobrist_castling(self.castling_rights);
        } else {
            if let Some(capture) = capture {
                // reset halfmove clock
                self.halfmove_clock = 0;

                // remove a captured piece from the target square
                self.xor_piece::<T::Other>(capture, mv.to);

                // update pawn hash if a pawn was taken
                if capture == Pawn {
                    self.pawn_hash ^= zobrist_piece::<T::Other>(Pawn, mv.to);
                }

                // update castling rights for captured rooks
                if (target_mask & self.castling_rights[other_color][0]).is_not_empty() {
                    self.hash ^= zobrist_castling(self.castling_rights);
                    self.castling_rights[other_color][0] = BitBoard::empty();
                    self.hash ^= zobrist_castling(self.castling_rights);
                } else if (target_mask & self.castling_rights[other_color][1]).is_not_empty() {
                    self.hash ^= zobrist_castling(self.castling_rights);
                    self.castling_rights[other_color][1] = BitBoard::empty();
                    self.hash ^= zobrist_castling(self.castling_rights);
                }
            }
            // move the moving piece
            self.move_piece::<T>(piece, mv.from, mv.to);
        }

        let mut new_ep_mask = BitBoard::empty();
        // Per-piece special cases
        match piece {
            Knight => {
                let new_checkers = lookup_knight(other_king) & target_mask;
                if new_checkers.is_not_empty() {
                    self.check_mask = new_checkers;
                }
            }
            Pawn => {
                // reset the halfmove clock
                self.halfmove_clock = 0;

                // update the pawn hash
                self.pawn_hash ^= zobrist_piece::<T>(Pawn, mv.from);
                self.pawn_hash ^= zobrist_piece::<T>(Pawn, mv.to);

                if mv.promotion != Pawn {
                    // remove the pawn and place the promoted piece
                    self.xor_piece::<T>(Pawn, mv.to);
                    self.pawn_hash ^= zobrist_piece::<T>(Pawn, mv.to);
                    self.xor_piece::<T>(mv.promotion, mv.to);

                    // update check mask for knight promotions
                    if mv.promotion == Knight {
                        let new_checkers = lookup_knight(other_king) & target_mask;
                        if new_checkers.is_not_empty() {
                            self.check_mask = new_checkers;
                        }
                    }
                } else {
                    if mv.to.abs_diff(*mv.from) == 16 {
                        // double push
                        new_ep_mask = self.forward::<T::Other>(target_mask);
                    } else if target_mask == self.ep_mask {
                        let target_square = self.forward::<T::Other>(target_mask).first_square();
                        // remove en passent captured pawn
                        self.xor_piece::<T::Other>(Pawn, target_square);
                        self.pawn_hash ^= zobrist_piece::<T::Other>(Pawn, target_square)
                    }
                }
                // update check mask
                let new_checkers = self.pawn_attack::<T::Other>(other_king) & target_mask;
                if new_checkers.is_not_empty() {
                    self.check_mask = new_checkers;
                }
            }
            King => {
                self.hash ^= zobrist_castling(self.castling_rights);
                self.castling_rights[color] = [BitBoard::empty(); 2];
                self.hash ^= zobrist_castling(self.castling_rights);
            }
            Rook => {
                let rights = self.castling_rights[color][0] | self.castling_rights[color][1];
                if (start_mask & rights).is_not_empty() {
                    let clear = start_mask.inverse();

                    // clear castling rights, only the correct side will be cleared
                    self.hash ^= zobrist_castling(self.castling_rights);
                    self.castling_rights[color][0] &= clear;
                    self.castling_rights[color][1] &= clear;
                    self.hash ^= zobrist_castling(self.castling_rights);
                }
            }
            _ => {}
        }

        // clear the old ep zobrist number
        self.hash ^= zobrist_ep(self.ep_mask);

        // if ep is possible then update the mask and add it to the hash
        let enemy_pawns = if T::WHITE {
            self.black_pawns
        } else {
            self.white_pawns
        };
        if new_ep_mask != BitBoard::empty()
            && (self.pawn_attack::<T>(new_ep_mask.first_square()) & enemy_pawns).is_not_empty()
        {
            self.ep_mask = new_ep_mask;
            self.hash ^= zobrist_ep(self.ep_mask);
        } else {
            self.ep_mask = BitBoard::empty();
            self.hash ^= zobrist_ep(self.ep_mask)
        }

        let (bishops, rooks, friendly_pieces, enemy_pieces) = if T::WHITE {
            (
                self.white_bishops | self.white_queens,
                self.white_rooks | self.white_queens,
                self.white_pieces,
                self.black_pieces,
            )
        } else {
            (
                self.black_bishops | self.black_queens,
                self.black_rooks | self.black_queens,
                self.black_pieces,
                self.white_pieces,
            )
        };

        // update diagonal pins and checkers
        for bishop in lookup_bishop(other_king, friendly_pieces) & bishops {
            let between = lookup_between(bishop, other_king) | bishop.bitboard();
            match (between & enemy_pieces).count_ones() {
                0 => self.check_mask &= between,
                1 => self.diagonal_pin_mask |= between,
                _ => {}
            }
        }
        // update orthogonal pins and checkers
        for rook in lookup_rook(other_king, friendly_pieces) & rooks {
            let between = lookup_between(rook, other_king) | rook.bitboard();
            match (between & enemy_pieces).count_ones() {
                0 => self.check_mask &= between,
                1 => self.orthogonal_pin_mask |= between,
                _ => {}
            }
        }

        // toggle side to move
        self.black_to_move = !self.black_to_move;
        self.hash ^= zobrist_player();

        debug_assert!(self.hash == self.calculate_hash());
        debug_assert!(self.pawn_hash == self.calculate_pawn_hash());
    }

    pub fn make_null_move(&mut self) {
        self.hash ^= zobrist_ep(self.ep_mask);
        self.ep_mask = BitBoard::empty();
        self.hash ^= zobrist_ep(self.ep_mask);

        self.black_to_move = !self.black_to_move;
        self.hash ^= zobrist_player();
        self.halfmove_clock += 1;

        // null move can never be made when in check
        self.check_mask = FULL_BOARD;

        // update pin mask for new player
        if self.black_to_move {
            self.calculate_pin_masks::<Black>();
        } else {
            self.calculate_pin_masks::<White>();
        }
    }

    #[inline(always)]
    pub fn calculate_check_mask<T: TypeColor>(&mut self) {
        let (king, enemy_pawns, enemy_knights, enemy_bishops, enemy_rooks) = if T::WHITE {
            (
                self.white_king.first_square(),
                self.black_pawns,
                self.black_knights,
                self.black_bishops | self.black_queens,
                self.black_rooks | self.black_queens,
            )
        } else {
            (
                self.black_king.first_square(),
                self.white_pawns,
                self.white_knights,
                self.white_bishops | self.white_queens,
                self.white_rooks | self.white_queens,
            )
        };
        let pawn_attackers = self.pawn_attack::<T>(king) & enemy_pawns;
        let knight_attackers = lookup_knight(king) & enemy_knights;
        let bishop_attackers = lookup_bishop(king, self.occupied) & enemy_bishops;
        let rook_attackers = lookup_rook(king, self.occupied) & enemy_rooks;

        let all_attackers = pawn_attackers | knight_attackers | bishop_attackers | rook_attackers;

        match all_attackers.count_ones() {
            0 => self.check_mask = FULL_BOARD,
            1 => {
                if pawn_attackers.is_not_empty() {
                    self.check_mask = pawn_attackers;
                } else if knight_attackers.is_not_empty() {
                    self.check_mask = knight_attackers;
                } else if bishop_attackers.is_not_empty() {
                    let bishop = bishop_attackers.first_square();
                    self.check_mask = lookup_between(king, bishop) | bishop_attackers;
                } else {
                    let rook = rook_attackers.first_square();
                    self.check_mask = lookup_between(king, rook) | rook_attackers;
                }
            }
            _ => self.check_mask = BitBoard::empty(),
        }
    }

    #[inline(always)]
    pub fn calculate_pin_masks<T: TypeColor>(&mut self) {
        self.diagonal_pin_mask = BitBoard::empty();
        self.orthogonal_pin_mask = BitBoard::empty();

        let (king, pieces, enemy_bishops, enemy_rooks, enemy_pieces) = if T::WHITE {
            (
                self.white_king.first_square(),
                self.white_pieces,
                self.black_bishops | self.black_queens,
                self.black_rooks | self.black_queens,
                self.black_pieces,
            )
        } else {
            (
                self.black_king.first_square(),
                self.black_pieces,
                self.white_bishops | self.white_queens,
                self.white_rooks | self.white_queens,
                self.white_pieces,
            )
        };

        let diagonals = lookup_bishop(king, enemy_pieces);
        for bishop in diagonals & enemy_bishops {
            let ray = bishop.bitboard() | lookup_between(king, bishop);
            if (ray & pieces).count_ones() == 1 {
                self.diagonal_pin_mask |= ray;
            }
        }

        let orthogonals = lookup_rook(king, enemy_pieces);
        for rook in orthogonals & enemy_rooks {
            let ray = rook.bitboard() | lookup_between(king, rook);
            if (ray & pieces).count_ones() == 1 {
                self.orthogonal_pin_mask |= ray;
            }
        }
    }

    pub fn calculate_hash(&self) -> u64 {
        let mut hash = 0;

        if self.black_to_move {
            hash ^= zobrist_player();
        }

        for wp in self.white_pawns {
            hash ^= zobrist_piece::<White>(Pawn, wp);
        }
        for bp in self.black_pawns {
            hash ^= zobrist_piece::<Black>(Pawn, bp);
        }
        for wn in self.white_knights {
            hash ^= zobrist_piece::<White>(Knight, wn);
        }
        for bn in self.black_knights {
            hash ^= zobrist_piece::<Black>(Knight, bn);
        }
        for wb in self.white_bishops {
            hash ^= zobrist_piece::<White>(Bishop, wb)
        }
        for bb in self.black_bishops {
            hash ^= zobrist_piece::<Black>(Bishop, bb)
        }
        for wr in self.white_rooks {
            hash ^= zobrist_piece::<White>(Rook, wr)
        }
        for br in self.black_rooks {
            hash ^= zobrist_piece::<Black>(Rook, br)
        }
        for wq in self.white_queens {
            hash ^= zobrist_piece::<White>(Queen, wq)
        }
        for bq in self.black_queens {
            hash ^= zobrist_piece::<Black>(Queen, bq)
        }
        for wk in self.white_king {
            hash ^= zobrist_piece::<White>(King, wk)
        }
        for bk in self.black_king {
            hash ^= zobrist_piece::<Black>(King, bk)
        }

        hash ^= zobrist_ep(self.ep_mask);
        hash ^= zobrist_castling(self.castling_rights);

        hash
    }

    pub fn calculate_pawn_hash(&self) -> u64 {
        let mut hash = 0;

        for p in self.white_pawns {
            hash ^= zobrist_piece::<White>(Pawn, p);
        }
        for p in self.black_pawns {
            hash ^= zobrist_piece::<Black>(Pawn, p);
        }

        hash
    }

    pub fn from_fen<T: AsRef<str>>(fen: T) -> Option<Self> {
        let mut fen = fen.as_ref().split_whitespace();
        let pieces = fen.next()?;
        let stm = fen.next()?;
        let castling_rights = fen.next()?;
        let ep_square = fen.next()?;
        let halfmove_clock = fen.next()?;

        let mut white_pawns = BitBoard::empty();
        let mut black_pawns = BitBoard::empty();
        let mut white_knights = BitBoard::empty();
        let mut black_knights = BitBoard::empty();
        let mut white_bishops = BitBoard::empty();
        let mut black_bishops = BitBoard::empty();
        let mut white_rooks = BitBoard::empty();
        let mut black_rooks = BitBoard::empty();
        let mut white_queens = BitBoard::empty();
        let mut black_queens = BitBoard::empty();
        let mut white_king = BitBoard::empty();
        let mut black_king = BitBoard::empty();

        for (row, text) in pieces.split('/').enumerate() {
            let mut col = 0;
            for symbol in text.chars() {
                match symbol {
                    'p' => black_pawns |= BitBoard(1 << (8 * (7 - row) + col)),
                    'P' => white_pawns |= BitBoard(1 << (8 * (7 - row) + col)),
                    'n' => black_knights |= BitBoard(1 << (8 * (7 - row) + col)),
                    'N' => white_knights |= BitBoard(1 << (8 * (7 - row) + col)),
                    'b' => black_bishops |= BitBoard(1 << (8 * (7 - row) + col)),
                    'B' => white_bishops |= BitBoard(1 << (8 * (7 - row) + col)),
                    'r' => black_rooks |= BitBoard(1 << (8 * (7 - row) + col)),
                    'R' => white_rooks |= BitBoard(1 << (8 * (7 - row) + col)),
                    'q' => black_queens |= BitBoard(1 << (8 * (7 - row) + col)),
                    'Q' => white_queens |= BitBoard(1 << (8 * (7 - row) + col)),
                    'k' => black_king |= BitBoard(1 << (8 * (7 - row) + col)),
                    'K' => white_king |= BitBoard(1 << (8 * (7 - row) + col)),
                    n @ '1'..='8' => col += n.to_digit(10)? as usize - 1,
                    _ => return None,
                }
                col += 1;
            }
        }

        let ep_file = match ep_square.chars().nth(0)? {
            'a' => 0,
            'b' => 1,
            'c' => 2,
            'd' => 3,
            'e' => 4,
            'f' => 5,
            'g' => 6,
            'h' => 7,
            '-' => 8,
            _ => return None,
        };
        let ep_rank = match ep_square.chars().nth(1) {
            Some('3') => 2,
            Some('6') => 5,
            _ => 8,
        };

        let ep_mask = if ep_file != 8 {
            BitBoard(1 << (8 * ep_rank + ep_file))
        } else {
            BitBoard::empty()
        };

        let white_pieces =
            white_pawns | white_knights | white_bishops | white_rooks | white_queens | white_king;
        let black_pieces =
            black_pawns | black_knights | black_bishops | black_rooks | black_queens | black_king;
        let occupied = white_pieces | black_pieces;
        let mut board = Self {
            white_pawns,
            black_pawns,
            white_knights,
            black_knights,
            white_bishops,
            black_bishops,
            white_rooks,
            black_rooks,
            white_queens,
            black_queens,
            white_king,
            black_king,
            white_pieces,
            black_pieces,
            occupied,
            castling_rights: [[BitBoard::empty(); 2]; 2],
            check_mask: FULL_BOARD,
            diagonal_pin_mask: BitBoard::empty(),
            orthogonal_pin_mask: BitBoard::empty(),
            black_to_move: stm == "b",
            ep_mask,
            halfmove_clock: halfmove_clock.parse::<u8>().ok()?,
            hash: 0,
            pawn_hash: 0,
        };

        if board.black_to_move {
            board.calculate_check_mask::<Black>();
            board.calculate_pin_masks::<Black>();
        } else {
            board.calculate_check_mask::<White>();
            board.calculate_pin_masks::<White>();
        }

        if castling_rights != "-" {
            for c in castling_rights.chars() {
                let black = c.is_ascii_lowercase();
                let king_file = if black {
                    board.black_king.first_square().file()
                } else {
                    board.white_king.first_square().file()
                } as u64;
                let file = match c.to_ascii_lowercase() {
                    'k' | 'h' => 7,
                    'q' | 'a' => 0,
                    'b' => 1,
                    'c' => 2,
                    'd' => 3,
                    'e' => 4,
                    'f' => 5,
                    'g' => 6,
                    _ => return None,
                } as u64;
                let queenside = file < king_file;
                let mask = BitBoard(1 << (file + (56 * black as u64)));
                board.castling_rights[black as usize][queenside as usize] = mask;
            }
        }

        board.hash = board.calculate_hash();
        board.pawn_hash = board.calculate_pawn_hash();

        Some(board)
    }

    pub fn fen(&self) -> String {
        let mut fen = String::new();
        for rank in (0..8).rev() {
            let mut blank_counter = 0;
            for file in 0..8 {
                let square = Square::from(rank * 8 + file);
                let white = (self.white_pieces & square.bitboard()).is_not_empty();
                if self.piece_on(square).is_some() && blank_counter != 0 {
                    fen += &blank_counter.to_string();
                    blank_counter = 0;
                }
                match self.piece_on(square) {
                    None => blank_counter += 1,
                    Some(Pawn) => fen.push_str(if white { "P" } else { "p" }),
                    Some(Knight) => fen.push_str(if white { "N" } else { "n" }),
                    Some(Bishop) => fen.push_str(if white { "B" } else { "b" }),
                    Some(Rook) => fen.push_str(if white { "R" } else { "r" }),
                    Some(Queen) => fen.push_str(if white { "Q" } else { "q" }),
                    Some(King) => fen.push_str(if white { "K" } else { "k" }),
                }
            }

            if blank_counter != 0 {
                fen += &blank_counter.to_string();
            }

            if rank != 0 {
                fen.push_str("/");
            }
        }

        fen.push_str(if self.black_to_move { " b" } else { " w" });

        if self
            .castling_rights
            .iter()
            .flatten()
            .any(|b| b.is_not_empty())
        {
            let mut rights_string = String::new();
            let rights = [
                self.castling_rights[0][0].first_square(),
                self.castling_rights[0][1].first_square(),
                self.castling_rights[1][0].first_square(),
                self.castling_rights[1][1].first_square(),
            ];

            for (i, &right) in rights.iter().enumerate() {
                if *right != 64 {
                    let letter = match right.file() {
                        0 => "a",
                        1 => "b",
                        2 => "c",
                        3 => "d",
                        4 => "e",
                        5 => "f",
                        6 => "g",
                        7 => "h",
                        _ => unreachable!(),
                    };
                    let uppercase = letter.to_ascii_uppercase();
                    rights_string.push_str(if i / 2 == 0 { &uppercase } else { letter });
                }
            }

            if rights_string
                .chars()
                .all(|c| matches!(c, 'a' | 'A' | 'h' | 'H'))
            {
                rights_string = rights_string
                    .chars()
                    .map(|c| match c {
                        'a' => 'q',
                        'A' => 'Q',
                        'h' => 'k',
                        'H' => 'K',
                        _ => unreachable!(),
                    })
                    .collect::<String>();
            }

            fen.push(' ');
            fen.push_str(&rights_string);
        } else {
            fen.push_str(" -");
        }

        if self.ep_mask.is_not_empty() {
            fen.push_str(&self.ep_mask.first_square().coord());
        } else {
            fen.push_str(" -")
        }

        fen.push_str(&format!(" {} 1", self.halfmove_clock));

        fen
    }

    pub fn dump_state(&self) {
        println!("White Pawns:\n{}\n", self.white_pawns);
        println!("Black Pawns:\n{}\n", self.black_pawns);
        println!("White Knights:\n{}\n", self.white_knights);
        println!("Black Knights:\n{}\n", self.black_knights);
        println!("White Bishops:\n{}\n", self.white_bishops);
        println!("Black Bishops:\n{}\n", self.black_bishops);
        println!("White Rooks:\n{}\n", self.white_rooks);
        println!("Black Rooks:\n{}\n", self.black_rooks);
        println!("White Queens:\n{}\n", self.white_queens);
        println!("Black Queens:\n{}\n", self.black_queens);
        println!("White King:\n{}\n", self.white_king);
        println!("Black King:\n{}\n", self.black_king);

        println!("White Pieces:\n{}\n", self.white_pieces);
        println!("Black Pieces:\n{}\n", self.black_pieces);
        println!(
            "White Castling Rights:\n[{}, {}]\n",
            self.castling_rights[0][0], self.castling_rights[0][1]
        );
        println!(
            "Black Castling Rights:\n[{}, {}]\n",
            self.castling_rights[1][0], self.castling_rights[1][1]
        );
        println!(
            "Pin Masks:\n{}\n",
            self.diagonal_pin_mask | self.orthogonal_pin_mask
        );
        println!("Check Mask:\n{}\n", self.check_mask);
        println!("EP mask:\n{}\n", self.ep_mask);
        println!("WTM: {}", !self.black_to_move);
    }
}
