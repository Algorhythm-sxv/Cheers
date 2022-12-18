# Cheers - a UCI chess engine written in Rust

Cheers is not a complete chess program and should be used in conjunction with a UCI frontend e.g. [Cute Chess](https://cutechess.com/).

Notably Cheers is still single-threaded, with plans to enable multithreading with Lazy SMP in the near future.

## Options
Currently supported options are:
- **Hash**: Transposition table size in megabytes. Default: 32, Minimum: 1, Maximum: 32768 (arbitrarily, this is likely to change). Only power-of-two values are supported, so any other values will be rounded up to the nearest power of two.

## Features (in no particular order)

### General
- Magic Bitboard board representation
- Legal move generation

### Search
- Iterative Deeping
- Transposition Table
- Aspiration Windows
- Principal Variation Alpha-Beta search
- Null Move Pruning
- Late Move Reduction
- Late Move Pruning
- Futility Pruning
- Reverse Futility Pruning
- Internal Iterative Reduction
- Static Exchange Evaluation Pruning
- Mate Distance Pruning

### Quiescence Search
- Transposition Table
- Delta Pruning
- Static Exchange Evaluation Pruning

### Move Ordering
- Hash move from transposition table
- Static Exchange Evaluation on captures
- Queen and Rook promotions
- Killer Move Heuristic
- Counter Move Heuristic
- History Heuristic

## Evaluation
Cheers currently uses a hand-crafted evaluation function with Texel-tuned parameters. NNUE is planned for the future.
### General
- Tapered Evaluation
- Pawn Hash Table (8MB)

### Pawns
- Material
- Piece-Square Tables
- Doubled Pawn
- Isolated Pawn
- Connected Pawn
- Passed Pawn
- Passed Pawn Rank
- Passed Pawn Blocked
- Passed Pawn Connected
- Passed Pawn Supported by Rook
- Passed Pawn Uncatchable by enemy King

### Knights
- Material
- Piece-Square Tables
- Mobility
- Knight behind Pawn
- Knight distance from friendly King
- Knight on (defended) outpost

### Bishops
- Material
- Piece-Square Tables
- Mobility
- Bishop behind Pawn
- Bishop distance from friendly King
- Bishop on (defended) outpost
- Bishop Pair
- Bishop on long diagonal

### Rooks
- Material
- Piece-Square Tables
- Mobility
- Rook on (semi) open file
- Rook on seventh rank
- Rook trapped by friendly King

### Queens
- Material
- Piece-Square Tables
- Mobility
- Risk of discovered attack

### Kings
- Piece-Square Tables
- Mobility
- Minor piece defenders
- King on open file
