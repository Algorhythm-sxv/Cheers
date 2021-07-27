use itertools::repeat_n;

use crate::lookup_tables::*;
use crate::types::*;

#[derive(Clone, Default, Debug)]
pub struct BitBoards {
    color_masks: ColorMasks,
    piece_masks: PieceMasks,
    piece_list: Vec<Option<(PieceIndex, ColorIndex)>>,
    current_player: ColorIndex,
    castling_rights: CastlingRights,
    en_passent_mask: u64,
    halfmove_clock: u8,
    move_history: Vec<UnmakeMove>,
}

impl BitBoards {
    /// Creates a new set of bitboards in the starting position
    pub fn new() -> Self {
        let mut boards = BitBoards {
            ..Default::default()
        };
        boards.reset();

        boards
    }
    pub fn reset(&mut self) {
        let mut piece_list = vec![None; 64];

        piece_list.splice(
            0..8,
            [
                Some((Rook, White)),
                Some((Knight, White)),
                Some((Bishop, White)),
                Some((Queen, White)),
                Some((King, White)),
                Some((Bishop, White)),
                Some((Knight, White)),
                Some((Rook, White)),
            ],
        );

        piece_list.splice(8..16, repeat_n(Some((Pawn, White)), 8));
        piece_list.splice(48..56, repeat_n(Some((Pawn, Black)), 8));

        piece_list.splice(
            56..64,
            [
                Some((Rook, Black)),
                Some((Knight, Black)),
                Some((Bishop, Black)),
                Some((Queen, Black)),
                Some((King, Black)),
                Some((Bishop, Black)),
                Some((Knight, Black)),
                Some((Rook, Black)),
            ],
        );

        let black_mask = 0xFFFF000000000000;
        let white_mask = 0x000000000000FFFF;

        let pawn_mask = 0x00FF00000000FF00;
        let bishop_mask = 0x2400000000000024;
        let knight_mask = 0x4200000000000042;
        let rook_mask = 0x8100000000000081;

        let queen_mask = 0x0800000000000008;
        let king_mask = 0x1000000000000010;

        self.piece_list = piece_list;
        self.color_masks = ColorMasks([white_mask, black_mask]);
        self.piece_masks = PieceMasks([
            pawn_mask,
            bishop_mask,
            knight_mask,
            rook_mask,
            queen_mask,
            king_mask,
        ]);
        self.current_player = White;
        self.castling_rights = CastlingRights([[true, true], [true, true]]);
    }

    pub fn set_from_fen(&mut self, fen: String) {
        self.piece_masks = PieceMasks([0; 6]);
        self.color_masks = ColorMasks([0; 2]);

        self.piece_list.fill(None);

        let lines = fen.split(&['/', ' '][..]);

        for (i, line) in lines.take(8).enumerate() {
            let mut index = 56 - i * 8;
            for chr in line.chars() {
                match chr {
                    'n' => {
                        self.piece_list[index] = Some((Knight, Black));
                        self.piece_masks[Knight] |= 1 << index;
                        self.color_masks[Black] |= 1 << index;
                    }
                    'N' => {
                        self.piece_list[index] = Some((Knight, White));
                        self.piece_masks[Knight] |= 1 << index;
                        self.color_masks[White] |= 1 << index;
                    }
                    'b' => {
                        self.piece_list[index] = Some((Bishop, Black));
                        self.piece_masks[Bishop] |= 1 << index;
                        self.color_masks[Black] |= 1 << index;
                    }
                    'B' => {
                        self.piece_list[index] = Some((Bishop, White));
                        self.piece_masks[Bishop] |= 1 << index;
                        self.color_masks[White] |= 1 << index;
                    }
                    'r' => {
                        self.piece_list[index] = Some((Rook, Black));
                        self.piece_masks[Rook] |= 1 << index;
                        self.color_masks[Black] |= 1 << index;
                    }
                    'R' => {
                        self.piece_list[index] = Some((Rook, White));
                        self.piece_masks[Rook] |= 1 << index;
                        self.color_masks[White] |= 1 << index;
                    }
                    'q' => {
                        self.piece_list[index] = Some((Queen, Black));
                        self.piece_masks[Queen] |= 1 << index;
                        self.color_masks[Black] |= 1 << index;
                    }
                    'Q' => {
                        self.piece_list[index] = Some((Queen, White));
                        self.piece_masks[Queen] |= 1 << index;
                        self.color_masks[White] |= 1 << index;
                    }
                    'k' => {
                        self.piece_list[index] = Some((King, Black));
                        self.piece_masks[King] |= 1 << index;
                        self.color_masks[Black] |= 1 << index;
                    }
                    'K' => {
                        self.piece_list[index] = Some((King, White));
                        self.piece_masks[King] |= 1 << index;
                        self.color_masks[White] |= 1 << index;
                    }
                    'p' => {
                        self.piece_list[index] = Some((Pawn, Black));
                        self.piece_masks[Pawn] |= 1 << index;
                        self.color_masks[Black] |= 1 << index;
                    }
                    'P' => {
                        self.piece_list[index] = Some((Pawn, White));
                        self.piece_masks[Pawn] |= 1 << index;
                        self.color_masks[White] |= 1 << index;
                    }
                    other @ _ => panic!("unknown character in FEN: {}", other),
                }
                index += 1;
            }
        }
        // TODO: stuff with the rest of the FEN
    }

    pub fn knight_attacks(&self, color: ColorIndex) -> u64 {
        let mut knights = self.piece_masks[Knight] & self.color_masks[color];

        let mut result = 0;
        while knights != 0 {
            let i = knights.trailing_zeros() as usize;
            result |= lookup_tables().lookup_knight(i);
            knights ^= 1 << i;
        }
        result
    }

    pub fn knight_moves(&self, color: ColorIndex, moves: &mut Vec<Move>) {
        let mut knights = self.piece_masks[Knight] & self.color_masks[color];
        while knights != 0 {
            let i = knights.trailing_zeros() as usize;
            let mut result = lookup_tables().lookup_knight(i) & !self.color_masks[color];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                moves.push(Move::new(i as u8, target, None));

                result ^= 1 << target;
            }
            knights ^= 1 << i;
        }
    }

    pub fn bishop_attacks(&self, color: ColorIndex) -> u64 {
        let mut bishops = self.piece_masks[Bishop] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        let mut result = 0;
        while bishops != 0 {
            let i = bishops.trailing_zeros() as usize;
            result |= lookup_tables().lookup_bishop(i, blocking_mask);
            bishops ^= 1 << i;
        }
        result
    }

    pub fn bishop_moves(&self, color: ColorIndex, moves: &mut Vec<Move>) {
        let mut bishops = self.piece_masks[Bishop] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        while bishops != 0 {
            let i = bishops.trailing_zeros() as usize;
            let mut result =
                lookup_tables().lookup_bishop(i, blocking_mask) & !self.color_masks[color];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                moves.push(Move::new(i as u8, target, None));

                result ^= 1 << target;
            }
            bishops ^= 1 << i;
        }
    }

    pub fn rook_attacks(&self, color: ColorIndex) -> u64 {
        let mut rooks = self.piece_masks[Rook] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        let mut result = 0;
        while rooks != 0 {
            let i = rooks.trailing_zeros() as usize;
            result |= lookup_tables().lookup_rook(i, blocking_mask);
            rooks ^= 1 << i;
        }
        result
    }

    pub fn rook_moves(&self, color: ColorIndex, moves: &mut Vec<Move>) {
        let mut rooks = self.piece_masks[Rook] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        while rooks != 0 {
            let i = rooks.trailing_zeros() as usize;
            let mut result =
                lookup_tables().lookup_rook(i, blocking_mask) & !self.color_masks[color];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                moves.push(Move::new(i as u8, target, None));

                result ^= 1 << target;
            }
            rooks ^= 1 << i;
        }
    }

    pub fn queen_attacks(&self, color: ColorIndex) -> u64 {
        let mut queens = self.piece_masks[Queen] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        let mut result = 0;
        while queens != 0 {
            let i = queens.trailing_zeros() as usize;
            result |= lookup_tables().lookup_queen(i, blocking_mask);
            queens ^= 1 << i;
        }
        result
    }

    pub fn queen_moves(&self, color: ColorIndex, moves: &mut Vec<Move>) {
        let mut queens = self.piece_masks[Queen] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        while queens != 0 {
            let i = queens.trailing_zeros() as usize;
            let mut result =
                lookup_tables().lookup_queen(i, blocking_mask) & !self.color_masks[color];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                moves.push(Move::new(i as u8, target, None));

                result ^= 1 << target;
            }
            queens ^= 1 << i;
        }
    }

    pub fn king_attacks(&self, color: ColorIndex) -> u64 {
        let king = self.piece_masks[King] & self.color_masks[color];

        lookup_tables().lookup_king(king.trailing_zeros() as usize)
    }

    pub fn king_moves(&self, color: ColorIndex, moves: &mut Vec<Move>) {
        let king = self.piece_masks[King] & self.color_masks[color];
        let square = king.trailing_zeros() as usize;

        let mut result = lookup_tables().lookup_king(square) & !self.color_masks[color];

        while result != 0 {
            let target = result.trailing_zeros() as u8;
            moves.push(Move::new(square as u8, target, None));

            result ^= 1 << target;
        }
    }

    pub fn pawn_attacks(&self, color: ColorIndex) -> u64 {
        match color {
            White => {
                let pawns = self.piece_masks[Pawn] & self.color_masks[White];
                let west_attacks = (pawns << 7) & NOT_H_FILE;
                let east_attacks = (pawns << 9) & NOT_A_FILE;

                west_attacks | east_attacks
            }
            Black => {
                let pawns = self.piece_masks[Pawn] & self.color_masks[Black];
                let west_attacks = (pawns >> 9) & NOT_H_FILE;
                let east_attacks = (pawns >> 7) & NOT_A_FILE;

                west_attacks | east_attacks
            }
        }
    }

    pub fn all_attacks(&self, color: ColorIndex) -> u64 {
        self.pawn_attacks(color)
            | self.knight_attacks(color)
            | self.king_attacks(color)
            | self.bishop_attacks(color)
            | self.rook_attacks(color)
            | self.queen_attacks(color)
    }

    pub fn white_pawn_moves(&self, moves: &mut Vec<Move>) {
        let mut pawns = self.piece_masks[Pawn] & self.color_masks[White];

        while pawns != 0 {
            let i = pawns.trailing_zeros() as usize;
            let mut result = lookup_tables().lookup_pawn_push(i, White);

            let empty = !(self.color_masks[White] | self.color_masks[Black]);

            // add double pushes to relevant unblocked single pushes
            result |= (result & THIRD_RANK & empty) << 8;

            // remove blocked double pushes
            result &= empty;

            let attacks = lookup_tables().lookup_pawn_attack(i, White);
            result |= attacks & self.color_masks[Black];

            // taking en passent
            result |= attacks & self.en_passent_mask;

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                if target / 8 == 7 {
                    moves.push(Move::new(i as u8, target, Some(Queen)));
                    moves.push(Move::new(i as u8, target, Some(Rook)));
                    moves.push(Move::new(i as u8, target, Some(Knight)));
                    moves.push(Move::new(i as u8, target, Some(Bishop)));
                } else {
                    moves.push(Move::new(i as u8, target, None));
                }

                result ^= 1 << target;
            }
            pawns ^= 1 << i;
        }
    }

    pub fn black_pawn_moves(&self, moves: &mut Vec<Move>) {
        let mut pawns = self.piece_masks[Pawn] & self.color_masks[Black];

        while pawns != 0 {
            let i = pawns.trailing_zeros() as usize;
            let mut result = lookup_tables().lookup_pawn_push(i, Black);

            let empty = !(self.color_masks[White] | self.color_masks[Black]);

            // add double pushes to relevant unblocked single pushes
            result |= (result & SIXTH_RANK & empty) >> 8;

            // remove blocked double pushes
            result &= empty;

            let attacks = lookup_tables().lookup_pawn_attack(i, Black);
            result |= attacks & self.color_masks[White];

            // taking en passent
            result |= attacks & self.en_passent_mask;

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                if target / 8 == 0 {
                    moves.push(Move::new(i as u8, target, Some(Queen)));
                    moves.push(Move::new(i as u8, target, Some(Rook)));
                    moves.push(Move::new(i as u8, target, Some(Knight)));
                    moves.push(Move::new(i as u8, target, Some(Bishop)));
                } else {
                    moves.push(Move::new(i as u8, target, None));
                }

                result ^= 1 << target;
            }
            pawns ^= 1 << i;
        }
    }

    /// generate legal castling moves, check for castling into, out of and through check
    fn legal_castles(&self, color: ColorIndex) -> Vec<Move> {
        let mut moves = Vec::new();

        let all_attacks = self.all_attacks(!color);
        let king = self.piece_masks[King] & self.color_masks[color];

        // can't castle out of check
        if all_attacks & king != 0 {
            return moves;
        }

        let square = king.trailing_zeros() as u8;

        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        // kingside
        if self.castling_rights[(color, Kingside)]
            // file is clear to the rook
            && lookup_tables().lookup_rook(square as usize, blocking_mask) & H_FILE != 0
            // can't castle through or into check
            && all_attacks & ((king << 1) | (king << 2)) == 0
        {
            moves.push(Move::new(square, square + 2, None))
        }
        // queenside
        if self.castling_rights[(color, Queenside)]
            // file is clear to the rook
            && lookup_tables().lookup_rook(square as usize, blocking_mask) & A_FILE != 0
            // can't castle through or into check
            && all_attacks & ((king >> 1) | (king >> 2)) == 0
        {
            moves.push(Move::new(square, square - 2, None))
        }

        moves
    }

    pub fn generate_legal_moves(&mut self) -> Vec<Move> {
        let moves = self.generate_pseudolegal_moves();

        let mut moves: Vec<_> = moves
            .into_iter()
            .filter(|move_| {
                self.make_move(move_);
                let all_attacks = self.pawn_attacks(self.current_player)
                    | self.knight_attacks(self.current_player)
                    | self.king_attacks(self.current_player)
                    | self.bishop_attacks(self.current_player)
                    | self.rook_attacks(self.current_player)
                    | self.queen_attacks(self.current_player);
                let result =
                    all_attacks & self.piece_masks[King] & self.color_masks[!self.current_player]
                        == 0;
                self.unmake_move();
                result
            })
            .collect();
        moves.extend(self.legal_castles(self.current_player));
        moves
    }

    /// Generate legal moves except castling
    pub fn generate_pseudolegal_moves(&self) -> Vec<Move> {
        self._generate_pseudolegal_moves(self.current_player)
    }

    fn _generate_pseudolegal_moves(&self, color: ColorIndex) -> Vec<Move> {
        // try to avoid re-allocation
        let mut moves = Vec::with_capacity(50);

        self.knight_moves(color, &mut moves);
        self.bishop_moves(color, &mut moves);
        self.rook_moves(color, &mut moves);
        self.queen_moves(color, &mut moves);
        self.king_moves(color, &mut moves);
        match color {
            White => self.white_pawn_moves(&mut moves),
            Black => self.black_pawn_moves(&mut moves),
        }

        moves
    }

    pub fn make_move(&mut self, move_: &Move) {
        let mut unmove = UnmakeMove {
            start: move_.start,
            target: move_.target,
            halfmove_clock: self.halfmove_clock,
            castling_rights: self.castling_rights,
            en_passent_mask: self.en_passent_mask,
            ..Default::default()
        };

        // assume we only get legal moves from the UI
        let (mut piece, color) = self.piece_list[move_.start as usize].unwrap();

        // increment the halfmove clock (resets are handled elsewhere)
        self.halfmove_clock += 1;

        // take a piece off the target square
        if let Some((taken_piece, taken_color)) = self.piece_list[move_.target as usize] {
            unmove.taken = Some(taken_piece);

            self.piece_masks[taken_piece] ^= 1 << move_.target;
            self.color_masks[taken_color] ^= 1 << move_.target;

            // reset the halfmove clock on capture
            self.halfmove_clock = 0;
        }

        // move the piece to the target square
        self.piece_masks[piece] |= 1 << move_.target;
        self.piece_masks[piece] ^= 1 << move_.start;

        // update the color mask
        self.color_masks[color] |= 1 << move_.target;
        self.color_masks[color] ^= 1 << move_.start;

        // update castling rights and move the castling rook
        if piece == King {
            self.castling_rights[color] = [false, false];
            if (move_.target as i8 - move_.start as i8).abs() == 2 {
                unmove.castling = true;
                if move_.target % 8 == 6 {
                    // kingside
                    let rook = self.piece_masks[Rook] & self.color_masks[color] & H_FILE;
                    self.piece_masks[Rook] ^= rook | (rook >> 2);
                    self.color_masks[color] ^= rook | (rook >> 2);
                    self.piece_list[move_.target as usize + 1] = None;
                    self.piece_list[move_.target as usize - 1] = Some((Rook, color));
                } else {
                    // queenside
                    let rook = self.piece_masks[Rook] & self.color_masks[color] & A_FILE;
                    self.piece_masks[Rook] ^= rook | (rook << 3);
                    self.color_masks[color] ^= rook | (rook << 3);
                    self.piece_list[move_.target as usize - 2] = None;
                    self.piece_list[move_.target as usize + 1] = Some((Rook, color));
                }
            }
        }

        // update castling rights
        if piece == Rook {
            if move_.start % 8 == 0 {
                // queenside
                self.castling_rights[(color, Queenside)] = false;
            } else if move_.start % 8 == 7 {
                // kingside
                self.castling_rights[(color, Kingside)] = false;
            }
        }

        // pawn move specialties
        if piece == Pawn {
            // reset halfmove clock
            self.halfmove_clock = 0;

            // en passent capture
            if move_.target == self.en_passent_mask.trailing_zeros() as u8 {
                self.piece_masks[Pawn] &=
                    !((self.en_passent_mask << 8) | (self.en_passent_mask >> 8));
                self.color_masks[!color] &=
                    !((self.en_passent_mask << 8) | (self.en_passent_mask >> 8));
                self.piece_list[move_.target as usize + 8 - 16 * color as usize] = None;
            }

            // update en passent state
            if (move_.target as i8 - move_.start as i8).abs() == 16 {
                // double push
                self.en_passent_mask = 1 << (move_.target - 8) << (16 * color as u8);
            } else {
                // single push/capture
                self.en_passent_mask = 0;
            }

            // promotion
            if let Some(target_piece) = move_.promotion {
                unmove.promotion = true;
                self.piece_masks[Pawn] ^= 1 << move_.target;
                self.piece_masks[target_piece] |= 1 << move_.target;
                piece = target_piece;
            }
        } else {
            // moving other pieces clears en passent state

            self.en_passent_mask = 0;
        }

        // update piece list
        self.piece_list[move_.start as usize] = None;
        self.piece_list[move_.target as usize] = Some((piece, color));

        // switch current player
        self.current_player = !self.current_player;

        // add move details to history
        self.move_history.push(unmove);
    }

    pub fn unmake_move(&mut self) {
        let unmove = self.move_history.pop().unwrap();

        let (mut piece, color) = self.piece_list[unmove.target as usize].unwrap();

        // undo castling
        if unmove.castling {
            // move the castling rook back
            // kingside
            if unmove.target % 8 == 7 {
                self.piece_list[unmove.target as usize - 1] = None;
                self.piece_list[unmove.target as usize + 1] = Some((Rook, color));
                let mask = (1 << (unmove.target - 1)) | (1 << unmove.target + 1);
                self.piece_masks[Rook] ^= mask;
                self.color_masks[color] ^= mask;
            // queenside
            } else {
                self.piece_list[unmove.target as usize + 1] = None;
                self.piece_list[unmove.target as usize - 2] = Some((Rook, color));
                let mask = (1 << (unmove.target - 2)) | (1 << unmove.target + 1);
                self.piece_masks[Rook] ^= mask;
                self.color_masks[color] ^= mask;
            }
        }

        // undo promotion
        if unmove.promotion {
            let (promoted, _) = self.piece_list[unmove.target as usize].unwrap();
            self.piece_list[unmove.target as usize] = None;
            self.piece_masks[promoted] ^= 1 << unmove.target;
            self.color_masks[color] ^= 1 << unmove.target;

            piece = Pawn;
        }

        // update piece list (target square gets updated with captures)
        self.piece_list[unmove.start as usize] = Some((piece, color));

        // update piece/color masks
        self.piece_masks[piece] ^= (1 << unmove.target) | (1 << unmove.start);
        self.color_masks[color] ^= (1 << unmove.target) | (1 << unmove.start);

        // reset castling rights
        self.castling_rights = unmove.castling_rights;

        // reset en passent mask
        self.en_passent_mask = unmove.en_passent_mask;

        // reset halfmove clock
        self.halfmove_clock = unmove.halfmove_clock;

        // replace pawn taken en passent
        if unmove.en_passent {
            let shift = unmove.target as usize - 8 + (16 * color as usize);
            self.piece_masks[Pawn] |= 1 << shift;
            self.color_masks[!color] |= 1 << shift;
            self.piece_list[shift] = Some((Pawn, !color));

        // replace other taken pieces
        } else if let Some(taken_piece) = unmove.taken {
            self.piece_masks[taken_piece] |= 1 << unmove.target;
            self.color_masks[!color] |= 1 << unmove.target;
            self.piece_list[unmove.target as usize] = Some((taken_piece, !color));

        // clear the square if the move was not a capture
        } else {
            self.piece_list[unmove.target as usize] = None;
        }

        // switch current player
        self.current_player = !self.current_player;
    }

    /// Get a reference to the bit boards's piece masks.
    pub fn piece_masks(&self) -> &PieceMasks {
        &self.piece_masks
    }
}
