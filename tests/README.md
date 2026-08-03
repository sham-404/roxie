# 🧪 Roxie Testing & Match Framework

This folder contains scripts and outputs used to evaluate different versions of the Roxie chess engine.

---

## 📂 Structure

* `match.py`
  Python script to run automated matches using `cutechess-cli`

* `results/` *(gitignored)*
  Stores JSON summaries of match results

* `pgns/` *(gitignored)*
  Stores PGN files of played games

---

## ⚙️ Requirements

* `cutechess-cli` installed and available in PATH
* UCI-compatible engine binaries (e.g., `roxie_v0_2`, `roxie_v0_3`)

---

## ▶️ Usage

```bash
python match.py <engine1> <engine2> [options]
```

---

## 🧾 Arguments

* `engine1`
  Path to first engine binary

* `engine2`
  Path to second engine binary

---

## ⚙️ Options

* `-n, --games N`
  Number of games to play
  Default: `20`

* `--depth N`
  Fixed search depth for both engines
  Default: `1`
  ⚠️ Recommended for early versions (no time management)

* `--tc TIME`
  Time control in seconds + increment
  Example: `1+0.1`
  Used only if `--depth` is not specified

* `-o, --outdir DIR`
  Output directory for results
  Default: `tests/results`

* `--epd PATH_TO_EPD_FILE`
  Uses provided .epd file to test the engine from a position

* `--fen FEN_STRING`
  Uses the given fen string as start position and match the engines

---

## 🧠 Behavior Notes

* If `--depth` is provided → engines run with fixed depth (`tc=inf depth=N`)
* If `--depth` is NOT provided → time control (`--tc`) is used
* Engine names are automatically derived from binary filenames

---

## 🧪 Examples

```bash
# Compare two versions
python match.py ./roxie_v0_2 ./roxie_v0_3 -n 100 --depth 1

# Use time control 
python match.py ./roxie_v0_3 ./roxie_v0_4 -n 50 --tc 1+0.1

# Custom output directory
python match.py ./roxie_v0_2 ./roxie_v0_3 -n 100 --depth 1 -o experiments/run1

# Test positions from an EPD file
python match.py ./roxie stockfish -n 1 --epd tests/position.epd --tc 300+3

# Test a single FEN position directly
python match.py ./roxie stockfish -n 1 --fen "8/8/8/4k3/8/2B5/3N4/4K3 w - - 0 1" --tc 300+3
```
---

## 📊 Output

Each run generates:

* PGN file → full game records
* JSON file → summarized results

Example JSON:

```json
{
    "engine1": "roxie_v0_2",
    "engine2": "roxie_v0_3",
    "games": 100,
    "result": {
        "engine1_wins": 40,
        "engine1_losses": 45,
        "draws": 15
    },
    "score_percent": 0.475
}
```

---

## 🎯 Purpose

This framework is used to:

* Compare engine versions over time
* Track strength improvements
* Detect regressions

---
