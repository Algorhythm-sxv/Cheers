use crate::{
    bitboard::BitBoard,
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

mod eval_params;
mod evaluate;
mod search;

pub use self::search::{NODE_COUNT, NPS_COUNT, RUN_SEARCH};

pub use self::eval_params::*;
pub use self::evaluate::{NoTracing, Tracing, TracingType};

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
    position_history: Vec<u64>,
    unmove_history: Vec<UnMove>,
    transposition_table: TranspositionTable,
    eval_params: EvalParams,
}

impl ChessGame {
    pub fn new(tt: TranspositionTable) -> Self {
        let mut boards = Self {
            color_masks: ColorMasks::default(),
            combined: BitBoard::empty(),
            piece_masks: PieceMasks::default(),
            current_player: ColorIndex::default(),
            castling_rights: CastlingRights::default(),
            en_passent_mask: BitBoard::empty(),
            halfmove_clock: 0,
            hash: 0,
            position_history: Vec::new(),
            unmove_history: Vec::new(),
            transposition_table: tt,
            eval_params: EVAL_PARAMS,
        };
        boards.combined = boards.color_masks[White] | boards.color_masks[Black];
        boards
            .set_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        boards
    }

    pub fn reset(&mut self) {
        let params = self.eval_params;
        *self = Self {
            color_masks: ColorMasks::default(),
            combined: BitBoard::empty(),
            piece_masks: PieceMasks::default(),
            current_player: ColorIndex::default(),
            castling_rights: CastlingRights::default(),
            en_passent_mask: BitBoard::empty(),
            halfmove_clock: 0,
            hash: 0,
            position_history: Vec::new(),
            unmove_history: Vec::new(),
            transposition_table: self.transposition_table.clone(),
            eval_params: params,
        };
        self.combined = self.color_masks[White] | self.color_masks[Black];
        self.set_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap()
    }

    pub fn set_from_fen(
        &mut self,
        fen: impl Into<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.piece_masks = PieceMasks([[BitBoard::empty(); 6]; 2]);
        self.color_masks = ColorMasks([BitBoard::empty(); 2]);

        let fen = fen.into();
        let mut lines = fen.split(&['/', ' '][..]);

        for (i, line) in lines.clone().take(8).enumerate() {
            let mut index = 56 - i * 8;
            for chr in line.chars() {
                match chr {
                    'n' => {
                        self.piece_masks[(Black, Knight)] |= BitBoard(1 << index);
                        self.color_masks[Black] |= BitBoard(1 << index);
                    }
                    'N' => {
                        self.piece_masks[(White, Knight)] |= BitBoard(1 << index);
                        self.color_masks[White] |= BitBoard(1 << index);
                    }
                    'b' => {
                        self.piece_masks[(Black, Bishop)] |= BitBoard(1 << index);
                        self.color_masks[Black] |= BitBoard(1 << index);
                    }
                    'B' => {
                        self.piece_masks[(White, Bishop)] |= BitBoard(1 << index);
                        self.color_masks[White] |= BitBoard(1 << index);
                    }
                    'r' => {
                        self.piece_masks[(Black, Rook)] |= BitBoard(1 << index);
                        self.color_masks[Black] |= BitBoard(1 << index);
                    }
                    'R' => {
                        self.piece_masks[(White, Rook)] |= BitBoard(1 << index);
                        self.color_masks[White] |= BitBoard(1 << index);
                    }
                    'q' => {
                        self.piece_masks[(Black, Queen)] |= BitBoard(1 << index);
                        self.color_masks[Black] |= BitBoard(1 << index);
                    }
                    'Q' => {
                        self.piece_masks[(White, Queen)] |= BitBoard(1 << index);
                        self.color_masks[White] |= BitBoard(1 << index);
                    }
                    'k' => {
                        self.piece_masks[(Black, King)] |= BitBoard(1 << index);
                        self.color_masks[Black] |= BitBoard(1 << index);
                    }
                    'K' => {
                        self.piece_masks[(White, King)] |= BitBoard(1 << index);
                        self.color_masks[White] |= BitBoard(1 << index);
                    }
                    'p' => {
                        self.piece_masks[(Black, Pawn)] |= BitBoard(1 << index);
                        self.color_masks[Black] |= BitBoard(1 << index);
                    }
                    'P' => {
                        self.piece_masks[(White, Pawn)] |= BitBoard(1 << index);
                        self.color_masks[White] |= BitBoard(1 << index);
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
            "-" => self.en_passent_mask = BitBoard::empty(),
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
                self.en_passent_mask = BitBoard(1 << square);
            }
        }

        self.halfmove_clock = lines
            .next()
            .ok_or_else(|| String::from("No halfmove clock!"))?
            .parse::<u8>()?;

        self.combined = self.color_masks[White] | self.color_masks[Black];
        let hash = self.zobrist_hash();
        self.hash = hash;

        Ok(())
    }

    pub fn fen(&self) -> String {
        let mut fen = String::new();
        // get pieces by square
        for rank in (0..8).rev() {
            let mut empty_counter = 0;
            for file in 0..8 {
                let square = 8 * rank + file;
                let piece = self.piece_at(square);

                match piece {
                    NoPiece => empty_counter += 1,
                    piece => {
                        if empty_counter != 0 {
                            fen.push(char::from_digit(empty_counter, 10).unwrap());
                            empty_counter = 0;
                        }
                        let mut letter = match piece {
                            Pawn => 'p',
                            Knight => 'n',
                            Bishop => 'b',
                            Rook => 'r',
                            Queen => 'q',
                            King => 'k',
                            NoPiece => unreachable!(),
                        };
                        if self.color_at(square) == White {
                            letter = letter.to_ascii_uppercase();
                        }
                        fen.push(letter);
                    }
                }
            }
            if empty_counter != 0 {
                fen.push(char::from_digit(empty_counter, 10).unwrap());
            }
            fen.push('/');
        }
        // remove trailing '/'
        fen.pop();
        fen.push(' ');

        // metadata
        // side to move
        fen.push(match self.current_player() {
            White => 'w',
            Black => 'b',
        });
        fen.push(' ');

        // castling rights
        if self.castling_rights[(White, Kingside)] {
            fen.push('K')
        }
        if self.castling_rights[(White, Queenside)] {
            fen.push('Q')
        }
        if self.castling_rights[(Black, Kingside)] {
            fen.push('k')
        }
        if self.castling_rights[(Black, Kingside)] {
            fen.push('q')
        }
        if self.castling_rights == CastlingRights([[false, false], [false, false]]) {
            fen.push('-')
        }
        fen.push(' ');

        // en passent square
        match self.enpassent_square() {
            64 => fen.push('-'),
            square => fen.push_str(&coord(square as u8)),
        }
        fen.push(' ');

        // halfmove clock
        fen.push_str(&self.halfmove_clock.to_string());
        fen.push(' ');

        // fullmove number
        fen.push_str(&(self.position_history.len() / 2).to_string());

        fen
    }

    pub fn set_eval_params(&mut self, params: EvalParams) {
        self.eval_params = params;
    }

    pub fn eval_params(&self) -> EvalParams {
        self.eval_params
    }

    pub fn piece_masks(&self) -> PieceMasks {
        self.piece_masks
    }

    pub fn enpassent_square(&self) -> usize {
        self.en_passent_mask.lsb_index() as usize
    }

    pub fn current_player(&self) -> ColorIndex {
        self.current_player
    }
    pub fn combined(&self) -> BitBoard {
        self.combined
    }

    #[inline]
    pub fn piece_at(&self, square: usize) -> PieceIndex {
        let test = BitBoard(1 << square as u64);
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

    pub fn color_at(&self, square: usize) -> ColorIndex {
        if (self.color_masks[White] & BitBoard(1 << square)).is_not_empty() {
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

    pub fn pawn_front_spans(&self, color: ColorIndex) -> BitBoard {
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
            result |= lookup_knight(i.into());
        }
        result
    }

    fn bishop_attacks(&self, color: ColorIndex, blocking_mask: BitBoard) -> BitBoard {
        let bishops = self.piece_masks[(color, Bishop)];

        let mut result = BitBoard::empty();
        for i in bishops {
            result |= lookup_bishop(i.into(), blocking_mask);
        }
        result
    }

    fn rook_attacks(&self, color: ColorIndex, blocking_mask: BitBoard) -> BitBoard {
        let rooks = self.piece_masks[(color, Rook)];

        let mut result = BitBoard::empty();
        for i in rooks {
            result |= lookup_rook(i.into(), blocking_mask);
        }
        result
    }

    fn queen_attacks(&self, color: ColorIndex, blocking_mask: BitBoard) -> BitBoard {
        let queens = self.piece_masks[(color, Queen)];

        let mut result = BitBoard::empty();
        for i in queens {
            result |= lookup_queen(i.into(), blocking_mask);
        }
        result
    }

    fn king_attacks(&self, color: ColorIndex) -> BitBoard {
        let king = self.piece_masks[(color, King)];
        lookup_king(king.lsb_index() as usize)
    }

    fn discovered_attacks(&self, square: usize, color: ColorIndex) -> BitBoard {
        let rook_attacks = lookup_rook(square, self.combined);
        let bishop_attacks = lookup_bishop(square, self.combined);

        let rooks = self.piece_masks[(!color, Rook)] & rook_attacks.inverse();
        let bishops = self.piece_masks[(!color, Bishop)] & bishop_attacks.inverse();

        return (rooks & lookup_rook(square, self.combined & rook_attacks.inverse()))
            | (bishops & lookup_bishop(square, self.combined & bishop_attacks.inverse()));
    }

    fn all_attacks(&self, color: ColorIndex, blocking_mask: BitBoard) -> BitBoard {
        self.pawn_attacks(color)
            | self.knight_attacks(color)
            | self.king_attacks(color)
            | self.bishop_attacks(color, blocking_mask)
            | self.rook_attacks(color, blocking_mask)
            | self.queen_attacks(color, blocking_mask)
    }

    pub fn in_check(&self, color: ColorIndex) -> bool {
        (self.all_attacks(!color, self.combined) & self.piece_masks[(color, King)]).is_not_empty()
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
                    (self.pawn_attacks(color)
                        & (self.color_masks[!color] | self.en_passent_mask)
                        & BitBoard(1 << target))
                    .is_not_empty()
                } else {
                    // pushes
                    let push_one =
                        lookup_pawn_push(start as usize, color) & (self.combined).inverse();
                    if d == 8 && (push_one & BitBoard(1 << target)).is_not_empty() {
                        true
                    } else if d == 16 && push_one.is_not_empty() {
                        (lookup_pawn_push(push_one.lsb_index() as usize, color)
                            & (self.combined).inverse()
                            & BitBoard(1 << target))
                        .is_not_empty()
                    } else {
                        false
                    }
                }
            }
            Knight => (self.knight_attacks(color)
                & self.color_masks[color].inverse()
                & BitBoard(1 << target))
            .is_not_empty(),
            Bishop => (self.bishop_attacks(color, self.combined)
                & self.color_masks[color].inverse()
                & BitBoard(1 << target))
            .is_not_empty(),
            Rook => (self.rook_attacks(color, self.combined)
                & self.color_masks[color].inverse()
                & BitBoard(1 << target))
            .is_not_empty(),
            Queen => (self.queen_attacks(color, self.combined)
                & self.color_masks[color].inverse()
                & BitBoard(1 << target))
            .is_not_empty(),
            King => (self.king_attacks(color)
                & self.color_masks[color].inverse()
                & BitBoard(1 << target))
            .is_not_empty(),
            NoPiece => false,
        }
    }

    pub fn legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::with_capacity(64);
        let color = self.current_player;

        let king_square = self.piece_masks[(color, King)].lsb_index() as usize;

        // King moves
        let kingless_blocking_mask =
            (self.color_masks[color] ^ self.piece_masks[(color, King)]) | self.color_masks[!color];
        let attacked_squares = self.all_attacks(!color, kingless_blocking_mask);
        let king_moves =
            self.king_attacks(color) & (attacked_squares | self.color_masks[color]).inverse();
        for target in king_moves {
            let capture = (BitBoard(1 << target) & self.color_masks[!color]).is_not_empty();
            moves.push(Move::king_move(king_square as u8, target, capture));
        }

        // Check evasions
        let checkers = (lookup_pawn_attack(king_square, color) & self.piece_masks[(!color, Pawn)])
            | (lookup_knight(king_square) & self.piece_masks[(!color, Knight)])
            | (lookup_bishop(king_square, self.combined)
                & (self.piece_masks[(!color, Bishop)] | self.piece_masks[(!color, Queen)]))
            | (lookup_rook(king_square, self.combined)
                & (self.piece_masks[(!color, Rook)] | self.piece_masks[(!color, Queen)]));

        let num_checkers = checkers.count_ones();
        // - Double Check
        // only king moves are legal in double+ check
        if num_checkers > 1 {
            return moves;
        }

        // mask of square a piece can capture on
        let mut capture_mask = BitBoard(0xFFFFFFFFFFFFFFFFu64);
        // mask of squares a piece can move to
        let mut push_mask = BitBoard(0xFFFFFFFFFFFFFFFFu64);
        // - Single Check
        if num_checkers == 1 {
            capture_mask = checkers;

            let checker_square = checkers.lsb_index() as usize;
            if self.piece_at(checker_square).is_slider() {
                // if the checking piece is a slider, we can push a piece to block it
                let slider_rays;
                if (king_square % 8) == (checker_square % 8)
                    || (king_square / 8) == (checker_square / 8)
                {
                    // orthogonal slider
                    slider_rays = lookup_rook(king_square, BitBoard(1 << checker_square));
                    push_mask =
                        lookup_rook(checker_square, BitBoard(1 << king_square)) & slider_rays;
                } else {
                    // diagonal slider
                    slider_rays = lookup_bishop(king_square, BitBoard(1 << checker_square));
                    push_mask =
                        lookup_bishop(checker_square, BitBoard(1 << king_square)) & slider_rays;
                }
            } else {
                // if the piece is not a slider, we can only capture
                push_mask = BitBoard::empty();
            }
        }
        // Pinned pieces
        let mut pinned_pieces = BitBoard::empty();

        let orthogonal_pin_rays = lookup_rook(king_square, self.color_masks[!color]);
        let pinning_orthogonals = (self.piece_masks[(!color, Rook)]
            | self.piece_masks[(!color, Queen)])
            & orthogonal_pin_rays;
        for pinner_square in pinning_orthogonals {
            let pin_ray = lookup_between(king_square as u8, pinner_square);

            if (pin_ray & self.color_masks[color]).count_ones() == 1 {
                // there is only one piece on this ray so there is a pin
                // we only need to generate moves for rooks, queens and pawn pushes in this case

                // add any pinned piece to the mask
                pinned_pieces |= pin_ray & self.color_masks[color];

                let pinned_rook_or_queen =
                    pin_ray & (self.piece_masks[(color, Rook)] | self.piece_masks[(color, Queen)]);
                if pinned_rook_or_queen.is_not_empty() {
                    let rook_square = pinned_rook_or_queen.lsb_index() as u8;
                    let rook_moves = (pin_ray | BitBoard(1 << pinner_square))
                        & (push_mask | capture_mask)
                        & pinned_rook_or_queen.inverse();
                    for target in rook_moves {
                        let capture = target == pinner_square;
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
                    }
                }
                let pinned_pawn = pin_ray & self.piece_masks[(color, Pawn)];
                if pinned_pawn.is_not_empty() {
                    let pawn_square = pinned_pawn.lsb_index() as u8;
                    let mut pawn_moves = lookup_pawn_push(pawn_square as usize, color)
                        & pin_ray
                        & push_mask
                        & (self.combined).inverse();
                    if pawn_moves.is_not_empty()
                        && ((color == White
                            && pawn_square / 8 == 1
                            && ((self.combined) & BitBoard(1 << (pawn_square + 16))).is_empty())
                            || (color == Black
                                && pawn_square / 8 == 6
                                && ((self.combined) & BitBoard(1 << (pawn_square - 16)))
                                    .is_empty()))
                    {
                        pawn_moves |= lookup_pawn_push(pawn_moves.lsb_index() as usize, color)
                    }
                    for target in pawn_moves {
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
                    }
                }
            }
        }
        let diagonal_pin_rays = lookup_bishop(king_square, self.color_masks[!color]);
        let pinning_diagonals = (self.piece_masks[(!color, Bishop)]
            | self.piece_masks[(!color, Queen)])
            & diagonal_pin_rays;
        for pinner_square in pinning_diagonals {
            let pin_ray = lookup_between(king_square as u8, pinner_square);

            if (pin_ray & self.color_masks[color]).count_ones() == 1 {
                // there is only the king and one piece on this ray so there is a pin
                // we only need to generate moves for bishops, queens and pawn captures in this case

                // add any pinned piece to the mask
                pinned_pieces |= pin_ray & self.color_masks[color];

                let pinned_bishop_or_queen = pin_ray
                    & (self.piece_masks[(color, Bishop)] | self.piece_masks[(color, Queen)]);
                if pinned_bishop_or_queen.is_not_empty() {
                    let bishop_square = pinned_bishop_or_queen.lsb_index() as u8;
                    let bishop_moves = (pin_ray | BitBoard(1 << pinner_square))
                        & (push_mask | capture_mask)
                        & pinned_bishop_or_queen.inverse();
                    for target in bishop_moves {
                        let capture = target == pinner_square;
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
                    }
                }

                let pinned_pawn = pin_ray & self.piece_masks[(color, Pawn)];
                if pinned_pawn.is_not_empty() {
                    let pawn_square = pinned_pawn.lsb_index() as u8;
                    let pawn_moves = lookup_pawn_attack(pawn_square as usize, color)
                        & BitBoard(1 << pinner_square)
                        & capture_mask
                        & (self.color_masks[!color] | self.en_passent_mask);
                    for target in pawn_moves {
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
                                target == self.en_passent_mask.lsb_index() as u8,
                                false,
                            ));
                        }
                    }
                }
            }
        }

        // Other moves
        // Castling if not in check
        if num_checkers == 0 {
            let king = self.piece_masks[(color, King)];
            if self.castling_rights[(color, Kingside)]
                && (self.combined & (king << 1 | king << 2)).is_empty()
                && (attacked_squares & (king << 1 | king << 2)).is_empty()
            {
                // generate castling kingside if rights remain, the way is clear and the squares aren't attacked
                let start = king.lsb_index() as u8;
                moves.push(Move::king_castle(start, start + 2));
            }
            if self.castling_rights[(color, Queenside)]
                && ((self.combined) & (king >> 1 | king >> 2 | king >> 3)).is_empty()
                && (attacked_squares & (king >> 1 | king >> 2)).is_empty()
            {
                // generate castling queenside if rights remain, the way is clear and the squares aren't attacked
                let start = king.lsb_index() as u8;
                moves.push(Move::king_castle(start, start - 2));
            }
        }
        // Pawn moves
        let pawns = self.piece_masks[(color, Pawn)] & pinned_pieces.inverse();
        if color == White {
            // white pawn moves
            for pawn_square in pawns {
                let pawn = BitBoard(1 << pawn_square);

                // single pawn pushes
                let pawn_push_one = (pawn << 8) & push_mask & (self.combined).inverse();
                if pawn_push_one.is_not_empty() {
                    let target = pawn_push_one.lsb_index() as u8;
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
                let pawn_push_two = ((((pawn & SECOND_RANK) << 8) & (self.combined).inverse())
                    << 8)
                    & (self.combined).inverse()
                    & push_mask;

                if pawn_push_two.is_not_empty() {
                    moves.push(Move::pawn_double_push(
                        pawn_square,
                        pawn_push_two.lsb_index() as u8,
                    ));
                }
                // pawn captures
                let pawn_captures = (((pawn & NOT_A_FILE) << 7) | ((pawn & NOT_H_FILE) << 9))
                    // if a double-pushed pawn is giving check, mark it as takeable by en passent
                    & (capture_mask | (self.en_passent_mask & (capture_mask << 8)))
                    & (self.color_masks[!color] | self.en_passent_mask);
                for target in pawn_captures {
                    if target / 8 == 7 {
                        // promotions
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Knight));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Bishop));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Rook));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Queen));
                    } else if BitBoard(1 << target) == self.en_passent_mask {
                        // en passent capture
                        if self.piece_masks[(color, King)].lsb_index() / 8 == 4 {
                            let mut en_passent_pinned = false;
                            let blocking_mask = (self.combined)
                                & (BitBoard(1 << pawn_square) | (self.en_passent_mask >> 8))
                                    .inverse();
                            let attacking_rooks_or_queens = (self.piece_masks[(!color, Rook)]
                                | self.piece_masks[(!color, Queen)])
                                & FIFTH_RANK;
                            for rook_square in attacking_rooks_or_queens {
                                if (lookup_rook(rook_square.into(), blocking_mask)
                                    & self.piece_masks[(color, King)])
                                    .is_not_empty()
                                {
                                    en_passent_pinned = true;
                                    break;
                                }
                            }
                            let attacking_queens = self.piece_masks[(!color, Queen)] & FOURTH_RANK;
                            for queen_square in attacking_queens {
                                if (lookup_queen(queen_square.into(), blocking_mask)
                                    & self.piece_masks[(color, King)])
                                    .is_not_empty()
                                {
                                    en_passent_pinned = true;
                                    break;
                                }
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
                }
            }
        } else {
            // black pawn moves
            for pawn_square in pawns {
                let pawn = BitBoard(1 << pawn_square);

                // single pawn pushes
                let pawn_push_one = pawn >> 8 & push_mask & (self.combined).inverse();
                if pawn_push_one.is_not_empty() {
                    let target = pawn_push_one.lsb_index() as u8;
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
                let pawn_push_two = ((((pawn & SEVENTH_RANK) >> 8) & (self.combined).inverse())
                    >> 8)
                    & (self.combined).inverse()
                    & push_mask;
                if pawn_push_two.is_not_empty() {
                    moves.push(Move::pawn_double_push(
                        pawn_square,
                        pawn_push_two.lsb_index() as u8,
                    ));
                }
                // pawn captures
                let pawn_captures = (((pawn & NOT_A_FILE) >> 9) | ((pawn & NOT_H_FILE) >> 7))
                    // if a double-pushed pawn is giving check, mark it as takeable by en passent
                    & (capture_mask | (self.en_passent_mask & (capture_mask >> 8)))
                    & (self.color_masks[!color] | self.en_passent_mask);
                for target in pawn_captures {
                    if target / 8 == 0 {
                        // promotions
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Knight));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Bishop));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Rook));
                        moves.push(Move::pawn_capture_promotion(pawn_square, target, Queen));
                    } else if BitBoard(1 << target) == self.en_passent_mask {
                        // en passent capture
                        if self.piece_masks[(color, King)].lsb_index() / 8 == 3 {
                            let mut en_passent_pinned = false;
                            let blocking_mask = (self.combined)
                                & (BitBoard(1 << pawn_square) | self.en_passent_mask << 8)
                                    .inverse();
                            let attacking_rooks_or_queens = (self.piece_masks[(!color, Rook)]
                                | self.piece_masks[(!color, Queen)])
                                & FOURTH_RANK;
                            for rook_square in attacking_rooks_or_queens {
                                if (lookup_rook(rook_square.into(), blocking_mask)
                                    & self.piece_masks[(color, King)])
                                    .is_not_empty()
                                {
                                    en_passent_pinned = true;
                                    break;
                                }
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
                }
            }
        }

        // Knight moves
        let knights = self.piece_masks[(color, Knight)] & pinned_pieces.inverse();
        for knight_square in knights {
            let attacks = lookup_knight(knight_square.into())
                & self.color_masks[color].inverse()
                & (push_mask | capture_mask);
            for target in attacks {
                let capture = (self.color_masks[!color] & BitBoard(1 << target)).is_not_empty();
                moves.push(Move::knight_move(knight_square as u8, target, capture));
            }
        }

        // Bishop moves
        let bishops = self.piece_masks[(color, Bishop)] & pinned_pieces.inverse();
        for bishop_square in bishops {
            let attacks = lookup_bishop(bishop_square.into(), self.combined)
                & self.color_masks[color].inverse()
                & (push_mask | capture_mask);
            for target in attacks {
                let capture = (self.color_masks[!color] & BitBoard(1 << target)).is_not_empty();
                moves.push(Move::bishop_move(bishop_square as u8, target, capture));
            }
        }

        // Rook moves
        let rooks = self.piece_masks[(color, Rook)] & pinned_pieces.inverse();
        for rook_square in rooks {
            let attacks = lookup_rook(rook_square.into(), self.combined)
                & self.color_masks[color].inverse()
                & (push_mask | capture_mask);
            for target in attacks {
                let capture = (self.color_masks[!color] & BitBoard(1 << target)).is_not_empty();
                moves.push(Move::rook_move(rook_square as u8, target, capture));
            }
        }

        // queen moves
        let queens = self.piece_masks[(color, Queen)] & pinned_pieces.inverse();
        for queen_square in queens {
            let attacks = lookup_queen(queen_square.into(), self.combined)
                & self.color_masks[color].inverse()
                & (push_mask | capture_mask);
            for target in attacks {
                let capture = (self.color_masks[!color] & BitBoard(1 << target)).is_not_empty();
                moves.push(Move::queen_move(queen_square as u8, target, capture));
            }
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
            self.piece_masks[(color, King)] ^= BitBoard(1 << target) | BitBoard(1 << start);
            // update rook position and hash
            self.hash ^=
                zobrist_piece(Rook, color, rook_start) ^ zobrist_piece(Rook, color, rook_target);
            self.piece_masks[(color, Rook)] ^=
                BitBoard(1 << rook_target) | BitBoard(1 << rook_start);
            // update color masks
            self.color_masks[color] ^= BitBoard(1 << start)
                | BitBoard(1 << target)
                | BitBoard(1 << rook_start)
                | BitBoard(1 << rook_target);
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
            self.piece_masks[(!color, captured)] ^= BitBoard(1 << cap_square);
            self.color_masks[!color] ^= BitBoard(1 << cap_square);

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
            self.piece_masks[(color, piece)] ^= BitBoard(1 << start) | BitBoard(1 << target);
            self.color_masks[color] ^= BitBoard(1 << start) | BitBoard(1 << target);
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
                self.en_passent_mask = BitBoard(1 << ep_square) & self.pawn_attacks(!color);
                if self.en_passent_mask.is_not_empty() {
                    self.hash ^= zobrist_enpassent(self.en_passent_mask);
                }
            }
            // promotion
            if move_.promotion() != NoPiece {
                self.hash ^= zobrist_piece(Pawn, color, target)
                    ^ zobrist_piece(move_.promotion(), color, target);
                self.piece_masks[(color, Pawn)] ^= BitBoard(1 << target);
                self.piece_masks[(color, move_.promotion())] |= BitBoard(1 << target);
            }
            // rule 50
            self.halfmove_clock = 0;
        }

        // swap players
        self.hash ^= zobrist_player();
        self.current_player = !self.current_player;

        // update combined mask
        self.combined = self.color_masks[White] | self.color_masks[Black];

        // debug_assert!(self.hash == self.zobrist_hash());
    }

    pub fn unmake_move(&mut self) {
        self.current_player = !self.current_player;

        let unmove = self.unmove_history.pop().unwrap();
        let start = unmove.start as usize;
        let target = unmove.target as usize;

        let mut piece = self.piece_at(target);
        if unmove.promotion {
            self.piece_masks[(self.current_player, piece)] ^= BitBoard(1 << target);

            self.piece_masks[(self.current_player, Pawn)] ^= BitBoard(1 << target);
            piece = Pawn;
        }

        if unmove.castling {
            if target % 8 == 2 {
                // queenside
                self.piece_masks[(self.current_player, King)] ^=
                    BitBoard(1 << start) | BitBoard(1 << target);

                let rook_start = target - 2;
                let rook_target = target + 1;

                self.piece_masks[(self.current_player, Rook)] ^=
                    BitBoard(1 << rook_start) | BitBoard(1 << rook_target);

                self.color_masks[self.current_player] ^= BitBoard(1 << start)
                    | BitBoard(1 << target)
                    | BitBoard(1 << rook_start)
                    | BitBoard(1 << rook_target);
            } else {
                // kingside
                self.piece_masks[(self.current_player, King)] ^=
                    BitBoard(1 << start) | BitBoard(1 << target);

                let rook_start = target + 1;
                let rook_target = target - 1;

                self.piece_masks[(self.current_player, Rook)] ^=
                    BitBoard(1 << rook_start) | BitBoard(1 << rook_target);

                self.color_masks[self.current_player] ^= BitBoard(1 << start)
                    | BitBoard(1 << target)
                    | BitBoard(1 << rook_start)
                    | BitBoard(1 << rook_target);
            }
        } else {
            // move piece back to start
            self.piece_masks[(self.current_player, piece)] ^=
                BitBoard(1 << start) | BitBoard(1 << target);
            self.color_masks[self.current_player] ^= BitBoard(1 << start) | BitBoard(1 << target);

            if unmove.capture != NoPiece {
                let mut cap_square = target;
                if unmove.en_passent {
                    cap_square = match self.current_player {
                        White => target - 8,
                        Black => target + 8,
                    };
                }
                // replace captured piece
                self.piece_masks[(!self.current_player, unmove.capture)] ^=
                    BitBoard(1 << cap_square);
                self.color_masks[!self.current_player] ^= BitBoard(1 << cap_square);
            }
        }

        // restore board state
        self.castling_rights = unmove.castling_rights;
        self.en_passent_mask = unmove.en_passent_mask;
        self.hash = self.position_history.pop().unwrap();
        self.halfmove_clock = unmove.halfmove_clock;

        self.combined = self.color_masks[White] | self.color_masks[Black];

        // debug_assert!(self.hash == self.zobrist_hash());
    }

    pub fn make_null_move(&mut self) {
        let unmove = UnMove::new(
            0,
            0,
            false,
            NoPiece,
            false,
            self.en_passent_mask,
            false,
            self.castling_rights,
            0,
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
                    hash ^= zobrist_piece(piece, color, square.into());
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

    pub fn perft(&mut self, depth: usize) -> usize {
        if depth == 1 {
            return self.legal_moves().len();
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

#[cfg(test)]
mod tests {
    use crate::{
        chessgame::ChessGame,
        lookup_tables::LookupTables,
        transposition_table::{TranspositionTable, TT_DEFAULT_SIZE},
        zobrist::initialise_zobrist_numbers,
    };

    #[test]
    fn search_speed() -> Result<(), ()> {
        LookupTables::generate_all(true);
        initialise_zobrist_numbers();
        let tt = TranspositionTable::new(TT_DEFAULT_SIZE);
        let game = ChessGame::new(tt);

        game.search(Some(6), true);

        Ok(())
    }
}
