# Cheers - a UCI chess engine written in Rust

Cheers is not a complete chess program and should be used in conjunction with a UCI frontend e.g. [Cute Chess](https://cutechess.com/).

## Options
Currently supported options are:
- **Hash**: 
  `Default: 32, Minimum: 1, Maximum: 32768`\
  Transposition table size in megabytes.
- **Threads**
  `Default: 1, Minimum: 1, Maximum: 256`\
  The number of thread to search with.
- **NmpDepth**:
  `Default: 1, Minimum: 1, Maximum: 10`\
  The depth above which Null Move Pruning is applied.
- **NmpConstReduction**:
  `Default: 3, Minimum: 1, Maximum: 10`\
  The constant reduction to apply to all Null Move Pruning.
- **NmpLinearDivisor**:
  `Default: 3, Minimum: 1, Maximum: 10`\
  The divisor applied to the depth in Null Move Pruning. The general formula is `reduction = NmpConstReduction + (depth / NmpLinearDivisor)`.
- **SeePruningDepth**:
  `Default: 6, Minimum: 1, Maximum: 10`\
  The depth below which SEE Pruning is applied.
- **SeeCaptureMargin**:
  `Default: -30, Minimum: -200, Maximum: 200`\
  The margin around `beta` where SEE Pruning is accepted for captures.
- **SeeQuietMargin**:
  `Default: -98, Minimum: -100, Maximum: -100`\
  The margin around `beta` where SEE Pruning is accepted for quiet moves.
- **PvsFullDepth**:
  `Default: 2, Minimum: 1, Maximum: 5`\
  The depth at which all moves are searched without reductions.
- **DeltaPruningMargin**:
  `Default: 91, Miniimum: 0, Maximum: 300`\
  The margin around `beta` where Delta Pruning is accepted.
- **FpMargin1**:
  `Default: 162, Minimum: 0, Maximum: 300`\
  The margin around `alpha` where Futility Pruning is accepted at depth 1.
- **FpMargin2**:
  `Default: 320, Minimum: 0, Maximum: 700`\
  The margin around `alpha` where Futility Pruning is accepted at depth 2.
- **FpMargin3**:
  `Default: 706, Minimum: 500, Maximum: 1000`\
  The margin around `alpha` where Futility Pruning is accepted at depth 3.
- **RfpMargin**:
  `Default: 140, Minimum: 0, Maximum: 300`\
  The margin around `beta` where Reverse Futility Pruning is accepted.
- **LmpDepth**:
  `Default: 2, Minimum: 0, Maximum: 10`\
  The depth at or below which Late Move Pruning is applied 
- **LmpMargin**:
  `Default: 6, Minimum: 1, Maximum: 15`\
  The quadratic coefficient with depth. After `LmpMargin * depth*depth` moves pruning can be applied.
- **IirDepth**:
  `Default: 6, Minimum: 2, Maximum: 10`\
  The depth above which Internal Iterative Reduction is applied.
## Features (in no particular order)

### General
- Magic Bitboard board representation
- Legal move generation

### Search
- Multithreading with Lazy SMP
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
- MVV-LVA ordering on captures
- Queen promotions
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
