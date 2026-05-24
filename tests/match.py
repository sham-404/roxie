#!/usr/bin/env python3

import argparse
import subprocess
import datetime
import os
import re
import json

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))


def get_engine_name(path):
    return os.path.basename(path)


def resolve_outdir(outdir):
    if os.path.isabs(outdir):
        return outdir
    return os.path.join(SCRIPT_DIR, outdir)


def append_engine_options(cmd, opts):
    for opt in opts:
        cmd.append(f"option.{opt}")


def run_match(
    engine1,
    engine2,
    games,
    tc,
    depth,
    outdir,
    save,
    concurrency,
    sprt,
    engine1_opts,
    engine2_opts,
):
    ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")

    name1 = get_engine_name(engine1)
    name2 = get_engine_name(engine2)

    base_name = f"{ts}_{name1}_vs_{name2}"

    e1_path = os.path.abspath(engine1)
    e2_path = os.path.abspath(engine2)

    pgn_path = os.path.abspath(os.path.join(SCRIPT_DIR, "chess_moves.pgn"))

    pgns_dir = resolve_outdir("pgns")
    os.makedirs(pgns_dir, exist_ok=True)

    pgn_file = os.path.join(pgns_dir, f"{base_name}.pgn")

    cmd = [
        "cutechess-cli",
        "-engine",
        f"cmd={e1_path}",
        f"name={name1}",
        "-engine",
        f"cmd={e2_path}",
        f"name={name2}",
        "-each",
        "proto=uci",
    ]

    append_engine_options(cmd, engine1_opts)
    append_engine_options(cmd, engine2_opts)

    if depth is not None:
        cmd += ["tc=inf", f"depth={depth}"]
        mode = "depth"
        mode_value = depth
    else:
        cmd += [f"tc={tc}"]
        mode = "time"
        mode_value = tc

    cmd += [
        "-openings",
        f"file={pgn_path}",
        "format=pgn",
        "order=random",
        "plies=10",
        "-games",
        str(games),
        "-repeat",
        "-concurrency",
        str(concurrency),
    ]

    if sprt:
        cmd += [
            "-sprt",
            "elo0=0",
            "elo1=10",
            "alpha=0.05",
            "beta=0.05",
        ]

    if save:
        cmd += ["-pgnout", pgn_file]

    print("\nRunning:\n")
    print(" ".join(cmd))
    print()

    full_output = []

    process = subprocess.Popen(
        cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1
    )

    for line in process.stdout:
        print(line, end="", flush=True)
        full_output.append(line)

    process.wait()

    output_text = "".join(full_output)

    summary = parse_summary(output_text)
    elo_data = parse_elo(output_text)

    if save and summary:
        results_dir = resolve_outdir(outdir)
        os.makedirs(results_dir, exist_ok=True)

        json_file = os.path.join(results_dir, f"{base_name}.json")

        data = {
            "engine1": name1,
            "engine2": name2,
            "timestamp": ts,
            "engine1_options": engine1_opts,
            "engine2_options": engine2_opts,
            "sprt_enabled": sprt,
            "full_command": cmd,
            "games": summary["total"],
            "mode": mode,
            "mode_value": mode_value,
            "concurrency": concurrency,
            "opening_file": pgn_path,
            "opening_plies": 10,
            "pgn_file": pgn_file,
            "result": {
                "engine1_wins": summary["wins"],
                "engine1_losses": summary["losses"],
                "draws": summary["draws"],
            },
            "score_percent": summary["score_pct"],
            "elo": elo_data,
        }

        with open(json_file, "w") as f:
            json.dump(data, f, indent=4)

        print(f"\nJSON saved to: {json_file}")
        print(f"PGN saved to: {pgn_file}")

    return


def parse_summary(output):
    matches = re.findall(r"Score of .*: (\d+) - (\d+) - (\d+)", output)

    if not matches:
        return None

    wins, losses, draws = map(int, matches[-1])

    total = wins + losses + draws
    score = wins + 0.5 * draws

    return {
        "wins": wins,
        "losses": losses,
        "draws": draws,
        "total": total,
        "score_pct": score / total if total else 0.0,
    }


def parse_elo(output):
    match = re.search(
        r"Elo difference: ([+-]?\d+(?:\.\d+)?) \+/- (\d+(?:\.\d+)?), LOS: ([\d\.]+)",
        output,
    )

    if not match:
        return None

    return {
        "elo_diff": float(match.group(1)),
        "confidence_interval": float(match.group(2)),
        "los": float(match.group(3)),
    }


def main():
    ap = argparse.ArgumentParser()

    ap.add_argument("engine1")
    ap.add_argument("engine2")

    ap.add_argument("-n", "--games", type=int, default=200)

    ap.add_argument("--tc", default="0.1+0.01")

    ap.add_argument("--depth", type=int)

    ap.add_argument("-o", "--outdir", default="results")

    ap.add_argument("--save", action="store_true")

    ap.add_argument("-c", "--concurrency", type=int, default=1)

    ap.add_argument("--sprt", action="store_true")

    ap.add_argument("--engine1-opt", action="append", default=[])

    ap.add_argument("--engine2-opt", action="append", default=[])

    args = ap.parse_args()

    run_match(
        args.engine1,
        args.engine2,
        args.games,
        args.tc,
        args.depth,
        args.outdir,
        args.save,
        args.concurrency,
        args.sprt,
        args.engine1_opt,
        args.engine2_opt,
    )


if __name__ == "__main__":
    main()
