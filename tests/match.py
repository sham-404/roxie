#!/usr/bin/env python3
import argparse
import subprocess
import datetime
import os
import re
import json

# 📍 Anchor everything to script location
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))


def get_engine_name(path):
    return os.path.basename(path)


def resolve_outdir(outdir):
    # If absolute → use as-is
    if os.path.isabs(outdir):
        return outdir
    # If relative → anchor to script directory
    return os.path.join(SCRIPT_DIR, outdir)


def run_match(engine1, engine2, games, tc, depth, outdir, save):
    ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")

    name1 = os.path.basename(engine1)
    name2 = os.path.basename(engine2)

    base_name = f"{ts}_{name1}_vs_{name2}"

    if save:
        results_dir = resolve_outdir(outdir)
        pgns_dir = resolve_outdir("pgns")

        os.makedirs(results_dir, exist_ok=True)
        os.makedirs(pgns_dir, exist_ok=True)

    json_file = None
    pgn_file = None

    cmd = [
        "cutechess-cli",
        "-engine",
        f"cmd={engine1}",
        f"name={name1}",
        "-engine",
        f"cmd={engine2}",
        f"name={name2}",
        "-each",
        "proto=uci",
    ]

    if depth is not None:
        cmd += ["tc=inf", f"depth={depth}"]
    else:
        cmd += [f"tc={tc}"]

    cmd += [
        "-games",
        str(games),
        "-repeat",
    ]

    if save:
        cmd += ["-pgnout", pgn_file]

    print("Running:\n", " ".join(cmd), "\n")

    result = subprocess.run(cmd, capture_output=True, text=True)

    print(result.stdout)
    if result.stderr:
        print("STDERR:\n", result.stderr)

    summary = parse_summary(result.stdout)

    if save:
        json_file = os.path.join(results_dir, f"{base_name}.json")
        pgn_file = os.path.join(pgns_dir, f"{base_name}.pgn")

        if summary:
            data = {
                "engine1": name1,
                "engine2": name2,
                "timestamp": ts,  # 🧠 useful metadata
                "games": summary["total"],
                "result": {
                    "engine1_wins": summary["wins"],
                    "engine1_losses": summary["losses"],
                    "draws": summary["draws"],
                },
                "score_percent": summary["score_pct"],
            }

            with open(json_file, "w") as f:
                json.dump(data, f, indent=4)

            print(f"\nJSON saved to: {json_file}")

    return pgn_file, json_file


def parse_summary(output):
    matches = re.findall(r"Score of .*: (\d+) - (\d+) - (\d+)", output)

    if not matches:
        return None

    wins, losses, draws = map(int, matches[-1])  # final score
    total = wins + losses + draws
    score = wins + 0.5 * draws

    return {
        "wins": wins,
        "losses": losses,
        "draws": draws,
        "total": total,
        "score_pct": (score / total) if total else 0.0,
    }


def main():
    ap = argparse.ArgumentParser(description="Run engine matches via cutechess-cli")
    ap.add_argument("engine1", help="path to engine 1")
    ap.add_argument("engine2", help="path to engine 2")
    ap.add_argument("-n", "--games", type=int, default=20)
    ap.add_argument("--tc", default="0.1+0.01")
    ap.add_argument("--depth", type=int, default=1)
    ap.add_argument("-o", "--outdir", default="results")  # 🔥 cleaner default
    ap.add_argument("--save", action="store_true", help="store PGN and JSON results")

    args = ap.parse_args()

    run_match(
        args.engine1,
        args.engine2,
        args.games,
        args.tc,
        args.depth,
        args.outdir,
        args.save,
    )


if __name__ == "__main__":
    main()
