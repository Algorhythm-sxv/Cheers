use crate::{
    lookup_tables::*,
    moves::*,
    transposition_table::TranspositionTable,
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

mod evaluate;
mod piece_tables;
mod search;
pub use search::*;

#[allow(dead_code)]
fn print_bitboard(board: u64) {
    let board_string = format!("{:064b}", board);
    let ranks = board_string
        .chars()
        .rev()
        .map(|c| if c == '0' { "." } else { "1" })
        .collect::<Vec<_>>()
        .chunks(8)
        .map(|c| c.join(" "))
        .rev()
        .collect::<Vec<String>>();

    println!("8  {}", ranks[0]);
    println!("7  {}", ranks[1]);
    println!("6  {}", ranks[2]);
    println!("5  {}", ranks[3]);
    println!("4  {}", ranks[4]);
    println!("3  {}", ranks[5]);
    println!("2  {}", ranks[6]);
    println!("1  {}", ranks[7]);
    println!("\n   a b c d e f g h")
}

#[derive(Clone)]
pub struct BitBoards {
    color_masks: ColorMasks,
    piece_masks: PieceMasks,
    piece_list: [PieceIndex; 64],
    current_player: ColorIndex,
    castling_rights: CastlingRights,
    en_passent_mask: u64,
    halfmove_clock: u8,
    hash: u64,
    position_history: Vec<u64>,
    unmove_history: Vec<UnMove>,
    transposition_table: TranspositionTable,
}

impl BitBoards {
    pub fn new(tt: TranspositionTable) -> Self {
        let mut boards = Self {
            color_masks: ColorMasks::default(),
            piece_masks: PieceMasks::default(),
            piece_list: [NoPiece; 64],
            current_player: ColorIndex::default(),
            castling_rights: CastlingRights::default(),
            en_passent_mask: 0,
            halfmove_clock: 0,
            hash: 0,
            position_history: Vec::new(),
            unmove_history: Vec::new(),
            transposition_table: tt,
        };
        boards
            .set_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        boards
    }

    pub fn reset(&mut self) {
        *self = Self {
            color_masks: ColorMasks::default(),
            piece_masks: PieceMasks::default(),
            piece_list: [NoPiece; 64],
            current_player: ColorIndex::default(),
            castling_rights: CastlingRights::default(),
            en_passent_mask: 0,
            halfmove_clock: 0,
            hash: 0,
            position_history: Vec::new(),
            unmove_history: Vec::new(),
            transposition_table: self.transposition_table.clone(),
        };
        self.set_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap()
    }

    pub fn set_from_fen(
        &mut self,
        fen: impl Into<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.piece_masks = PieceMasks([[0; 6]; 2]);
        self.color_masks = ColorMasks([0; 2]);
        self.piece_list = [NoPiece; 64];

        let fen = fen.into();
        let mut lines = fen.split(&['/', ' '][..]);

        for (i, line) in lines.clone().take(8).enumerate() {
            let mut index = 56 - i * 8;
            for chr in line.chars() {
                match chr {
                    'n' => {
                        self.piece_masks[(Black, Knight)] |= 1 << index;
                        self.color_masks[Black] |= 1 << index;
                        self.piece_list[index] = Knight;
                    }
                    'N' => {
                        self.piece_masks[(White, Knight)] |= 1 << index;
                        self.color_masks[White] |= 1 << index;
                        self.piece_list[index] = Knight;
                    }
                    'b' => {
                        self.piece_masks[(Black, Bishop)] |= 1 << index;
                        self.color_masks[Black] |= 1 << index;
                        self.piece_list[index] = Bishop;
                    }
                    'B' => {
                        self.piece_masks[(White, Bishop)] |= 1 << index;
                        self.color_masks[White] |= 1 << index;
                        self.piece_list[index] = Bishop;
                    }
                    'r' => {
                        self.piece_masks[(Black, Rook)] |= 1 << index;
                        self.color_masks[Black] |= 1 << index;
                        self.piece_list[index] = Rook;
                    }
                    'R' => {
                        self.piece_masks[(White, Rook)] |= 1 << index;
                        self.color_masks[White] |= 1 << index;
                        self.piece_list[index] = Rook;
                    }
                    'q' => {
                        self.piece_masks[(Black, Queen)] |= 1 << index;
                        self.color_masks[Black] |= 1 << index;
                        self.piece_list[index] = Queen;
                    }
                    'Q' => {
                        self.piece_masks[(White, Queen)] |= 1 << index;
                        self.color_masks[White] |= 1 << index;
                        self.piece_list[index] = Queen;
                    }
                    'k' => {
                        self.piece_masks[(Black, King)] |= 1 << index;
                        self.color_masks[Black] |= 1 << index;
                        self.piece_list[index] = King;
                    }
                    'K' => {
                        self.piece_masks[(White, King)] |= 1 << index;
                        self.color_masks[White] |= 1 << index;
                        self.piece_list[index] = King;
                    }
                    'p' => {
                        self.piece_masks[(Black, Pawn)] |= 1 << index;
                        self.color_masks[Black] |= 1 << index;
                        self.piece_list[index] = Pawn;
                    }
                    'P' => {
                        self.piece_masks[(White, Pawn)] |= 1 << index;
                        self.color_masks[White] |= 1 << index;
                        self.piece_list[index] = Pawn;
                    }
                    digit @ '1'..='8' => index += digit.to_digit(10).unwrap() as usize - 1,
                    other => eprintln!("Unexpected character in FEN: {}", other),
                }
                index += 1;
            }
        }

        match lines.nth(8).ok_or_else(|| String::from("No metadata!"))? {
            "w" => self.current_player = White,
            "b" => self.current_player = Black,
            other => return Err(format!("Invalid player character: {}", other).into()),
        }

        self.castling_rights = CastlingRights([[false, false], [false, false]]);
        match lines
            .next()
            .ok_or_else(|| String::from("Insufficient metadata for castling rights!"))?
        {
            "-" => self.castling_rights = CastlingRights([[false, false], [false, false]]),
            other => other.chars().try_for_each(|chr| match chr {
                'K' => {
                    self.castling_rights[(White, Kingside)] = true;
                    Ok(())
                }
                'k' => {
                    self.castling_rights[(Black, Kingside)] = true;
                    Ok(())
                }
                'Q' => {
                    self.castling_rights[(White, Queenside)] = true;
                    Ok(())
                }
                'q' => {
                    self.castling_rights[(Black, Queenside)] = true;
                    Ok(())
                }
                _ => Err(format!("Invalid player character: {}", other)),
            })?,
        }

        match lines
            .next()
            .ok_or_else(|| String::from("Insufficient metadata for en passent square!"))?
        {
            "-" => self.en_passent_mask = 0,
            other => {
                let mut square = 0;
                match other
                    .as_bytes()
                    .get(0)
                    .ok_or_else(|| "Empty en passent string!".to_string())?
                {
                    file @ b'a'..=b'h' => square += file - b'a',
                    other => return Err(format!("Invalid en passent file: {}", other).into()),
                }
                match other
                    .as_bytes()
                    .get(1)
                    .ok_or_else(|| "En passent string too short".to_string())?
                {
                    rank @ b'1'..=b'8' => square += 8 * (rank - b'1'),
                    other => return Err(format!("Invalid en passent rank: {}", other).into()),
                }
                self.en_passent_mask = 1 << square;
            }
        }

        self.halfmove_clock = lines
            .next()
            .ok_or_else(|| String::from("No halfmove clock!"))?
            .parse::<u8>()?;

        let hash = self.zobrist_hash();
        self.hash = hash;

        Ok(())
    }

    pub fn enpassent_square(&self) -> usize {
        self.en_passent_mask.trailing_zeros() as usize
    }

    pub fn current_player(&self) -> ColorIndex {
        self.current_player
    }

    pub fn piece_at(&self, square: usize) -> PieceIndex {
        self.piece_list[square]
        // if (self.color_masks[White] | self.color_masks[Black]) & (1 << square) == 0 {
        //     return NoPiece;
        // }
        // for piece in [Pawn, Knight, Bishop, Rook, Queen, King] {
        //     if (self.piece_masks[(White, piece)] | self.piece_masks[(Black, piece)]) & (1 << square)
        //         != 0
        //     {
        //         return piece;
        //     }
        // }
        // NoPiece
    }

    fn pawn_attacks(&self, color: ColorIndex) -> u64 {
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

    pub fn pawn_front_spans(&self, color: ColorIndex) -> u64 {
        let mut spans = self.piece_masks[(color, Pawn)];
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

    fn knight_attacks(&self, color: ColorIndex) -> u64 {
        let mut knights = self.piece_masks[(color, Knight)];

        let mut result = 0;
        while knights != 0 {
            let i = knights.trailing_zeros() as usize;
            result |= lookup_tables().lookup_knight(i);
            knights ^= 1 << i;
        }
        result
    }

    fn bishop_attacks(&self, color: ColorIndex, blocking_mask: u64) -> u64 {
        let mut bishops = self.piece_masks[(color, Bishop)];

        let mut result = 0;
        while bishops != 0 {
            let i = bishops.trailing_zeros() as usize;
            result |= lookup_tables().lookup_bishop(i, blocking_mask);
            bishops ^= 1 << i;
        }
        result
    }

    fn rook_attacks(&self, color: ColorIndex, blocking_mask: u64) -> u64 {
        let mut rooks = self.piece_masks[(color, Rook)];

        let mut result = 0;
        while rooks != 0 {
            let i = rooks.trailing_zeros() as usize;
            result |= lookup_tables().lookup_rook(i, blocking_mask);
            rooks ^= 1 << i;
        }
        result
    }

    fn queen_attacks(&self, color: ColorIndex, blocking_mask: u64) -> u64 {
        let mut queens = self.piece_masks[(color, Queen)];

        let mut result = 0;
        while queens != 0 {
            let i = queens.trailing_zeros() as usize;
            result |= lookup_tables().lookup_queen(i, blocking_mask);
            queens ^= 1 << i;
        }
        result
    }

    fn king_attacks(&self, color: ColorIndex) -> u64 {
        let king = self.piece_masks[(color, King)];
        lookup_tables().lookup_king(king.trailing_zeros() as usize)
    }

    fn all_attacks(&self, color: ColorIndex, blocking_mask: u64) -> u64 {
        self.pawn_attacks(color)
            | self.knight_attacks(color)
            | self.king_attacks(color)
            | self.bishop_attacks(color, blocking_mask)
            | self.rook_attacks(color, blocking_mask)
            | self.queen_attacks(color, blocking_mask)
    }

    pub fn in_check(&self, color: ColorIndex) -> bool {
        self.all_attacks(!color, self.color_masks[White] | self.color_masks[Black])
            & self.piece_masks[(color, King)]
            != 0
    }

    pub fn is_pseudolegal(&self, start: u8, target: u8) -> bool {
        let piece = self.piece_at(start as usize);
        let color = self.current_player;

        match piece {
            Pawn => {
                // TODO: replace with abs_diff
                let d = (target as i8 - start as i8).abs();
                if d % 8 != 0 {
                    // captures
                    self.pawn_attacks(color)
                        & (self.color_masks[!color] | self.en_passent_mask)
                        & (1 << target)
                        != 0
                } else {
                    // pushes
                    let push_one = lookup_tables().lookup_pawn_push(start as usize, color)
                        & !(self.color_masks[White] | self.color_masks[Black]);
                    if d == 8 && push_one & (1 << target) != 0 {
                        true
                    } else if d == 16 && push_one != 0 {
                        return lookup_tables()
                            .lookup_pawn_push(push_one.trailing_zeros() as usize, color)
                            & !(self.color_masks[White] | self.color_masks[Black])
                            & (1 << target)
                            != 0;
                    } else {
                        false
                    }
                }
            }
            Knight => self.knight_attacks(color) & !self.color_masks[color] & (1 << target) != 0,
            Bishop => {
                self.bishop_attacks(color, self.color_masks[White] | self.color_masks[Black])
                    & !self.color_masks[color]
                    & (1 << target)
                    != 0
            }
            Rook => {
                self.rook_attacks(color, self.color_masks[White] | self.color_masks[Black])
                    & !self.color_masks[color]
                    & (1 << target)
                    != 0
            }
            Queen => {
                self.queen_attacks(color, self.color_masks[White] | self.color_masks[Black])
                    & !self.color_masks[color]
                    & (1 << target)
                    != 0
            }
            King => self.king_attacks(color) & !self.color_masks[color] & (1 << target) != 0,
            NoPiece => false,
        }
    }

    pub fn move_from(&self, start: u8, target: u8, promotion: PieceIndex) -> Move {
        let piece = self.piece_at(start as usize);
        debug_assert!(piece != NoPiece);
        let double_pawn_push = piece == Pawn && (target as i8 - start as i8).abs() == 16;
        let capture = self.piece_at(target as usize) != NoPiece;
        let en_passent = piece == Pawn && target == self.enpassent_square() as u8;
        let castling = piece == King && (target as i8 - start as i8) == 2;
        Move::new(
            start,
            target,
            piece,
            promotion,
            capture,
            double_pawn_push,
            en_passent,
            castling,
        )
    }

    pub fn legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::with_capacity(50);
        let color = self.current_player;

        let tables = lookup_tables();
        let king_square = self.piece_masks[(color, King)].trailing_zeros() as usize;

        // King moves
        let kingless_blocking_mask =
            (self.color_masks[color] ^ self.piece_masks[(color, King)]) | self.color_masks[!color];
        let attacked_squares = self.all_attacks(!color, kingless_blocking_mask);
        let mut king_moves =
            self.king_attacks(color) & !(attacked_squares | self.color_masks[color]);
        while king_moves != 0 {
            let target = king_moves.trailing_zeros() as u8;
            let capture = (1 << target) & self.color_masks[!color] != 0;
            moves.push(Move::king_move(king_square as u8, target, capture));
            king_moves ^= 1 << target;
        }

        // Check evasions
        let checkers = tables.lookup_pawn_attack(king_square, color)
            & self.piece_masks[(!color, Pawn)]
            | tables.lookup_knight(king_square) & self.piece_masks[(!color, Knight)]
            | tables.lookup_bishop(
                king_square,
                self.color_masks[White] | self.color_masks[Black],
            ) & self.piece_masks[(!color, Bishop)]
            | tables.lookup_rook(
                king_square,
                self.color_masks[White] | self.color_masks[Black],
            ) & self.piece_masks[(!color, Rook)]
            | tables.lookup_queen(
                king_square,
                self.color_masks[White] | self.color_masks[Black],
            ) & self.piece_masks[(!color, Queen)];

        let num_checkers = checkers.count_ones();
        // - Double Check
        // only king moves are legal in double+ check
        if num_checkers > 1 {
            return moves;
        }

        // mask of square a piece can capture on
        let mut capture_mask = 0xFFFFFFFFFFFFFFFFu64;
        // mask of squares a piece can move to
        let mut push_mask = 0xFFFFFFFFFFFFFFFFu64;
        // - Single Check
        if num_checkers == 1 {
            capture_mask = checkers;

            let checker_square = checkers.trailing_zeros() as usize;
            if self.piece_at(checker_square).is_slider() {
                // if the checking piece is a slider, we can push a piece to block it
                let slider_rays;
                if (king_square % 8) == (checker_square % 8)
                    || (king_square / 8) == (checker_square / 8)
                {
                    // orthogonal slider
                    slider_rays = tables.lookup_rook(king_square, 1 << checker_square);
                    push_mask = tables.lookup_rook(checker_square, 1 << king_square) & slider_rays;
                } else {
                    // diagonal slider
                    slider_rays = tables.lookup_bishop(king_square, 1 << checker_square);
                    push_mask =
                        tables.lookup_bishop(checker_square, 1 << king_square) & slider_rays;
                }
            } else {
                // if the piece is not a slider, we can only capture
                push_mask = 0u64;
            }
        }
        // Pinned pieces
        let mut pinned_pieces = 0u64;

        let orthogonal_pin_rays = tables.lookup_rook(king_square, self.color_masks[!color]);
        let mut pinning_orthogonals = (self.piece_masks[(!color, Rook)]
            | self.piece_masks[(!color, Queen)])
            & orthogonal_pin_rays;
        while pinning_orthogonals != 0 {
            let pinner_square = pinning_orthogonals.trailing_zeros() as usize;
            let pin_ray = (orthogonal_pin_rays
                & tables.lookup_rook(pinner_square, 1 << king_square))
                | (1 << pinner_square)
                | (1 << king_square);

            if (pin_ray & self.color_masks[color]).count_ones() == 2 {
                // there is only the king and one piece on this ray so there is a pin
                // we only need to generate moves for rooks, queens and pawn pushes in this case

                // add any pinned piece to the mask
                pinned_pieces |=
                    pin_ray & (self.color_masks[color] & !self.piece_masks[(color, King)]);

                let pinned_rook_or_queen =
                    pin_ray & (self.piece_masks[(color, Rook)] | self.piece_masks[(color, Queen)]);
                if pinned_rook_or_queen != 0 {
                    let rook_square = pinned_rook_or_queen.trailing_zeros() as u8;
                    let mut rook_moves = pin_ray
                        & (push_mask | capture_mask)
                        & !((1 << king_square) | pinned_rook_or_queen);
                    while rook_moves != 0 {
                        let target = rook_moves.trailing_zeros() as u8;
                        let capture = target as usize == pinner_square;
                        moves.push(Move::new(
                            rook_square,
                            target,
                            self.piece_at(rook_square as usize),
                            NoPiece,
                            capture,
                            false,
                            false,
                            false,
                        ));
                        rook_moves ^= 1 << target;
                    }
                }
                let pinned_pawn = pin_ray & self.piece_masks[(color, Pawn)];
                if pinned_pawn != 0 {
                    let pawn_square = pinned_pawn.trailing_zeros() as u8;
                    let mut pawn_moves = tables.lookup_pawn_push(pawn_square as usize, color)
                        & pin_ray
                        & push_mask
                        & !(self.color_masks[color] | self.color_masks[!color]);
                    pawn_moves |= if pawn_moves != 0
                        && ((color == White
                            && pawn_square / 8 == 1
                            && (self.color_masks[White] | self.color_masks[Black])
                                & 1 << (pawn_square + 16)
                                == 0)
                            || (color == Black
                                && pawn_square / 8 == 6
                                && (self.color_masks[White] | self.color_masks[Black])
                                    & 1 << (pawn_square - 16)
                                    == 0))
                    {
                        tables.lookup_pawn_push(pawn_moves.trailing_zeros() as usize, color)
                    } else {
                        0
                    };
                    while pawn_moves != 0 {
                        let target = pawn_moves.trailing_zeros() as u8;
                        moves.push(Move::new(
                            pawn_square,
                            target,
                            Pawn,
                            NoPiece,
                            false,
                            // double pawn push
                            (target as isize - pawn_square as isize).abs() == 16,
                            false,
                            false,
                        ));
                        pawn_moves ^= 1 << target;
                    }
                }
            }
            pinning_orthogonals ^= 1 << pinner_square;
        }
        let diagonal_pin_rays = tables.lookup_bishop(king_square, self.color_masks[!color]);
        let mut pinning_diagonals = (self.piece_masks[(!color, Bishop)]
            | self.piece_masks[(!color, Queen)])
            & diagonal_pin_rays;
        while pinning_diagonals != 0 {
            let pinner_square = pinning_diagonals.trailing_zeros() as usize;
            let pin_ray = (diagonal_pin_rays
                & tables.lookup_bishop(pinner_square, 1 << king_square))
                | (1 << pinner_square)
                | (1 << king_square);

            if (pin_ray & self.color_masks[color]).count_ones() == 2 {
                // there is only the king and one piece on this ray so there is a pin
                // we only need to generate moves for bishops, queens and pawn captures in this case

                // add any pinned piece to the mask
                pinned_pieces |=
                    pin_ray & (self.color_masks[color] & !self.piece_masks[(color, King)]);

                let pinned_bishop_or_queen = pin_ray
                    & (self.piece_masks[(color, Bishop)] | self.piece_masks[(color, Queen)]);
                if pinned_bishop_or_queen != 0 {
                    let bishop_square = pinned_bishop_or_queen.trailing_zeros() as u8;
                    let mut bishop_moves = pin_ray
                        & (push_mask | capture_mask)
                        & !((1 << king_square) | pinned_bishop_or_queen);
                    while bishop_moves != 0 {
                        let target = bishop_moves.trailing_zeros() as u8;
                        let capture = target as usize == pinner_square;
                        moves.push(Move::new(
                            bishop_square,
                            target,
                            self.piece_at(bishop_square as usize),
                            NoPiece,
                            capture,
                            false,
                            false,
                            false,
                        ));
                        bishop_moves ^= 1 << target;
                    }
                }

                let pinned_pawn = pin_ray & self.piece_masks[(color, Pawn)];
                if pinned_pawn != 0 {
                    let pawn_square = pinned_pawn.trailing_zeros() as u8;
                    let mut pawn_moves = tables.lookup_pawn_attack(pawn_square as usize, color)
                        & pin_ray
                        & capture_mask
                        & (self.color_masks[!color] | self.en_passent_mask);
                    while pawn_moves != 0 {
                        let target = pawn_moves.trailing_zeros() as u8;
                        if target / 8 == !color as u8 * 7 {
                            // pinned pawn capture promotions
                            moves.push(Move::pawn_capture_promotion(pawn_square, target, Knight));
                            moves.push(Move::pawn_capture_promotion(pawn_square, target, Bishop));
                            moves.push(Move::pawn_capture_promotion(pawn_square, target, Rook));
                            moves.push(Move::pawn_capture_promotion(pawn_square, target, Queen));
                        } else {
                            moves.push(Move::new(
                                pawn_square,
                                target,
                                Pawn,
                                NoPiece,
                                true,
                                false,
                                // en passent capture
                                target == self.en_passent_mask.trailing_zeros() as u8,
                                false,
                            ));
                        }
                        pawn_moves ^= 1 << target;
                    }
                }
            }
            pinning_diagonals ^= 1 << pinner_square;
        }

        // Other moves
        // Castling if not in check
        if num_checkers == 0 {
            let king = self.piece_masks[(color, King)];
            if self.castling_rights[(color, Kingside)]
                && (self.color_masks[White] | self.color_masks[Black]) & ((king << 1) | (king << 2))
                    == 0
                && attacked_squares & ((king << 1) | (king << 2)) == 0
            {
                // generate castling kingside if rights remain, the way is clear and the squares aren't attacked
                let start = king.trailing_zeros() as u8;
                moves.push(Move::king_castle(start, start + 2));
            }
            if self.castling_rights[(color, Queenside)]
                && (self.color_masks[White] | self.color_masks[Black])
                    & ((king >> 1) | (king >> 2) | (king >> 3))
                    == 0
                && attacked_squares & ((king >> 1) | (king >> 2)) == 0
            {
                // generate castling queenside if rights remain, the way is clear and the squares aren't attacked
                let start = king.trailing_zeros() as u8;
                moves.push(Move::king_castle(start, start - 2));
            }
        }
        // Pawn moves
        let mut pawns = self.piece_masks[(color, Pawn)] & !pinned_pieces;
        if color == White {
            // white pawn moves
            while pawns != 0 {
                let pawn_square = pawns.trailing_zeros() as u8;
                let pawn = 1 << pawn_square;

                // single pawn pushes
                let pawn_push_one =
                    (pawn << 8) & push_mask & !(self.color_masks[White] | self.color_masks[Black]);
                if pawn_push_one != 0 {
                    let target = pawn_push_one.trailing_zeros() as u8;
                    // promotions
                    if target / 8 == 7 {
                        moves.push(Move::pawn_push_promotion(pawn_square, target, Knight));
                        moves.push(Move::pawn_push_promotion(pawn_square, target, Bishop));
                        moves.push(Move::pawn_push_promotion(pawn_square, target, Rook));
                        moves.push(Move::pawn_push_promotion(pawn_square, target, Queen));
                    } else {
                        // no promotion
                        moves.push(Move::pawn_push(pawn_square, target));
                    }
                }
                // double pawn pushes
                let pawn_push_two = ((((pawn & SECOND_RANK) << 8)
                    & !(self.color_masks[White] | self.color_masks[Black]))
                    << 8)
                    & !(self.color_masks[White] | self.color_masks[Black])
                    & push_mask;

                if pawn_push_two != 0 {
                    moves.push(Move::pawn_double_push(
                        pawn_square,
                        pawn_push_two.trailing_zeros() as u8,
                    ));
                }
                // pawn captures
                let mut pawn_captures = (((pawn & NOT_A_FILE) << 7) | ((pawn & NOT_H_FILE) << 9))
                    & (capture_mask | self.en_passent_mask)
                    & (self.color_masks[!color] | self.en_passent_mask);
                while pawn_captures != 0 {
                    let target = pawn_captures.trailing_zeros() as u8;
                    if target / 8 == 7 {
                        // promotions
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Knight));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Bishop));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Rook));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Queen));
                    } else if 1 << target == self.en_passent_mask {
                        // en passent capture
                        if self.piece_masks[(color, King)].trailing_zeros() / 8 == 4 {
                            let mut en_passent_pinned = false;
                            let blocking_mask = (self.color_masks[White] | self.color_masks[Black])
                                & !((1 << pawn_square) | (self.en_passent_mask >> 8));
                            let mut attacking_rooks_or_queens = (self.piece_masks[(!color, Rook)]
                                | self.piece_masks[(!color, Queen)])
                                & FIFTH_RANK;
                            while attacking_rooks_or_queens != 0 {
                                let rook_square =
                                    attacking_rooks_or_queens.trailing_zeros() as usize;
                                if tables.lookup_rook(rook_square, blocking_mask)
                                    & self.piece_masks[(color, King)]
                                    != 0
                                {
                                    en_passent_pinned = true;
                                    break;
                                }
                                attacking_rooks_or_queens ^= 1 << rook_square;
                            }
                            let mut attacking_queens =
                                self.piece_masks[(!color, Queen)] & FOURTH_RANK;
                            while attacking_queens != 0 {
                                let queen_square = attacking_queens.trailing_zeros() as usize;
                                if tables.lookup_queen(queen_square, blocking_mask)
                                    & self.piece_masks[(color, King)]
                                    != 0
                                {
                                    en_passent_pinned = true;
                                    break;
                                }
                                attacking_queens ^= 1 << queen_square;
                            }
                            if !en_passent_pinned {
                                moves.push(Move::pawn_enpassent_capture(pawn_square, target));
                            }
                        } else {
                            moves.push(Move::pawn_enpassent_capture(pawn_square, target));
                        }
                    } else {
                        // normal captures
                        moves.push(Move::pawn_capture(pawn_square, target));
                    }
                    pawn_captures ^= 1 << target;
                }
                pawns ^= 1 << pawn_square;
            }
        } else {
            // black pawn moves
            while pawns != 0 {
                let pawn_square = pawns.trailing_zeros() as u8;
                let pawn = 1 << pawn_square;

                // single pawn pushes
                let pawn_push_one =
                    (pawn >> 8) & push_mask & !(self.color_masks[White] | self.color_masks[Black]);
                if pawn_push_one != 0 {
                    let target = pawn_push_one.trailing_zeros() as u8;
                    // promotions
                    if target / 8 == 0 {
                        moves.push(Move::pawn_push_promotion(pawn_square, target, Knight));
                        moves.push(Move::pawn_push_promotion(pawn_square, target, Bishop));
                        moves.push(Move::pawn_push_promotion(pawn_square, target, Rook));
                        moves.push(Move::pawn_push_promotion(pawn_square, target, Queen));
                    } else {
                        // no promotion
                        moves.push(Move::pawn_push(pawn_square, target));
                    }
                }
                // double pawn pushes
                let pawn_push_two = ((((pawn & SEVENTH_RANK) >> 8)
                    & !(self.color_masks[White] | self.color_masks[Black]))
                    >> 8)
                    & !(self.color_masks[White] | self.color_masks[Black])
                    & push_mask;
                if pawn_push_two != 0 {
                    moves.push(Move::pawn_double_push(
                        pawn_square,
                        pawn_push_two.trailing_zeros() as u8,
                    ));
                }
                // pawn captures
                let mut pawn_captures = (((pawn & NOT_A_FILE) >> 9) | ((pawn & NOT_H_FILE) >> 7))
                    & (capture_mask | self.en_passent_mask)
                    & (self.color_masks[!color] | self.en_passent_mask);
                while pawn_captures != 0 {
                    let target = pawn_captures.trailing_zeros() as u8;
                    if target / 8 == 0 {
                        // promotions
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Knight));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Bishop));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Rook));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Queen));
                    } else if 1 << target == self.en_passent_mask {
                        // en passent capture
                        if self.piece_masks[(color, King)].trailing_zeros() / 8 == 3 {
                            let mut en_passent_pinned = false;
                            let blocking_mask = (self.color_masks[White] | self.color_masks[Black])
                                & !((1 << pawn_square) | (self.en_passent_mask << 8));
                            let mut attacking_rooks_or_queens = (self.piece_masks[(!color, Rook)]
                                | self.piece_masks[(!color, Queen)])
                                & FOURTH_RANK;
                            while attacking_rooks_or_queens != 0 {
                                let rook_square =
                                    attacking_rooks_or_queens.trailing_zeros() as usize;
                                if tables.lookup_rook(rook_square, blocking_mask)
                                    & self.piece_masks[(color, King)]
                                    != 0
                                {
                                    en_passent_pinned = true;
                                    break;
                                }
                                attacking_rooks_or_queens ^= 1 << rook_square;
                            }
                            if !en_passent_pinned {
                                moves.push(Move::pawn_enpassent_capture(pawn_square, target));
                            }
                        } else {
                            moves.push(Move::pawn_enpassent_capture(pawn_square, target));
                        }
                    } else {
                        // normal captures
                        moves.push(Move::pawn_capture(pawn_square, target));
                    }
                    pawn_captures ^= 1 << target;
                }
                pawns ^= 1 << pawn_square;
            }
        }

        // Knight moves
        let mut knights = self.piece_masks[(color, Knight)] & !pinned_pieces;
        while knights != 0 {
            let knight_square = knights.trailing_zeros() as usize;
            let mut attacks = tables.lookup_knight(knight_square)
                & !self.color_masks[color]
                & (push_mask | capture_mask);
            while attacks != 0 {
                let target = attacks.trailing_zeros() as u8;
                let capture = self.color_masks[!color] & (1 << target) != 0;
                moves.push(Move::knight_move(knight_square as u8, target, capture));
                attacks ^= 1 << target;
            }
            knights ^= 1 << knight_square;
        }

        // Bishop moves
        let mut bishops = self.piece_masks[(color, Bishop)] & !pinned_pieces;
        while bishops != 0 {
            let bishop_square = bishops.trailing_zeros() as usize;
            let mut attacks = tables.lookup_bishop(
                bishop_square,
                self.color_masks[White] | self.color_masks[Black],
            ) & !self.color_masks[color]
                & (push_mask | capture_mask);
            while attacks != 0 {
                let target = attacks.trailing_zeros() as u8;
                let capture = self.color_masks[!color] & (1 << target) != 0;
                moves.push(Move::bishop_move(bishop_square as u8, target, capture));
                attacks ^= 1 << target;
            }
            bishops ^= 1 << bishop_square;
        }

        // Rook moves
        let mut rooks = self.piece_masks[(color, Rook)] & !pinned_pieces;
        while rooks != 0 {
            let rook_square = rooks.trailing_zeros() as usize;
            let mut attacks = tables.lookup_rook(
                rook_square,
                self.color_masks[White] | self.color_masks[Black],
            ) & !self.color_masks[color]
                & (push_mask | capture_mask);
            while attacks != 0 {
                let target = attacks.trailing_zeros() as u8;
                let capture = self.color_masks[!color] & (1 << target) != 0;
                moves.push(Move::rook_move(rook_square as u8, target, capture));
                attacks ^= 1 << target;
            }
            rooks ^= 1 << rook_square;
        }

        // queen moves
        let mut queens = self.piece_masks[(color, Queen)] & !pinned_pieces;
        while queens != 0 {
            let queen_square = queens.trailing_zeros() as usize;
            let mut attacks = tables.lookup_queen(
                queen_square,
                self.color_masks[White] | self.color_masks[Black],
            ) & !self.color_masks[color]
                & (push_mask | capture_mask);
            while attacks != 0 {
                let target = attacks.trailing_zeros() as u8;
                let capture = self.color_masks[!color] & (1 << target) != 0;
                moves.push(Move::queen_move(queen_square as u8, target, capture));
                attacks ^= 1 << target;
            }
            queens ^= 1 << queen_square;
        }

        moves
    }

    pub fn make_move(&mut self, move_: Move) {
        let color = self.current_player;
        let start = move_.start() as usize;
        let target = move_.target() as usize;
        let piece = move_.piece();

        let captured = if move_.en_passent() {
            Pawn
        } else {
            self.piece_at(target)
        };

        // Update unmove history
        self.unmove_history.push(UnMove::new(
            start as u8,
            target as u8,
            move_.promotion() != NoPiece,
            captured,
            move_.en_passent(),
            self.en_passent_mask,
            move_.castling(),
            self.castling_rights,
            self.halfmove_clock,
        ));

        // add the last position into the history
        self.position_history.push(self.hash);

        // increment the halfmove clock for 50-move rule
        self.halfmove_clock += 1;

        // Castling
        if move_.castling() {
            let dx = target as isize - start as isize;
            let (rook_start, rook_target) = if dx == 2 {
                // Kingside
                (target + 1, target - 1)
            } else {
                // Queenside
                (target - 2, target + 1)
            };

            // update king position and hash
            self.hash ^= zobrist_piece(King, color, start) ^ zobrist_piece(King, color, target);
            self.piece_masks[(color, King)] ^= (1 << target) | (1 << start);
            self.piece_list[start] = NoPiece;
            self.piece_list[target] = King;
            // update rook position and hash
            self.hash ^=
                zobrist_piece(Rook, color, rook_start) ^ zobrist_piece(Rook, color, rook_target);
            self.piece_masks[(color, Rook)] ^= (1 << rook_target) | (1 << rook_start);
            self.piece_list[rook_start] = NoPiece;
            self.piece_list[rook_target] = Rook;
            // update color masks
            self.color_masks[color] ^=
                (1 << start) | (1 << target) | (1 << rook_start) | (1 << rook_target);
            // update castling rights
            self.hash ^= zobrist_castling(self.castling_rights);
            self.castling_rights[color] = [false, false];
            self.hash ^= zobrist_castling(self.castling_rights);
        }

        // Remove captured piece (en passent, rule 50)
        if captured != NoPiece {
            let cap_square = if move_.en_passent() {
                if color == White {
                    target - 8
                } else {
                    target + 8
                }
            } else {
                target
            };
            // remove piece from target square
            self.hash ^= zobrist_piece(captured, !color, cap_square);
            self.piece_masks[(!color, captured)] ^= 1 << cap_square;
            self.piece_list[cap_square] = NoPiece;
            self.color_masks[!color] ^= 1 << cap_square;

            // reset halfmove clock
            self.halfmove_clock = 0;
        }

        // reset en passent square
        if self.en_passent_mask != 0 {
            self.hash ^= zobrist_enpassent(self.en_passent_mask);
            self.en_passent_mask = 0;
        }

        // update castling rights
        if piece == King {
            self.hash ^= zobrist_castling(self.castling_rights);
            self.castling_rights[color] = [false, false];
            self.hash ^= zobrist_castling(self.castling_rights);
        } else if piece == Rook {
            if self.castling_rights[(color, Kingside)] && start == 7 + 56 * color as usize {
                // kingside rook has made first move
                self.hash ^= zobrist_castling(self.castling_rights);
                self.castling_rights[(color, Kingside)] = false;
                self.hash ^= zobrist_castling(self.castling_rights);
            } else if self.castling_rights[(color, Queenside)] && start == 56 * color as usize {
                // queenside rook has made first move
                self.hash ^= zobrist_castling(self.castling_rights);
                self.castling_rights[(color, Queenside)] = false;
                self.hash ^= zobrist_castling(self.castling_rights);
            }
        }
        if captured == Rook {
            if self.castling_rights[(!color, Kingside)] && target == 7 + 56 * !color as usize {
                // kingside rook has been captured
                self.hash ^= zobrist_castling(self.castling_rights);
                self.castling_rights[(!color, Kingside)] = false;
                self.hash ^= zobrist_castling(self.castling_rights);
            } else if self.castling_rights[(!color, Queenside)] && target == 56 * !color as usize {
                // queenside rook has been captured
                self.hash ^= zobrist_castling(self.castling_rights);
                self.castling_rights[(!color, Queenside)] = false;
                self.hash ^= zobrist_castling(self.castling_rights);
            }
        }

        // move the piece
        if !move_.castling() {
            self.hash ^= zobrist_piece(piece, color, start) ^ zobrist_piece(piece, color, target);
            self.piece_masks[(color, piece)] ^= (1 << start) | (1 << target);
            self.piece_list[start] = NoPiece;
            self.piece_list[target] = piece;
            self.color_masks[color] ^= (1 << start) | (1 << target);
        }

        // pawn special cases
        if piece == Pawn {
            // en passent square
            if move_.double_pawn_push() {
                let ep_square = if color == White {
                    target - 8
                } else {
                    target + 8
                };
                // only set the ep mask if the pawn can be taken
                self.en_passent_mask = (1 << ep_square) & self.pawn_attacks(!color);
                if self.en_passent_mask != 0 {
                    self.hash ^= zobrist_enpassent(self.en_passent_mask);
                }
            }
            // promotion
            if move_.promotion() != NoPiece {
                self.hash ^= zobrist_piece(Pawn, color, target)
                    ^ zobrist_piece(move_.promotion(), color, target);
                self.piece_masks[(color, Pawn)] ^= 1 << target;
                self.piece_list[target] = move_.promotion();
                self.piece_masks[(color, move_.promotion())] |= 1 << target;
            }
            // rule 50
            self.halfmove_clock = 0;
        }

        // swap players
        self.hash ^= zobrist_player();
        self.current_player = !self.current_player;

        debug_assert!(self.hash == self.zobrist_hash());
    }

    pub fn unmake_move(&mut self) {
        self.current_player = !self.current_player;

        let unmove = self.unmove_history.pop().unwrap();
        let start = unmove.start as usize;
        let target = unmove.target as usize;

        let mut piece = self.piece_list[target];
        if unmove.promotion {
            self.piece_masks[(self.current_player, piece)] ^= 1 << target;

            self.piece_masks[(self.current_player, Pawn)] ^= 1 << target;
            self.piece_list[target] = Pawn;
            piece = Pawn;
        }

        if unmove.castling {
            if target % 8 == 2 {
                // queenside
                self.piece_masks[(self.current_player, King)] ^= (1 << start) | (1 << target);
                self.piece_list[start] = King;
                self.piece_list[target] = NoPiece;

                let rook_start = target - 2;
                let rook_target = target + 1;

                self.piece_masks[(self.current_player, Rook)] ^=
                    (1 << rook_start) | (1 << rook_target);
                self.piece_list[rook_start] = Rook;
                self.piece_list[rook_target] = NoPiece;

                self.color_masks[self.current_player] ^=
                    (1 << start) | (1 << target) | (1 << rook_start) | (1 << rook_target);
            } else {
                // kingside
                self.piece_masks[(self.current_player, King)] ^= (1 << start) | (1 << target);
                self.piece_list[start] = King;
                self.piece_list[target] = NoPiece;

                let rook_start = target + 1;
                let rook_target = target - 1;

                self.piece_masks[(self.current_player, Rook)] ^=
                    (1 << rook_start) | (1 << rook_target);
                self.piece_list[rook_start] = Rook;
                self.piece_list[rook_target] = NoPiece;

                self.color_masks[self.current_player] ^=
                    (1 << start) | (1 << target) | (1 << rook_start) | (1 << rook_target);
            }
        } else {
            // move piece back to start
            self.piece_masks[(self.current_player, piece)] ^= (1 << start) | (1 << target);
            self.piece_list[target] = NoPiece;
            self.piece_list[start] = piece;
            self.color_masks[self.current_player] ^= (1 << start) | (1 << target);

            if unmove.capture != NoPiece {
                let mut cap_square = target;
                if unmove.en_passent {
                    cap_square = match self.current_player {
                        White => target - 8,
                        Black => target + 8,
                    };
                }
                // replace captured piece
                self.piece_masks[(!self.current_player, unmove.capture)] ^= 1 << cap_square;
                self.piece_list[cap_square] = unmove.capture;
                self.color_masks[!self.current_player] ^= 1 << cap_square;
            }
        }

        // restore board state
        self.castling_rights = unmove.castling_rights;
        self.en_passent_mask = unmove.en_passent_mask;
        self.hash = self.position_history.pop().unwrap();
        self.halfmove_clock = unmove.halfmove_clock;

        debug_assert!(self.hash == self.zobrist_hash());
    }

    pub fn zobrist_hash(&self) -> u64 {
        let mut hash = 0u64;
        // pieces
        for piece in [Pawn, Knight, Bishop, Rook, Queen, King] {
            for color in [White, Black] {
                let mut pieces = self.piece_masks[(color, piece)];
                while pieces != 0 {
                    let square = pieces.trailing_zeros() as usize;
                    hash ^= zobrist_piece(piece, color, square);
                    pieces ^= 1 << square;
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
        if self.en_passent_mask != 0 {
            hash ^= zobrist_enpassent(self.en_passent_mask);
        }

        hash
    }

    pub fn perft(&mut self, depth: usize) -> usize {
        if depth == 0 {
            return 1;
        }

        let moves = self.legal_moves();
        let mut nodes = 0;

        for move_ in moves {
            self.make_move(move_);
            nodes += self.perft(depth - 1);
            self.unmake_move();
        }
        nodes
    }

    pub fn divide(&mut self, depth: usize) {
        if depth == 0 {
            return;
        }
        let moves = self.legal_moves();
        let mut move_count = 0;
        let mut node_count = 0;
        for move_ in moves {
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
