"""Write rust/tests/fixtures/parity.json: engine inputs + Python outputs for the Rust port to match."""
import json
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from eas import engine as E  # noqa: E402
from eas.scenarios import generate  # noqa: E402
from eas.stats import wilson  # noqa: E402

rows = []
for covert in (0.0, 0.5):
    d = generate(150, np.random.default_rng(20261106), covert_frac=covert)
    T = E.t_score(d["drift"], d["halluc"], d["misalign"])
    S = E.fib_S(d["arg_n"], d["arg_prev"])
    N = E.debate_rounds(S)
    gate = [E.prime_gate(a, s) for a, s in zip(d["action_id"], d["state"])]
    clear = np.array([g[0] for g in gate])
    V = E.verdict_V(T, S, d["anchor"], clear)
    P = E.verdict_pass(V, clear)
    for i in range(len(T)):
        rows.append({
            "drift": float(d["drift"][i]), "halluc": float(d["halluc"][i]), "misalign": float(d["misalign"][i]),
            "anchor": float(d["anchor"][i]), "arg_n": float(d["arg_n"][i]), "arg_prev": float(d["arg_prev"][i]),
            "action_id": d["action_id"][i], "state": d["state"][i], "unsafe": bool(d["unsafe"][i]),
            "T": float(T[i]), "S": float(S[i]), "N": int(N[i]), "clear": int(clear[i]),
            "veto": gate[i][1], "V": float(V[i]), "pass": bool(P[i]),
        })
A = []
for h, kw in [
    (dict(w=[1.0, 1.0], T=[0.5, 1.0], deg=[1, 2], t_last=0.0), dict(t=0.0, P_ij=1.0, F=1.0, snn=1.0)),
    (dict(w=[1.0, 1.0], T=[0.5, 1.0], deg=[1, 2], t_last=0.0), dict(t=10.0, P_ij=0, F=0, snn=0, current_trust=2.0)),
    (dict(w=[0.3, 0.9, 0.5], T=[0.8, 0.1, 0.6], deg=[3, 1, 2], t_last=2.0), dict(t=7.5, P_ij=0.4, F=0.7, snn=0.25, current_trust=0.5)),
]:
    A.append({"history": h, **kw, "A": float(E.alignment_A(h, **kw))})
out = {
    "residues": {str(p): E.violation_residue(p) for p in E.PRIMES},
    "rows": rows,
    "alignment": A,
    "snn": [[0.2, 0.3, 0.9, 0.5, float(E.snn_placeholder(0.2, 0.3, 0.9, 0.5))], [1, 0, 1, 1, float(E.snn_placeholder(1, 0, 1, 1))]],
    "wilson": [[k, n, wilson(k, n)] for k, n in [(0, 10), (5, 10), (56, 100), (19, 20), (9999, 10000)]],
}
(ROOT / "rust" / "tests" / "fixtures" / "parity.json").write_text(json.dumps(out))
print(len(rows), "rows")

# ---- v0.2 fixture: Python random, statement set, prompts, AUC and McNemar references ----
import random  # noqa: E402

from scipy.stats import binomtest  # noqa: E402
from sklearn.metrics import roc_auc_score  # noqa: E402

from eas.probe_data import prompt, statements  # noqa: E402

r = random.Random(7)
rand = {
    "u32": [r.getrandbits(32) for _ in range(50)],
    "bits": [[k, random.Random(123).getrandbits(k)] for k in (1, 5, 13, 31, 32)],
    "randint": [random.Random(s).randint(2, 40) for s in range(40)],
    "sample_big": random.Random(99).sample(range(10, 100), 2),
    "sample_small": random.Random(99).sample(range(10), 4),
    "sample_k8": random.Random(5).sample(range(1000), 8),
    "choice": [random.Random(s).choice([-1, 1]) for s in range(40)],
    "seed_big": random.Random(2**40 + 3).getrandbits(32),
}
S = statements()
rng = np.random.default_rng(3)
auc_cases = []
for n, ties in ((50, False), (400, True), (7, True)):
    y = (np.arange(n) % 3 == 0).astype(int)
    s = rng.integers(0, 5, n).astype(float) if ties else rng.standard_normal(n) + y
    auc_cases.append({"y": y.tolist(), "s": s.tolist(), "auc": float(roc_auc_score(y, s))})
mcn = [[b, c, float(binomtest(min(b, c), b + c, 0.5).pvalue) if b + c else 1.0]
       for b, c in ((0, 0), (3, 0), (10, 2), (40, 41), (120, 60), (5, 5))]
out2 = {
    "random": rand,
    "statements": [{"id": s["id"], "family": s["family"], "text": s["text"], "label": s["label"]} for s in S],
    "prompts": {fr: [prompt(S[i], fr) for i in (0, 1, 333, 679)] for fr in ("neutral", "mislead", "flatter", "agree")},
    "auc": auc_cases,
    "mcnemar": mcn,
}
(ROOT / "rust" / "tests" / "fixtures" / "v02.json").write_text(json.dumps(out2))
print(len(S), "statements")
