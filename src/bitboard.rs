use itertools::repeat_n;

use crate::lookup_tables::*;
use crate::transposition_table::TranspositionTable;
use crate::types::*;
use crate::zobrist::*;

#[derive(Clone, Default, Debug)]
pub struct BitBoards {
    pub color_masks: ColorMasks,
    pub piece_masks: PieceMasks,
    pub piece_list: Vec<Option<(PieceIndex, ColorIndex)>>,
    pub current_player: ColorIndex,
    pub castling_rights: CastlingRights,
    pub en_passent_mask: u64,
    pub halfmove_clock: u8,
    pub move_history: Vec<UnmakeMove>,
    pub position_hash: u64,
    pub position_history: Vec<u64>,
    pub transposition_table: TranspositionTable,
}

impl BitBoards {
    /// Creates a new set of bitboards in the starting position
    pub fn new(table_size: usize) -> Self {
        let mut boards = BitBoards {
            transposition_table: TranspositionTable::new(table_size),
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

        let hash = zobrist_hash(self);
        self.position_hash = hash;
        self.position_history.clear();

        self.move_history.clear();
        self.halfmove_clock = 0;
    }

    pub fn set_from_fen(&mut self, fen: String) -> Result<(), Box<dyn std::error::Error>> {
        self.piece_masks = PieceMasks([0; 6]);
        self.color_masks = ColorMasks([0; 2]);

        self.piece_list.fill(None);

        let mut lines = fen.split(&['/', ' '][..]);

        for (i, line) in lines.clone().take(8).enumerate() {
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
                    digit @ '1'..='8' => index += digit.to_digit(10).unwrap() as usize - 1,
                    other @ _ => eprintln!("Unexpected character in FEN: {}", other),
                }
                index += 1;
            }
        }

        match lines.nth(8).ok_or(String::from("No metadata!"))? {
            "w" => self.current_player = White,
            "b" => self.current_player = Black,
            other @ _ => Err(format!("Invalid player character: {}", other))?,
        }

        match lines
            .next()
            .ok_or(String::from("Insufficient metadata for castling rights!"))?
        {
            "-" => self.castling_rights = CastlingRights([[false, false], [false, false]]),
            other @ _ => other.chars().try_for_each(|chr| match chr {
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
            .ok_or(String::from("Insufficient metadata for en passent square!"))?
        {
            "-" => self.en_passent_mask = 0,
            other @ _ => {
                let mut square = 0;
                match other
                    .bytes()
                    .nth(0)
                    .ok_or(format!("Empty en passent string!"))?
                {
                    file @ b'a'..=b'h' => square += file - b'a',
                    other @ _ => Err(format!("Invalid en passent file: {}", other))?,
                }
                match other
                    .bytes()
                    .nth(1)
                    .ok_or(format!("En passent string too short"))?
                {
                    rank @ b'1'..=b'8' => square += 8 * (rank - b'1'),
                    other @ _ => Err(format!("Invalid en passent rank: {}", other))?,
                }
                self.en_passent_mask = 1 << square;
            }
        }

        self.halfmove_clock = lines
            .next()
            .ok_or(String::from("No halfmove clock!"))?
            .parse::<u8>()?;

        Ok(())
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

    pub fn knight_captures(&self, color: ColorIndex, captures: &mut Vec<Capture>) {
        let mut knights = self.piece_masks[Knight] & self.color_masks[color];

        while knights != 0 {
            let i = knights.trailing_zeros() as usize;
            let mut result = lookup_tables().lookup_knight(i) & self.color_masks[!color];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                let capture = self.piece_list[target as usize].unwrap().0;
                captures.push(Capture::new(i as u8, target, Knight, capture, None));

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

    pub fn bishop_captures(&self, color: ColorIndex, captures: &mut Vec<Capture>) {
        let mut bishops = self.piece_masks[Bishop] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        while bishops != 0 {
            let i = bishops.trailing_zeros() as usize;
            let mut result =
                lookup_tables().lookup_bishop(i, blocking_mask) & self.color_masks[!color];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                let capture = self.piece_list[target as usize].unwrap().0;
                captures.push(Capture::new(i as u8, target, Bishop, capture, None));

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

    pub fn rook_captures(&self, color: ColorIndex, captures: &mut Vec<Capture>) {
        let mut rooks = self.piece_masks[Rook] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        while rooks != 0 {
            let i = rooks.trailing_zeros() as usize;
            let mut result =
                lookup_tables().lookup_rook(i, blocking_mask) & self.color_masks[!color];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                let capture = self.piece_list[target as usize].unwrap().0;
                captures.push(Capture::new(i as u8, target, Rook, capture, None));

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

    pub fn queen_captures(&self, color: ColorIndex, captures: &mut Vec<Capture>) {
        let mut queens = self.piece_masks[Queen] & self.color_masks[color];
        let blocking_mask = self.color_masks[White] | self.color_masks[Black];

        while queens != 0 {
            let i = queens.trailing_zeros() as usize;
            let mut result =
                lookup_tables().lookup_queen(i, blocking_mask) & self.color_masks[!color];

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                let capture = self.piece_list[target as usize].unwrap().0;
                captures.push(Capture::new(i as u8, target, Queen, capture, None));

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

    pub fn king_captures(&self, color: ColorIndex, captures: &mut Vec<Capture>) {
        let king = self.piece_masks[King] & self.color_masks[color];
        let square = king.trailing_zeros() as usize;

        let mut result = lookup_tables().lookup_king(square) & self.color_masks[!color];

        while result != 0 {
            let target = result.trailing_zeros() as u8;
            let capture = self.piece_list[target as usize].unwrap().0;
            captures.push(Capture::new(square as u8, target, King, capture, None));

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

    pub fn white_pawn_captures(&self, captures: &mut Vec<Capture>) {
        let mut pawns = self.piece_masks[Pawn] & self.color_masks[White];

        while pawns != 0 {
            let i = pawns.trailing_zeros() as usize;
            let mut result = 0;
            let attacks = lookup_tables().lookup_pawn_attack(i, White);
            result |= attacks & self.color_masks[Black];

            // taking en passent
            result |= attacks & self.en_passent_mask;

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                let capture = self.piece_list[target as usize].unwrap_or((Pawn, Black)).0;
                if target / 8 == 7 {
                    captures.push(Capture::new(i as u8, target, Pawn, capture, Some(Queen)));
                    captures.push(Capture::new(i as u8, target, Pawn, capture, Some(Rook)));
                    captures.push(Capture::new(i as u8, target, Pawn, capture, Some(Knight)));
                    captures.push(Capture::new(i as u8, target, Pawn, capture, Some(Bishop)));
                } else {
                    captures.push(Capture::new(i as u8, target, Pawn, capture, None));
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

    pub fn black_pawn_captures(&self, captures: &mut Vec<Capture>) {
        let mut pawns = self.piece_masks[Pawn] & self.color_masks[Black];

        while pawns != 0 {
            let i = pawns.trailing_zeros() as usize;
            let mut result = 0;
            let attacks = lookup_tables().lookup_pawn_attack(i, Black);
            result |= attacks & self.color_masks[White];

            // taking en passent
            result |= attacks & self.en_passent_mask;

            while result != 0 {
                let target = result.trailing_zeros() as u8;
                // assume captures to empty square were en passent
                let capture = self.piece_list[target as usize].unwrap_or((Pawn, White)).0;
                if target / 8 == 0 {
                    captures.push(Capture::new(i as u8, target, Pawn, capture, Some(Queen)));
                    captures.push(Capture::new(i as u8, target, Pawn, capture, Some(Rook)));
                    captures.push(Capture::new(i as u8, target, Pawn, capture, Some(Knight)));
                    captures.push(Capture::new(i as u8, target, Pawn, capture, Some(Bishop)));
                } else {
                    captures.push(Capture::new(i as u8, target, Pawn, capture, None));
                }

                result ^= 1 << target;
            }
            pawns ^= 1 << i;
        }
    }

    /// generate legal castling moves, check for castling into, out of and through check
    fn legal_castles(&self, color: ColorIndex, moves: &mut Vec<Move>) {
        let all_attacks = self.all_attacks(!color);
        let king = self.piece_masks[King] & self.color_masks[color];

        // can't castle out of check
        if all_attacks & king != 0 {
            return;
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
    }

    pub fn king_not_in_check(&self, color: ColorIndex) -> bool {
        self.all_attacks(!color) & self.piece_masks[King] & self.color_masks[color] == 0
    }

    pub fn generate_legal_moves(&mut self) -> Vec<Move> {
        let moves = self.generate_pseudolegal_moves();
        let mut moves: Vec<_> = moves
            .into_iter()
            .filter(|move_| {
                self.make_move(move_);
                let result = self.king_not_in_check(!self.current_player);
                self.unmake_move();
                result
            })
            .collect();
        self.legal_castles(self.current_player, &mut moves);
        moves
    }

    /// Generate pseudolegal moves that can be tested by king check only
    pub fn generate_pseudolegal_moves(&self) -> Vec<Move> {
        let mut moves = self._generate_pseudolegal_moves(self.current_player);
        self.legal_castles(self.current_player, &mut moves);
        moves
    }

    /// Generates all psudolegal moves for a color other than castling
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

    /// Generate pseudolegal captures that can be tested by king check only
    pub fn generate_captures(&self) -> Vec<Capture> {
        let mut captures = Vec::with_capacity(50);

        self.knight_captures(self.current_player, &mut captures);
        self.bishop_captures(self.current_player, &mut captures);
        self.rook_captures(self.current_player, &mut captures);
        self.queen_captures(self.current_player, &mut captures);
        self.king_captures(self.current_player, &mut captures);
        match self.current_player {
            White => self.white_pawn_captures(&mut captures),
            Black => self.black_pawn_captures(&mut captures),
        }
        captures
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

        // add current position to history for repetition checking
        self.position_history.push(self.position_hash);

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

            // update zobrist hash with taken piece
            self.position_hash ^= zobrist_numbers()
                [zobrist_piece_index(taken_piece, taken_color, move_.target as usize)];

            // if a rook is taken update castling rights
            if taken_piece == Rook {
                if move_.target % 8 == 7
                    && move_.target / 8 == 7 * (!color as u8)
                    && self.castling_rights[(!color, Kingside)]
                {
                    // kingside
                    self.position_hash ^=
                        zobrist_numbers()[zobrist_castling_index(self.castling_rights)];
                    self.castling_rights[(!color, Kingside)] = false;
                    self.position_hash ^=
                        zobrist_numbers()[zobrist_castling_index(self.castling_rights)];
                } else if move_.target % 8 == 0
                    && move_.target / 8 == 7 * (!color as u8)
                    && self.castling_rights[(!color, Queenside)]
                {
                    // queenside
                    self.position_hash ^=
                        zobrist_numbers()[zobrist_castling_index(self.castling_rights)];
                    self.castling_rights[(!color, Queenside)] = false;
                    self.position_hash ^=
                        zobrist_numbers()[zobrist_castling_index(self.castling_rights)];
                }
            }
        }

        // move the piece to the target square
        self.piece_masks[piece] |= 1 << move_.target;
        self.piece_masks[piece] ^= 1 << move_.start;

        // update the color mask
        self.color_masks[color] |= 1 << move_.target;
        self.color_masks[color] ^= 1 << move_.start;

        // update castling rights and move the castling rook
        if piece == King {
            if self.castling_rights[color] != [false, false] {
                self.position_hash ^=
                    zobrist_numbers()[zobrist_castling_index(self.castling_rights)];
                self.castling_rights[color] = [false, false];
                self.position_hash ^=
                    zobrist_numbers()[zobrist_castling_index(self.castling_rights)];
            }
            if (move_.target as i8 - move_.start as i8).abs() == 2 {
                unmove.castling = true;
                if move_.target % 8 == 6 {
                    // kingside
                    let rook = (self.piece_masks[King] & self.color_masks[color]) << 1;
                    self.piece_masks[Rook] ^= rook | (rook >> 2);
                    self.color_masks[color] ^= rook | (rook >> 2);
                    self.piece_list[move_.target as usize + 1] = None;
                    self.piece_list[move_.target as usize - 1] = Some((Rook, color));
                    self.position_hash ^= zobrist_numbers()
                        [zobrist_piece_index(Rook, color, move_.target as usize + 1)];
                    self.position_hash ^= zobrist_numbers()
                        [zobrist_piece_index(Rook, color, move_.target as usize - 1)];
                } else {
                    // queenside
                    let rook = (self.piece_masks[King] & self.color_masks[color]) >> 2;
                    self.piece_masks[Rook] ^= rook | (rook << 3);
                    self.color_masks[color] ^= rook | (rook << 3);
                    self.piece_list[move_.target as usize - 2] = None;
                    self.piece_list[move_.target as usize + 1] = Some((Rook, color));
                    self.position_hash ^= zobrist_numbers()
                        [zobrist_piece_index(Rook, color, move_.target as usize - 2)];
                    self.position_hash ^= zobrist_numbers()
                        [zobrist_piece_index(Rook, color, move_.target as usize + 1)];
                }
            }
        }

        if piece == Rook {
            // update castling rights
            self.position_hash ^= zobrist_numbers()[zobrist_castling_index(self.castling_rights)];
            if move_.start % 8 == 0 {
                // queenside
                self.castling_rights[(color, Queenside)] = false;
            } else if move_.start % 8 == 7 {
                // kingside
                self.castling_rights[(color, Kingside)] = false;
            }
            self.position_hash ^= zobrist_numbers()[zobrist_castling_index(self.castling_rights)];
        }

        // pawn move specialties
        if piece == Pawn {
            // reset halfmove clock
            self.halfmove_clock = 0;

            // en passent capture
            if move_.target == self.en_passent_mask.trailing_zeros() as u8 {
                unmove.en_passent = true;
                self.piece_masks[Pawn] &=
                    !((self.en_passent_mask << 8) | (self.en_passent_mask >> 8));
                self.color_masks[!color] &=
                    !((self.en_passent_mask << 8) | (self.en_passent_mask >> 8));
                self.piece_list[move_.target as usize - 8 + 16 * color as usize] = None;
                self.position_hash ^= zobrist_numbers()[zobrist_piece_index(
                    Pawn,
                    !color,
                    move_.target as usize - 8 + 16 * color as usize,
                )];
            }

            // update en passent state
            if (move_.target as i8 - move_.start as i8).abs() == 16 {
                // double push
                self.position_hash ^=
                    zobrist_numbers()[zobrist_en_passent_index(self.en_passent_mask)];
                let en_passent_mask = 1 << (move_.target - 8) << (16 * color as u8);
                // only update the en passent mask if a capture can happen next turn, otherwise clear it
                if en_passent_mask & self.pawn_attacks(!color) != 0 {
                    self.en_passent_mask = en_passent_mask;
                    self.position_hash ^=
                        zobrist_numbers()[zobrist_en_passent_index(self.en_passent_mask)];
                } else {
                    // a capture can't happen with the new mask, clear
                    self.en_passent_mask = 0;
                    self.position_hash ^= zobrist_numbers()[zobrist_en_passent_index(0)];
                }
            } else {
                // single push/capture
                self.position_hash ^=
                    zobrist_numbers()[zobrist_en_passent_index(self.en_passent_mask)];
                self.en_passent_mask = 0;
            }

            // promotion
            if let Some(target_piece) = move_.promotion {
                unmove.promotion = true;
                self.piece_masks[Pawn] ^= 1 << move_.target;
                self.piece_masks[target_piece] |= 1 << move_.target;
                self.position_hash ^=
                    zobrist_numbers()[zobrist_piece_index(Pawn, color, move_.start as usize)];
                piece = target_piece;
            }
        } else {
            // moving other pieces clears en passent state
            self.position_hash ^= zobrist_numbers()[zobrist_en_passent_index(self.en_passent_mask)];
            self.en_passent_mask = 0;
        }

        // update piece list
        self.piece_list[move_.start as usize] = None;
        self.piece_list[move_.target as usize] = Some((piece, color));

        // update zobrist hash with moved piece
        if let None = move_.promotion {
            // promotions updated the zobrist hash earlier because the piece type changes
            self.position_hash ^=
                zobrist_numbers()[zobrist_piece_index(piece, color, move_.start as usize)];
        }
        self.position_hash ^=
            zobrist_numbers()[zobrist_piece_index(piece, color, move_.target as usize)];

        // switch current player
        self.current_player = !self.current_player;
        self.position_hash ^= zobrist_numbers()[zobrist_player_index()];

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
            if unmove.target % 8 == 6 {
                self.piece_list[unmove.target as usize - 1] = None;
                self.piece_list[unmove.target as usize + 1] = Some((Rook, color));
                let mask = (1 << (unmove.target - 1)) | (1 << (unmove.target + 1));
                self.piece_masks[Rook] ^= mask;
                self.color_masks[color] ^= mask;
            // queenside
            } else {
                self.piece_list[unmove.target as usize + 1] = None;
                self.piece_list[unmove.target as usize - 2] = Some((Rook, color));
                let mask = (1 << (unmove.target - 2)) | (1 << (unmove.target + 1));
                self.piece_masks[Rook] ^= mask;
                self.color_masks[color] ^= mask;
            }
        }

        // undo promotion
        if unmove.promotion {
            self.piece_masks[piece] ^= 1 << unmove.target;
            // color gets updated later when the pawn gets moved back
            // self.color_masks[color] ^= 1 << unmove.target;

            piece = Pawn;
            // place the pawn back on the promotion square, will be moved later
            self.piece_masks[piece] |= 1 << unmove.target;
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

        // replace pawn taken en passent and clear the target square
        if unmove.en_passent {
            let shift = unmove.target as usize - 8 + (16 * color as usize);
            self.piece_masks[Pawn] |= 1 << shift;
            self.color_masks[!color] |= 1 << shift;
            self.piece_list[shift] = Some((Pawn, !color));

            // the target square for an en passent capture is empty
            self.piece_list[unmove.target as usize] = None;

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

        // reset zobrist hash
        self.position_hash = self.position_history.pop().unwrap();
    }
}
