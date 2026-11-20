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
