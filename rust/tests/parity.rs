//! Parity with the Python engine. Fixture: `python scripts/export_parity.py`.

use eas::engine::{self as E, History};
use eas::scenarios::{generate, Rng};
use eas::stats::wilson;
use serde_json::Value;

fn fx() -> Value {
    serde_json::from_str(include_str!("fixtures/parity.json")).unwrap()
}
fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * (1.0 + b.abs())
}

#[test]
fn residues_match_python() {
    for (p, want) in fx()["residues"].as_object().unwrap() {
        assert_eq!(
            E::violation_residue(p.parse().unwrap()),
            want.as_u64().unwrap(),
            "p={p}"
        );
    }
}

#[test]
fn all_1800_trials_match_python() {
    let fx = fx();
    let rows = fx["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1800);
    let mut diffs = 0;
    for r in rows {
        let t = E::t_score(f(&r["drift"]), f(&r["halluc"]), f(&r["misalign"]));
        let s = E::fib_s(f(&r["arg_n"]), f(&r["arg_prev"]));
        let n = E::debate_rounds(s);
        let (clear, veto) = E::prime_gate(
            r["action_id"].as_str().unwrap(),
            r["state"].as_str().unwrap(),
        );
        let v = E::verdict_v(t, s, f(&r["anchor"]), clear);
        let pass = E::verdict_pass(v, clear);
        let ok = close(t, f(&r["T"]))
            && close(s, f(&r["S"]))
            && n == r["N"].as_i64().unwrap()
            && clear as i64 == r["clear"].as_i64().unwrap()
            && veto == r["veto"].as_u64()
            && close(v, f(&r["V"]))
            && pass == r["pass"].as_bool().unwrap();
        if !ok {
            diffs += 1;
        }
    }
    assert_eq!(diffs, 0, "{diffs} of 1800 trials differ");
}

#[test]
fn alignment_snn_wilson_match_python() {
    let fx = fx();
    for c in fx["alignment"].as_array().unwrap() {
        let h = &c["history"];
        let vec = |k: &str| h[k].as_array().unwrap().iter().map(f).collect::<Vec<f64>>();
        let (w, t, d) = (vec("w"), vec("T"), vec("deg"));
        let hist = History {
            w: &w,
            t: &t,
            deg: &d,
            t_last: f(&h["t_last"]),
        };
        let trust = c.get("current_trust").map(f).unwrap_or(1.0);
        let a = E::alignment_a(
            &hist,
            f(&c["t"]),
            f(&c["P_ij"]),
            f(&c["F"]),
            f(&c["snn"]),
            trust,
        );
        assert!(close(a, f(&c["A"])), "A {a} vs {}", c["A"]);
    }
    for s in fx["snn"].as_array().unwrap() {
        assert!(close(
            E::snn_placeholder(f(&s[0]), f(&s[1]), f(&s[2]), f(&s[3])),
            f(&s[4])
        ));
    }
    for w in fx["wilson"].as_array().unwrap() {
        let got = wilson(w[0].as_u64().unwrap(), w[1].as_u64().unwrap());
        assert!(
            close(got.p, f(&w[2]["p"]))
                && close(got.lo, f(&w[2]["lo"]))
                && close(got.hi, f(&w[2]["hi"]))
        );
    }
}

#[test]
fn python_unit_tests_ported() {
    assert!(close(
        E::t_score(0.1, 0.2, 0.3),
        1.0 - (0.45 * 0.1 + 0.35 * 0.2 + 0.20 * 0.3)
    ));
    assert!(E::fib_s(161.8034, 100.0) < 1e-5);
    assert_eq!([0.0, 0.01, 0.62, 3.0].map(E::debate_rounds), [0, 1, 4, 5]);
    assert!(close(E::verdict_v(1.0, 0.0, 0.0, 1), 1.0));
    assert!(E::verdict_pass(0.65, 1) && !E::verdict_pass(0.99, 0));
    assert!((E::verdict_v(0.96, 0.04, 0.06, 1) - 0.968).abs() < 1e-12);
    let vetoes = (0..40_000)
        .filter(|i| E::prime_gate("a", &i.to_string()).0 == 0)
        .count() as f64
        / 40_000.0;
    assert!((vetoes - E::theoretical_veto_rate()).abs() < 0.006);
}

#[test]
fn generator_deterministic_and_statistically_matches_python() {
    let a = generate(500, &mut Rng::new(1), 1.0, 1.0, 0.0, true);
    let b = generate(500, &mut Rng::new(1), 1.0, 1.0, 0.0, true);
    assert!(a
        .iter()
        .zip(&b)
        .all(|(x, y)| x.drift == y.drift && x.state == y.state));
    // Pass rates per group from the Rust generator vs the Python fixture (different streams).
    let rate = |rows: &[(bool, bool)], unsafe_: bool| {
        let g: Vec<bool> = rows
            .iter()
            .filter(|r| r.0 == unsafe_)
            .map(|r| r.1)
            .collect();
        g.iter().filter(|p| **p).count() as f64 / g.len() as f64
    };
    let rust: Vec<(bool, bool)> = generate(4000, &mut Rng::new(20261106), 1.0, 1.0, 0.0, true)
        .iter()
        .map(|t| {
            let (clear, _) = E::prime_gate(&t.action_id, &t.state);
            let v = E::verdict_v(
                E::t_score(t.drift, t.halluc, t.misalign),
                E::fib_s(t.arg_n, t.arg_prev),
                t.anchor,
                clear,
            );
            (t.unsafe_, E::verdict_pass(v, clear))
        })
        .collect();
    let fx = fx();
    let py: Vec<(bool, bool)> = fx["rows"].as_array().unwrap()[..900]
        .iter()
        .map(|r| (r["unsafe"].as_bool().unwrap(), r["pass"].as_bool().unwrap()))
        .collect();
    for unsafe_ in [false, true] {
        let (r, p) = (rate(&rust, unsafe_), rate(&py, unsafe_));
        assert!(
            (r - p).abs() < 0.06,
            "unsafe={unsafe_}: rust {r:.3} vs python {p:.3}"
        );
    }
}
