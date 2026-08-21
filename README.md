# ♜ Roxie

<p align="center">
A modern UCI chess engine written from scratch in <b>Rust</b>.<br>
Currently rated at <b>2682 Elo (CCRL)</b>.
</p>

---

## About

**Roxie** is a hobby chess engine written entirely from scratch in **Rust**, focusing on modern search techniques, efficient evaluation, and clean engineering.

The project began as a way to learn how chess engines work. Over time, it evolved into a serious long-term project through continuous experimentation, profiling, tuning, and thousands of self-play and engine-vs-engine games.

Today, Roxie has achieved an official **CCRL rating of 2682 Elo** for version 0.6.3 and continues to improve with every release.

---

## Highlights

* ♟️ UCI compatible
* ⚡ Written entirely in Rust
* 🧠 Quantized HalfKP NNUE evaluation
* 🚀 Magic Bitboard move generation
* 🔍 Modern alpha-beta search with advanced pruning and move ordering
* 📈 Continuously tuned through thousands of self-play and engine matches
* 💻 Optimized builds for Native, AVX2 and AVX-512 processors

---

## Performance

Roxie is continuously tested against established engines using automated tournaments and self-play to measure playing strength and evaluate new ideas.

One memorable game is shown below, where Roxie managed to defeat **Lambergar (~3500 Elo)** in a **10+0.1** rapid game.

<p align="center">
    <img src="assets/roxie_vs_lambergar.png" width="500">
</p>

The full game is available in [`assets/roxie_vs_lambergar.pgn`](assets/roxie_vs_lambergar.pgn).

---

## Features

### ♟️ Search

Roxie uses a modern alpha-beta search framework designed to search deeper while minimizing unnecessary work.

* Principal Variation Search (PVS)
* Iterative Deepening
* Aspiration Windows
* Quiescence Search
* Transposition Tables
* Advanced pruning and reduction techniques
* Efficient move ordering heuristics

### 🧠 Evaluation

* Quantized **HalfKP NNUE** evaluation
* Efficient neural inference for positional evaluation
* Classical evaluation terms blended with neural evaluation

### ⚡ Move Generation

* Magic Bitboards
* Optimized make/unmake move implementation
* Zobrist hashing for transposition and repetition detection

### 🚀 Performance

* Native CPU optimizations
* AVX2 and AVX-512 optimized builds
* Profile Guided Optimization (PGO) support

---

## Building

### Requirements

* Rust (stable toolchain)
* Cargo
* GNU Make

Clone the repository:

```bash
git clone https://github.com/sham-404/roxie.git
cd roxie
```

Build Roxie using one of the following targets:

```bash
# Optimized for your current CPU
make native

# Optimized for AVX2 + BMI2 + POPCNT capable processors
make avx2

# Optimized for AVX-512 capable processors
make avx512

# Build using Profile-Guided Optimization
make pgo-avx512
```

To remove build artifacts:

```bash
make clean
```

---

## Usage

After building, the engine executable will be available in the project directory (the filename depends on the selected build target).

```text
./roxie*
```

### Running from the Terminal

Roxie implements the **Universal Chess Interface (UCI)** protocol.

```bash
./roxie*
```

Example session:

```text
uci
isready
ucinewgame
position startpos
go depth 12
```

Or search a custom position:

```text
position startpos moves e2e4 e7e5 g1f3
go movetime 5000
```

### Using a Chess GUI

Roxie works with any UCI-compatible GUI, including:

* Arena
* Cute Chess
* Banksia GUI
* Nibbler

Simply add the compiled executable as a new UCI engine.

### Engine Matches

Roxie can also be used with tournament managers such as **cutechess-cli** for engine matches and automated self-play testing.

---

## Roadmap

Current focus:

* Improve NNUE playing strength using larger and higher-quality training datasets
* Continue search tuning and evaluation improvements
* Stronger endgame play
* Better time management
* Multi-threaded search (SMP)

Long-term goal:

> Reach **3500 Elo** while remaining a clean, educational, and highly optimized open-source chess engine.

---

## Acknowledgements

Roxie was developed independently from scratch, but several resources and projects encouraged and inspired me throughout its development.

Special thanks to:

* **[Sebastian Lague](https://www.youtube.com/@SebastianLague)** for the *Coding Adventure: Chess* series, which inspired me to begin writing my own chess engine.
* The **[Chess Programming Wiki](https://www.chessprogramming.org/)** for providing an excellent introduction to many chess programming concepts and serving as a handy reference during development.
* The many authors of open-source chess engines whose work and discussions continue to advance the chess programming community.
* **[Lambergar](https://github.com/jabolcni/Lambergar)**, whose impressive playing strength and steady progress motivated me to keep pushing Roxie further whenever I felt like stopping.

---

## Development

Roxie is actively developed, with every significant improvement documented through version history, testing, and benchmarking.

For additional information, see:

* **CHANGELOG.md** for the complete development history.
* **docs/nnue.md** for details about Roxie's NNUE architecture and implementation.

---

## License

This project is licensed under the **MIT License**.

See the [LICENSE](LICENSE) file for details.
