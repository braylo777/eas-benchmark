#!/usr/bin/env python3
"""Compare regenerated parity fixtures with the committed ones. Floats match within rel 1e-9 (libm differs by platform)."""
import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FX = ["rust/tests/fixtures/parity.json", "rust/tests/fixtures/v02.json"]


def same(a, b):
    if isinstance(a, float) or isinstance(b, float):
        return math.isclose(a, b, rel_tol=1e-9, abs_tol=1e-12)
    if isinstance(a, dict):
        return a.keys() == b.keys() and all(same(a[k], b[k]) for k in a)
    if isinstance(a, list):
        return len(a) == len(b) and all(same(x, y) for x, y in zip(a, b))
    return a == b


old = {f: json.loads((ROOT / f).read_text()) for f in FX}
subprocess.run([sys.executable, str(ROOT / "scripts/export_parity.py")], check=True)
bad = [f for f in FX if not same(old[f], json.loads((ROOT / f).read_text()))]
sys.exit(f"fixtures out of date: {bad}" if bad else 0)
