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

---

## 🧠 Behavior Notes

* If `--depth` is provided → engines run with fixed depth (`tc=inf depth=N`)
* If `--depth` is NOT provided → time control (`--tc`) is used
* Engine names are automatically derived from binary filenames

---

## 🧪 Examples

```bash
# Compare two versions (recommended)
python match.py ./roxie_v0_2 ./roxie_v0_3 -n 100 --depth 1

# Use time control (after search is implemented)
python match.py ./roxie_v0_3 ./roxie_v0_4 -n 50 --tc 1+0.1

# Roxie vs Stockfish benchmark
python match.py ./roxie stockfish -n 20 --depth 1

# Custom output directory
python match.py ./roxie_v0_2 ./roxie_v0_3 -n 100 --depth 1 -o experiments/run1
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

## 🚀 Future Improvements

* Elo calculation from match results
* Strength progression graphs 📈
* Opening books for better position variety
* Parallel match execution
