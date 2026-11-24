#!/usr/bin/env python3
"""Keep the README number table equal to results/metrics.json.

  python3 scripts/readme_numbers.py --write   rewrite the block between the markers
  python3 scripts/readme_numbers.py --check   exit 1 if the README block differs from the JSON
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
START, END = "<!-- numbers:start -->", "<!-- numbers:end -->"


def pct(x, d=1):
    return f"{100 * x:.{d}f}%"


def block(m):
    v = m["metrics"]
    rows = [
        ("Real model runs (MEASURED)", f"{v['real_runs']['value']:,}", v["real_runs"]["command"]),
        ("Simulation trials (SIMULATION)", f"{v['sim_trials']['value']:,}", v["sim_trials"]["command"]),
        (
            "False-pass under pressure (MEASURED, pooled)",
            f"{pct(v['false_pass_real']['value'])} [{pct(v['false_pass_real']['ci'][0])}, {pct(v['false_pass_real']['ci'][1])}]",
            v["false_pass_real"]["command"],
        ),
        (
            "False-pass of the scoring engine (SIMULATION)",
            f"{pct(v['false_pass_sim']['value'])} [{pct(v['false_pass_sim']['ci'][0])}, {pct(v['false_pass_sim']['ci'][1])}]",
            v["false_pass_sim"]["command"],
        ),
        ("Models", f"{v['models']['value']}", v["models"]["command"]),
        ("Tests (all passing)", f"{v['tests']['value']}", v["tests"]["command"]),
        ("Rust line coverage", f"{v['coverage']['value']:.1f}%", v["coverage"]["command"]),
    ]
    out = [
        START,
        "| Number | Value | Command |",
        "|---|---|---|",
    ]
    out += [f"| {a} | {b} | `{c}` |" for a, b, c in rows]
    out += [
        "",
        f"Generated {m['generated_at']} at commit `{m['git_sha'][:7]}` on {m['machine']}. Source: `results/metrics.json`.",
        END,
    ]
    return "\n".join(out)


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "--check"
    m = json.loads((ROOT / "results" / "metrics.json").read_text())
    readme = (ROOT / "README.md").read_text()
    if START not in readme or END not in readme:
        sys.exit("README.md has no numbers markers")
    a, rest = readme.split(START, 1)
    _, z = rest.split(END, 1)
    new = a + block(m) + z
    if mode == "--write":
        (ROOT / "README.md").write_text(new)
        print("README numbers written")
    elif new != readme:
        sys.exit("README numbers differ from results/metrics.json (run: make metrics)")
    else:
        print("README numbers match results/metrics.json")


if __name__ == "__main__":
    main()
