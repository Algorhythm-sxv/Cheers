use crate::{
    lookup_tables::*,
    moves::*,
    types::{
        CastlingIndex::*,
        CastlingRights, ColorIndex,
        ColorIndex::*,
        ColorMasks,
        PieceIndex::{self, *},
        PieceMasks,
    },
    zobrist::*,
};
use cheers_bitboards::{BitBoard, Square};

pub mod eval_params;
pub mod eval_types;
pub mod evaluate;
pub mod fen;
pub mod movegen;
pub mod see;

pub use self::eval_params::*;
use self::movegen::{All, MoveList};

#[derive(Clone)]
pub struct ChessGame {
    color_masks: ColorMasks,
    combined: BitBoard,
    piece_masks: PieceMasks,
    current_player: ColorIndex,
    castling_rights: CastlingRights,
    en_passent_mask: BitBoard,
    halfmove_clock: u8,
    hash: u64,
    pawn_hash: u64,
    position_history: Vec<u64>,
    unmove_history: Vec<UnMove>,
}

impl ChessGame {
    pub fn new() -> Self {
        let mut boards = Self {
            color_masks: ColorMasks::default(),
            combined: BitBoard::empty(),
            piece_masks: PieceMasks::default(),
            current_player: ColorIndex::default(),
            castling_rights: CastlingRights::default(),
            en_passent_mask: BitBoard::empty(),
            halfmove_clock: 0,
            hash: 0,
            pawn_hash: 0,
            position_history: Vec::new(),
            unmove_history: Vec::new(),
        };
        boards.combined = boards.color_masks[White] | boards.color_masks[Black];
        boards.reset();
        boards
    }

    pub fn reset(&mut self) {
        self.set_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap()
    }

    #[inline]
    pub fn piece_masks(&self) -> PieceMasks {
        self.piece_masks
    }

    #[inline]
    pub fn en_passent_square(&self) -> Option<Square> {
        match self.en_passent_mask.first_square() {
            Square::NULL => None,
            sq => Some(sq),
        }
    }

    #[inline]
    pub fn current_player(&self) -> ColorIndex {
        self.current_player
    }

    #[inline]
    pub fn combined(&self) -> BitBoard {
        self.combined
    }

    #[inline]
    pub fn halfmove_clock(&self) -> u8 {
        self.halfmove_clock
    }

    #[inline]
    pub fn position_history(&self) -> &[u64] {
        &self.position_history
    }

    #[inline]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    #[inline]
    pub fn pawn_hash(&self) -> u64 {
        self.pawn_hash
    }

    #[inline]
    pub fn piece_at(&self, square: Square) -> PieceIndex {
        let test = square.bitboard();
        if (self.combined & test).is_empty() {
            NoPiece
        } else {
            let pawns = self.piece_masks[(White, Pawn)] | self.piece_masks[(Black, Pawn)];
            let knights = self.piece_masks[(White, Knight)] | self.piece_masks[(Black, Knight)];
            let bishops = self.piece_masks[(White, Bishop)] | self.piece_masks[(Black, Bishop)];
            let rooks = self.piece_masks[(White, Rook)] | self.piece_masks[(Black, Rook)];
            let queens = self.piece_masks[(White, Queen)] | self.piece_masks[(Black, Queen)];

            if ((pawns | knights | bishops) & test).is_not_empty() {
                if (pawns & test).is_not_empty() {
                    Pawn
                } else if (knights & test).is_not_empty() {
                    Knight
                } else {
                    Bishop
                }
            } else if (rooks & test).is_not_empty() {
                Rook
            } else if (queens & test).is_not_empty() {
                Queen
            } else {
                King
            }
        }
    }

    pub fn color_at(&self, square: Square) -> ColorIndex {
        if (self.color_masks[White] & square.bitboard()).is_not_empty() {
            White
        } else {
            Black
        }
    }

    pub fn has_non_pawn_material(&self, color: ColorIndex) -> bool {
        !(self.piece_masks[(color, Knight)]
            | self.piece_masks[(color, Bishop)]
            | self.piece_masks[(color, Rook)]
            | self.piece_masks[(color, Queen)])
            .is_empty()
    }

    fn pawn_attacks(&self, color: ColorIndex) -> BitBoard {
        match color {
            White => {
                let pawns = self.piece_masks[(White, Pawn)];
                let west_attacks = (pawns << 7) & NOT_H_FILE;
                let east_attacks = (pawns << 9) & NOT_A_FILE;

                west_attacks | east_attacks
            }
            Black => {
                let pawns = self.piece_masks[(Black, Pawn)];
                let west_attacks = (pawns >> 9) & NOT_H_FILE;
                let east_attacks = (pawns >> 7) & NOT_A_FILE;

                west_attacks | east_attacks
            }
        }
    }

    pub fn pawn_push_span(&self, square: Square, color: ColorIndex) -> BitBoard {
        let start = square.bitboard();

        let mut board = BitBoard::empty();
        if color == White {
            board |= start << 8 | start << 16;
            board |= board << 16;
            board |= board << 32;
        } else {
            board |= start >> 8 | start >> 16;
            board |= board >> 16;
            board |= board >> 32;
        }

        board
    }

    pub fn pawn_push_spans(&self, pawns: BitBoard, color: ColorIndex) -> BitBoard {
        let mut board = BitBoard::empty();
        if color == White {
            board |= pawns << 8 | pawns << 16;
            board |= board << 16;
            board |= board << 32;
        } else {
            board |= pawns >> 8 | pawns >> 16;
            board |= board >> 16;
            board |= board >> 32;
        }
        board
    }

    pub fn pawn_front_spans(&self, color: ColorIndex, pawns: BitBoard) -> BitBoard {
        let mut spans = pawns;
        if color == White {
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

    pub fn pawn_attack_spans(&self, color: ColorIndex) -> BitBoard {
        let mut spans = self.pawn_attacks(color);
        if color == White {
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

    fn knight_attacks(&self, color: ColorIndex) -> BitBoard {
        let knights = self.piece_masks[(color, Knight)];

        let mut result = BitBoard::empty();
        for i in knights {
            result |= lookup_knight(i);
        }
        result
    }

    fn bishop_attacks(&self, color: ColorIndex, blocking_mask: BitBoard) -> BitBoard {
        let bishops = self.piece_masks[(color, Bishop)];

        let mut result = BitBoard::empty();
        for i in bishops {
            result |= lookup_bishop(i, blocking_mask);
        }
        result
    }

    fn rook_attacks(&self, color: ColorIndex, blocking_mask: BitBoard) -> BitBoard {
        let rooks = self.piece_masks[(color, Rook)];

        let mut result = BitBoard::empty();
        for i in rooks {
            result |= lookup_rook(i, blocking_mask);
        }
        result
    }

    fn queen_attacks(&self, color: ColorIndex, blocking_mask: BitBoard) -> BitBoard {
        let queens = self.piece_masks[(color, Queen)];

        let mut result = BitBoard::empty();
        for i in queens {
            result |= lookup_queen(i, blocking_mask);
        }
        result
    }

    fn king_attacks(&self, color: ColorIndex) -> BitBoard {
        let king = self.piece_masks[(color, King)];
        lookup_king(king.first_square())
    }

    fn discovered_attacks(&self, square: Square, color: ColorIndex) -> BitBoard {
        let rook_attacks = lookup_rook(square, self.combined);
        let bishop_attacks = lookup_bishop(square, self.combined);

        let rooks = self.piece_masks[(!color, Rook)] & rook_attacks.inverse();
        let bishops = self.piece_masks[(!color, Bishop)] & bishop_attacks.inverse();

        (rooks & lookup_rook(square, self.combined & rook_attacks.inverse()))
            | (bishops & lookup_bishop(square, self.combined & bishop_attacks.inverse()))
    }

    fn all_attacks(&self, color: ColorIndex, blocking_mask: BitBoard) -> BitBoard {
        self.pawn_attacks(color)
            | self.knight_attacks(color)
            | self.king_attacks(color)
            | self.bishop_attacks(color, blocking_mask)
            | self.rook_attacks(color, blocking_mask)
            | self.queen_attacks(color, blocking_mask)
    }

    fn all_attacks_on(&self, target: Square, blocking_mask: BitBoard) -> BitBoard {
        let knights = self.piece_masks[(White, Knight)] | self.piece_masks[(Black, Knight)];
        let bishops = self.piece_masks[(White, Bishop)]
            | self.piece_masks[(Black, Bishop)]
            | self.piece_masks[(White, Queen)]
            | self.piece_masks[(Black, Queen)];
        let rooks = self.piece_masks[(White, Rook)]
            | self.piece_masks[(Black, Rook)]
            | self.piece_masks[(White, Queen)]
            | self.piece_masks[(Black, Queen)];
        let kings = self.piece_masks[(White, King)] | self.piece_masks[(Black, King)];

        (lookup_pawn_attack(target, White) & self.piece_masks[(Black, Pawn)])
            | (lookup_pawn_attack(target, Black) & self.piece_masks[(White, Pawn)])
            | (lookup_knight(target) & knights)
            | (lookup_bishop(target, blocking_mask) & bishops)
            | (lookup_rook(target, blocking_mask) & rooks)
            | (lookup_king(target) & kings)
    }

    pub fn in_check(&self, color: ColorIndex) -> bool {
        (self.all_attacks(!color, self.combined) & self.piece_masks[(color, King)]).is_not_empty()
    }

    pub fn is_pseudolegal(&self, start: Square, target: Square) -> bool {
        if start == target {
            return true;
        }

        let piece = self.piece_at(start);
        let color = self.current_player;

        match piece {
            Pawn => {
                let d = (target).abs_diff(*start);
                if d % 8 != 0 {
                    // captures
                    (self.pawn_attacks(color)
                        & (self.color_masks[!color] | self.en_passent_mask)
                        & target.bitboard())
                    .is_not_empty()
                } else {
                    // pushes
                    let push_one = lookup_pawn_push(start, color) & (self.combined).inverse();
                    if d == 8 && (push_one & target.bitboard()).is_not_empty() {
                        true
                    } else if d == 16 && push_one.is_not_empty() {
                        (lookup_pawn_push(push_one.first_square(), color)
                            & (self.combined).inverse()
                            & target.bitboard())
                        .is_not_empty()
                    } else {
                        false
                    }
                }
            }
            Knight => {
                (self.knight_attacks(color) & self.color_masks[color].inverse() & target.bitboard())
                    .is_not_empty()
            }
            Bishop => (self.bishop_attacks(color, self.combined)
                & self.color_masks[color].inverse()
                & target.bitboard())
            .is_not_empty(),
            Rook => (self.rook_attacks(color, self.combined)
                & self.color_masks[color].inverse()
                & target.bitboard())
            .is_not_empty(),
            Queen => (self.queen_attacks(color, self.combined)
                & self.color_masks[color].inverse()
                & target.bitboard())
            .is_not_empty(),
            King => {
                (self.king_attacks(color) & self.color_masks[color].inverse() & target.bitboard())
                    .is_not_empty()
            }
            NoPiece => false,
        }
    }

    pub fn make_move(&mut self, move_: Move) {
        let color = self.current_player;
        let start = move_.start();
        let target = move_.target();
        let piece = move_.piece();

        let captured = if move_.en_passent() {
            Pawn
        } else {
            self.piece_at(target)
        };

        // Update unmove history
        self.unmove_history.push(UnMove::new(
            start,
            target,
            move_.promotion() != NoPiece,
            captured,
            move_.en_passent(),
            self.en_passent_mask,
            move_.castling(),
            self.castling_rights,
            self.halfmove_clock,
            self.pawn_hash,
        ));

        // add the last position into the history
        self.position_history.push(self.hash);

        // increment the halfmove clock for 50-move rule
        self.halfmove_clock += 1;

        // Castling
        if move_.castling() {
            let dx = *target as isize - *start as isize;
            let (rook_start, rook_target) = if dx == 2 {
                // Kingside
                (target.offset(1, 0), target.offset(-1, 0))
            } else {
                // Queenside
                (target.offset(-2, 0), target.offset(1, 0))
            };

            // update king position and hash
            self.hash ^= zobrist_piece(King, color, start) ^ zobrist_piece(King, color, target);
            self.piece_masks[(color, King)] ^= target.bitboard() | start.bitboard();
            // update rook position and hash
            self.hash ^=
                zobrist_piece(Rook, color, rook_start) ^ zobrist_piece(Rook, color, rook_target);
            self.piece_masks[(color, Rook)] ^= rook_target.bitboard() | rook_start.bitboard();
            // update color masks
            self.color_masks[color] ^= start.bitboard()
                | target.bitboard()
                | rook_start.bitboard()
                | rook_target.bitboard();
            // update castling rights
            self.hash ^= zobrist_castling(self.castling_rights);
            self.castling_rights[color] = [false, false];
            self.hash ^= zobrist_castling(self.castling_rights);
        }

        // Remove captured piece (en passent, rule 50)
        if captured != NoPiece {
            let cap_square = if move_.en_passent() {
                let target = if color == White {
                    target.offset(0, -1)
                } else {
                    target.offset(0, 1)
                };
                self.pawn_hash ^= zobrist_piece(Pawn, !color, target);
                target
            } else {
                target
            };
            // remove piece from target square
            self.hash ^= zobrist_piece(captured, !color, cap_square);
            self.piece_masks[(!color, captured)] ^= cap_square.bitboard();
            self.color_masks[!color] ^= cap_square.bitboard();

            // reset halfmove clock
            self.halfmove_clock = 0;
        }

        // reset en passent square
        if self.en_passent_mask.is_not_empty() {
            self.hash ^= zobrist_enpassent(self.en_passent_mask);
            self.en_passent_mask = BitBoard::empty();
        }

        // update castling rights
        if piece == King {
            self.hash ^= zobrist_castling(self.castling_rights);
            self.castling_rights[color] = [false, false];
            self.hash ^= zobrist_castling(self.castling_rights);
        } else if piece == Rook {
            if self.castling_rights[(color, Kingside)] && *start as usize == 7 + 56 * color as usize
            {
                // kingside rook has made first move
                self.hash ^= zobrist_castling(self.castling_rights);
                self.castling_rights[(color, Kingside)] = false;
                self.hash ^= zobrist_castling(self.castling_rights);
            } else if self.castling_rights[(color, Queenside)]
                && *start as usize == 56 * color as usize
            {
                // queenside rook has made first move
                self.hash ^= zobrist_castling(self.castling_rights);
                self.castling_rights[(color, Queenside)] = false;
                self.hash ^= zobrist_castling(self.castling_rights);
            }
        }
        match captured {
            Rook => {
                if self.castling_rights[(!color, Kingside)]
                    && *target as usize == 7 + 56 * !color as usize
                {
                    // kingside rook has been captured
                    self.hash ^= zobrist_castling(self.castling_rights);
                    self.castling_rights[(!color, Kingside)] = false;
                    self.hash ^= zobrist_castling(self.castling_rights);
                } else if self.castling_rights[(!color, Queenside)]
                    && *target as usize == 56 * !color as usize
                {
                    // queenside rook has been captured
                    self.hash ^= zobrist_castling(self.castling_rights);
                    self.castling_rights[(!color, Queenside)] = false;
                    self.hash ^= zobrist_castling(self.castling_rights);
                }
            }
            Pawn => {
                if !move_.en_passent() {
                    self.pawn_hash ^= zobrist_piece(Pawn, !color, target)
                }
            }
            _ => {}
        }

        // move the piece
        if !move_.castling() {
            self.hash ^= zobrist_piece(piece, color, start) ^ zobrist_piece(piece, color, target);
            self.piece_masks[(color, piece)] ^= start.bitboard() | target.bitboard();
            self.color_masks[color] ^= start.bitboard() | target.bitboard();
        }

        // pawn special cases
        if piece == Pawn {
            self.pawn_hash ^=
                zobrist_piece(Pawn, color, start) ^ zobrist_piece(Pawn, color, target);
            // en passent square
            if move_.double_pawn_push() {
                let ep_square: Square = if color == White {
                    target.offset(0, -1)
                } else {
                    target.offset(0, 1)
                };
                // only set the ep mask if the pawn can be taken
                self.en_passent_mask = ep_square.bitboard() & self.pawn_attacks(!color);
                if self.en_passent_mask.is_not_empty() {
                    self.hash ^= zobrist_enpassent(self.en_passent_mask);
                }
            }
            // promotion
            if move_.promotion() != NoPiece {
                self.hash ^= zobrist_piece(Pawn, color, target)
                    ^ zobrist_piece(move_.promotion(), color, target);
                self.pawn_hash ^= zobrist_piece(Pawn, color, target);
                self.piece_masks[(color, Pawn)] ^= target.bitboard();
                self.piece_masks[(color, move_.promotion())] |= target.bitboard();
            }
            // rule 50
            self.halfmove_clock = 0;
        }

        // swap players
        self.hash ^= zobrist_player();
        self.current_player = !self.current_player;

        // update combined mask
        self.combined = self.color_masks[White] | self.color_masks[Black];
    }

    pub fn unmake_move(&mut self) {
        self.current_player = !self.current_player;

        let unmove = self.unmove_history.pop().unwrap();
        let start = unmove.start;
        let target = unmove.target;

        let mut piece = self.piece_at(target);
        if unmove.promotion {
            self.piece_masks[(self.current_player, piece)] ^= target.bitboard();

            self.piece_masks[(self.current_player, Pawn)] ^= target.bitboard();
            piece = Pawn;
        }

        if unmove.castling {
            if target.file() == 2 {
                // queenside
                self.piece_masks[(self.current_player, King)] ^=
                    start.bitboard() | target.bitboard();

                let rook_start: Square = target.offset(-2, 0);
                let rook_target: Square = target.offset(1, 0);

                self.piece_masks[(self.current_player, Rook)] ^=
                    rook_start.bitboard() | rook_target.bitboard();

                self.color_masks[self.current_player] ^= start.bitboard()
                    | target.bitboard()
                    | rook_start.bitboard()
                    | rook_target.bitboard();
            } else {
                // kingside
                self.piece_masks[(self.current_player, King)] ^=
                    start.bitboard() | target.bitboard();

                let rook_start: Square = target.offset(1, 0);
                let rook_target: Square = target.offset(-1, 0);

                self.piece_masks[(self.current_player, Rook)] ^=
                    rook_start.bitboard() | rook_target.bitboard();

                self.color_masks[self.current_player] ^= start.bitboard()
                    | target.bitboard()
                    | rook_start.bitboard()
                    | rook_target.bitboard();
            }
        } else {
            // move piece back to start
            self.piece_masks[(self.current_player, piece)] ^= start.bitboard() | target.bitboard();
            self.color_masks[self.current_player] ^= start.bitboard() | target.bitboard();

            if unmove.capture != NoPiece {
                let mut cap_square = target;
                if unmove.en_passent {
                    cap_square = match self.current_player {
                        White => target.offset(0, -1),
                        Black => target.offset(0, 1),
                    };
                }
                // replace captured piece
                self.piece_masks[(!self.current_player, unmove.capture)] ^= cap_square.bitboard();
                self.color_masks[!self.current_player] ^= cap_square.bitboard();
            }
        }

        // restore board state
        self.castling_rights = unmove.castling_rights;
        self.en_passent_mask = unmove.en_passent_mask;
        self.hash = self.position_history.pop().unwrap();
        self.halfmove_clock = unmove.halfmove_clock;

        self.combined = self.color_masks[White] | self.color_masks[Black];

        self.pawn_hash = unmove.pawn_hash;
    }

    pub fn make_null_move(&mut self) {
        let unmove = UnMove::new(
            Square::A1,
            Square::A1,
            false,
            NoPiece,
            false,
            self.en_passent_mask,
            false,
            self.castling_rights,
            0,
            self.pawn_hash,
        );

        self.unmove_history.push(unmove);
        self.position_history.push(self.hash);

        if self.en_passent_mask.is_not_empty() {
            self.hash ^= zobrist_enpassent(self.en_passent_mask);
            self.en_passent_mask = BitBoard::empty();
        }

        self.hash ^= zobrist_player();
        self.halfmove_clock += 1;
        self.current_player = !self.current_player;
    }

    pub fn unmake_null_move(&mut self) {
        let unmove = self.unmove_history.pop().unwrap();

        self.en_passent_mask = unmove.en_passent_mask;
        self.current_player = !self.current_player;
        self.halfmove_clock -= 1;
        self.hash = self.position_history.pop().unwrap();
    }

    pub fn zobrist_hash(&self) -> u64 {
        let mut hash = 0u64;
        // pieces
        for piece in [Pawn, Knight, Bishop, Rook, Queen, King] {
            for color in [White, Black] {
                let pieces = self.piece_masks[(color, piece)];
                for square in pieces {
                    hash ^= zobrist_piece(piece, color, square);
                }
            }
        }

        // side to move
        if self.current_player == Black {
            hash ^= zobrist_player();
        }

        // castling rights
        hash ^= zobrist_castling(self.castling_rights);

        // en passent square
        if self.en_passent_mask.is_not_empty() {
            hash ^= zobrist_enpassent(self.en_passent_mask);
        }

        hash
    }

    pub fn zobrist_pawn_hash(&self) -> u64 {
        let mut hash = 0u64;
        for color in [White, Black] {
            for pawn in self.piece_masks[(color, Pawn)] {
                hash ^= zobrist_piece(Pawn, color, pawn);
            }
        }
        hash
    }

    pub fn perft(&self, depth: usize) -> usize {
        let copy = self.clone();

        let mut perft = Perft::new(copy, depth);
        perft.run()
    }

    pub fn divide(&mut self, depth: usize) {
        if depth == 0 {
            return;
        }
        let moves = self.legal_moves();
        let mut move_count = 0;
        let mut node_count = 0;
        for &move_ in moves.inner() {
            move_count += 1;
            self.make_move(move_);
            let nodes = self.perft(depth - 1);
            self.unmake_move();
            node_count += nodes;
            println!(
                "{}{}: {}",
                move_.coords(),
                match move_.promotion() {
                    Knight => "=N",
                    Bishop => "=B",
                    Rook => "=R",
                    Queen => "=Q",
                    _ => "",
                },
                nodes
            );
        }
        println!("Moves: {}, Nodes: {}\n", move_count, node_count);
    }
}

pub struct Perft {
    game: ChessGame,
    depth: usize,
    move_lists: Vec<MoveList>,
}

impl Perft {
    pub fn new(game: ChessGame, depth: usize) -> Self {
        Self {
            game,
            depth,
            move_lists: vec![MoveList::new(); depth + 1],
        }
    }

    pub fn perft(&mut self, depth: usize) -> usize {
        if depth == 1 {
            self.game
                .generate_legal_moves::<All>(&mut self.move_lists[depth]);
            return self.move_lists[depth].len();
        } else if depth == 0 {
            return 1;
        }

        let mut nodes = 0;
        self.game
            .generate_legal_moves::<All>(&mut self.move_lists[depth]);

        for i in 0..self.move_lists[depth].len() {
            let move_ = self.move_lists[depth][i];
            self.game.make_move(move_);
            nodes += self.perft(depth - 1);
            self.game.unmake_move();
        }
        nodes
    }

    pub fn run(&mut self) -> usize {
        self.perft(self.depth)
    }
}

#[cfg(test)]
mod tests {
    use crate::{chessgame::ChessGame, search::Search};

    #[test]
    fn search_speed() -> Result<(), ()> {
        let game = ChessGame::new();

        let search = Search::new(game).max_depth(Some(8)).tt_size_mb(64);
        search.search();

        Ok(())
    }
}
