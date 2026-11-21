#!/usr/bin/env python3
"""Task runner (replaces the Makefile; pyCrust keeps tracked source to Python, C and Rust).

Usage: python3 tasks.py <task> [--py PATH] [--models M1 M2 ...]
Tasks: setup test audit sim probe report reproduce quick demo rust parity
       rsim behaviour analyze reanalyze metrics   (v0.2 Rust harness)
"""
import argparse
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
MODELS = ["Qwen/Qwen2.5-0.5B", "Qwen/Qwen2.5-1.5B"]


def run(cmd, log=None, cwd=ROOT):
    print("+", " ".join(cmd), flush=True)
    if log is None:
        subprocess.run(cmd, cwd=cwd, check=True)
        return
    (ROOT / "logs").mkdir(exist_ok=True)
    with open(ROOT / "logs" / log, "w") as f:
        p = subprocess.Popen(
            cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
        )
        for line in p.stdout:  # tee to screen and log
            sys.stdout.write(line)
            f.write(line)
        if p.wait():
            sys.exit(p.returncode)


def main():
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("task")
    ap.add_argument("--py", default=str(ROOT / ".venv" / "bin" / "python"))
    ap.add_argument("--models", nargs="+", default=MODELS)
    a = ap.parse_args()
    py = a.py if os.path.exists(a.py) else sys.executable
    t = a.task
    if t == "setup":  # venv (needs an arm64/x86_64 Python >= 3.10 with torch wheels)
        run([sys.executable, "-m", "venv", ".venv"])
        run(
            [
                str(ROOT / ".venv" / "bin" / "python"),
                "-m",
                "pip",
                "install",
                "-r",
                "requirements.txt",
            ]
        )
    elif t == "test":  # formula + determinism unit tests
        run([py, "tests/test_engine.py"])
    elif t == "audit":  # arithmetic audit of the source deck's worked examples
        run([py, "scripts/audit_deck.py"])
    elif t == "sim":  # Monte Carlo study, 100k trials/class
        run([py, "scripts/run_montecarlo.py", "--n", "100000"], log="montecarlo.log")
    elif t == "probe":  # real-model probes (downloads models on first run)
        for m in a.models:
            run(
                [py, "scripts/run_probe.py", "--model", m],
                log=f"probe_{m.split('/')[-1].lower()}.log",
            )
    elif t == "report":  # regenerate results/HEADLINES.md from JSON
        run([py, "scripts/make_report.py"])
    elif t == "reproduce":  # everything in the README
        for step in ("test", "audit", "sim", "probe", "report"):
            run([sys.executable, __file__, step, "--py", py, "--models", *a.models])
        print("done: see results/ and results/figures/")
    elif t == "quick":  # smoke test (small sim + small probe on 0.5B)
        run([py, "scripts/run_montecarlo.py", "--quick"])
        run(
            [
                py,
                "scripts/run_probe.py",
                "--model",
                "Qwen/Qwen2.5-0.5B",
                "--quick",
                "--n_random",
                "1",
            ]
        )
    elif t == "demo":  # live surface-vs-internal demo
        run([py, "scripts/demo_live.py"])
    elif t == "parity":  # regenerate the Rust parity fixture from the Python engine
        run([py, "scripts/export_parity.py"])
    elif t == "rust":  # Rust port: build + parity tests
        run(["cargo", "test", "--release"], cwd=ROOT / "rust")
    elif t == "rsim":  # v0.2 Study S: Rust Monte Carlo, 20 seeds, bootstrap CIs
        run(["cargo", "run", "--release", "--bin", "montecarlo"], cwd=ROOT / "rust")
    elif t == "behaviour":  # v0.2 Study B: local Ollama models (raw data -> ~/.cache/eaw-data)
        run(["cargo", "run", "--release", "--bin", "behaviour", "--", "run", "--models", ",".join(a.models)], cwd=ROOT / "rust")
    elif t == "analyze":  # v0.2 Study B summary + preregistered tests
        run(["cargo", "run", "--release", "--bin", "behaviour", "--", "analyze"], cwd=ROOT / "rust")
    elif t == "reanalyze":  # bootstrap CIs on the v0.1 probe items
        run(["cargo", "run", "--release", "--bin", "reanalyze"], cwd=ROOT / "rust")
    elif t == "metrics":  # metrics.json (runs tests + coverage)
        run(["cargo", "run", "--release", "--bin", "metrics", "--", "--coverage", "--python", py], cwd=ROOT / "rust")
    else:
        ap.error(f"unknown task {t!r}")


if __name__ == "__main__":
    main()
