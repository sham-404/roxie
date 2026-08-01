# Changelog

All notable changes to Roxie are documented in this file.

---

### unreleased

### Added 
- gen_quiet_moves() and gen_tactical_moves() were added to help the search become faster
- Added MovePicker, replacing the total move generation and move ordering at once
- optimized move generation functions, and introduced helpers to get the moves of a single piece 
- Added EvalHistory for lmp, and enabled lmp (Late Move Pruning) after a long time
- Added Singular Extension

### Improved
- Reduced the if branching for quiets and captures for performance, but not so significant tho.
- Improved castling right updation in make_move()

### Fixed
- see() is fixed with subtle, non significant issue
- quiescence() is fixed, as it lacked 3 fold and 50 move check before

---

## v0.6.3

### Added

- Added gives_check() and is_legal_mv(), which eliminating need of full make and unmake board mutation
- Embedded the .nnue file directly into the binary
- Added Continuation History
- setoption is now available (for Hash and Clear Hash)

### Improved
- Micro optimized move ordering and changed quiet scoring a bit
- Used magic bitboards for faster attack findings in is_square_attacked()

### Fixed
- Fixed bug in gives_check()
- Fixed prev_move updation error in qsearch

---

## v0.6.2

### Added

- Countermove heuristic
- Transposition table aging
- Quantized `.nnue` network loading
- SEE pruning
- Improved quiescence search
- Mate and node-based UCI search limits
- Better Makefile with multiple optimized build targets
- README, CHANGELOG, LICENCE, and a nnue.md at docs

### Improved

- Better Late Move Reduction logic
- Improved transposition table replacement policy
- Reduced memory usage of TT entries
- Improved principal variation generation
- Cleaner move ordering
- Minor search speed improvements

### Fixed

- Promotion move ordering
- Aspiration window overflow
- Various search and TT bugs

---

## v0.6.1

### Added

- Root Principal Variation Search
- Packed bucket-based transposition table
- Search statistics output
- Extension control
- LMR reduction tables

### Improved

- Null Move Pruning conditions
- Principal variation handling
- Transposition table efficiency
- NNUE integration and code organization

### Fixed

- Mate scoring in quiescence search
- Root PV orientation bug

---

## v0.6.0

### Added

- Quantized HalfKP NNUE evaluation
- Incremental NNUE accumulator
- Neural network inference integrated into search
- Binary NNUE loader

### Improved

- NNUE quantization
- Network architecture
- Evaluation performance

---

## v0.5.3

### Added

- Check extensions
- History heuristic
- Killer move heuristic

### Improved

- Move ordering
- History scoring
- Search organization

---

## v0.5.2

### Added

- King safety evaluation
- Tempo bonus
- Bishop pair bonus

### Improved

- Incremental PeSTO evaluation
- Pawn structure evaluation
- Mobility evaluation performance
- Endgame detection

---

## v0.5.1

### Added

- Mobility evaluation

### Improved

- Knight, bishop and rook mobility scoring
- Evaluation tuning

---

## v0.5.0

### Added

- Aspiration windows
- SEE move ordering

### Improved

- Search stability
- Move ordering

### Notes

- Approximately **+123 Elo** over v0.4.1 (100% LOS)

---

## v0.4.1

### Added

- Principal Variation Search
- Improved LMR integration

### Improved

- Search efficiency
- Magic bitboard initialization

### Notes

- Approximately **+10 Elo**

---

## v0.4.0

### Added

- Futility Pruning
- Reverse Futility Pruning

---

## v0.3.1

### Improved

- Capture move generation
- Move scoring logic

### Fixed

- Promotion scoring bugs

### Notes

- Approximately **+34 Elo**

---

## v0.3.0

### Added

- Magic Bitboard move generation

### Improved

- Sliding piece move generation
- Overall move generation performance

### Notes

- Around **15% faster move generation**

---

## v0.2.6

### Added

- Dedicated UCI module
- Search thread
- Stop command support

### Improved

- Quiescence search
- Static Exchange Evaluation

---

## v0.2.5

### Added

- Quiescence search

### Fixed

- Null move returned when search stopped early

---

## v0.2.4

### Added

- Time management
- Complete `go` command support

### Improved

- Search architecture

---

## v0.2.3

### Added

- Transposition Table
- Null Move Pruning
- Late Move Reductions

### Improved

- Search speed
- Search stability

---

## v0.2.2

### Improved

- Faster Zobrist updates

### Added

- Fifty-move rule tracking

---

## v0.2.1

### Fixed

- PeSTO mirror evaluation bug

---

## v0.2.0

### Added

- PeSTO evaluation
- MVV-LVA move ordering

---

## v0.1.3

### Added

- Alpha-beta pruning
- Better testing and benchmarking

### Improved

- Search performance

---

## v0.1.2

### Added

- Zobrist hashing
- Threefold repetition detection
- Basic negamax search

---

## v0.1.1

### Added

- FEN parsing
- Legal move generation
- Castling
- En passant
- Promotions
- Undo moves
- Attack detection
- Mailbox/bitboard hybrid board representation
- Interactive debugging tools
- Initial UCI communication

### Improved

- Core board representation
- Move generation

---

## v0.1.0

Initial release.

- Bitboard board representation
- Basic move generation
- Foundation of the engine
