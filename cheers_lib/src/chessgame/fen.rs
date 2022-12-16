use cheers_bitboards::BitBoard;

use super::ChessGame;
use crate::{
    moves::coord,
    types::{CastlingIndex, CastlingRights, ColorIndex, ColorMasks, PieceIndex, PieceMasks},
};
use CastlingIndex::*;
use ColorIndex::*;
use PieceIndex::*;

impl ChessGame {
    pub fn set_from_fen<T:AsRef<str>>(
        &mut self,
        fen: T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        *self = Self {
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

        self.piece_masks = PieceMasks([[BitBoard::empty(); 6]; 2]);
        self.color_masks = ColorMasks([BitBoard::empty(); 2]);

        let fen = fen.as_ref().to_string();
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

        self.pawn_hash = self.zobrist_pawn_hash();

        Ok(())
    }

    pub fn fen(&self) -> String {
        let mut fen = String::new();
        // get pieces by square
        for rank in (0..8).rev() {
            let mut empty_counter = 0;
            for file in 0..8 {
                let square = 8u8 * rank + file;
                let piece = self.piece_at(square.into());

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
                        if self.color_at(square.into()) == White {
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
        match self.en_passent_square() {
            None => fen.push('-'),
            Some(sq) => fen.push_str(&coord(sq)),
        }
        fen.push(' ');

        // halfmove clock
        fen.push_str(&self.halfmove_clock.to_string());
        fen.push(' ');

        // fullmove number
        fen.push_str(&(self.position_history.len() / 2).to_string());

        fen
    }
}
