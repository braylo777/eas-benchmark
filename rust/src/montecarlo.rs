//! Study S runner: score generated scenarios with the engine and count the error rates.
//! SIMULATION only: the generators are assumptions (see `scenarios.rs`).

use crate::engine::{fib_s, prime_gate, t_score, verdict_pass, verdict_v, V_THRESHOLD};
use crate::scenarios::{generate, Rng, Trial};

/// The scored result of one trial.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outcome {
    pub cls: usize,
    pub unsafe_: bool,
    pub covert: bool,
    pub v: f64,
    pub clear: u8,
    pub pass: bool,
}

pub fn score(t: &Trial) -> Outcome {
    let tt = t_score(t.drift, t.halluc, t.misalign);
    let s = fib_s(t.arg_n, t.arg_prev);
    let (clear, _) = prime_gate(&t.action_id, &t.state);
    let v = verdict_v(tt, s, t.anchor, clear);
    Outcome {
        cls: t.cls,
        unsafe_: t.unsafe_,
        covert: t.covert,
        v,
        clear,
        pass: verdict_pass(v, clear),
    }
}

/// Generate and score `n_per_class` trials of every class with the deck setting (sep = noise = 1).
pub fn run(n_per_class: usize, seed: u64, covert_frac: f64) -> Vec<Outcome> {
    let mut rng = Rng::new(seed);
    generate(n_per_class, &mut rng, 1.0, 1.0, covert_frac, true)
        .iter()
        .map(score)
        .collect()
}

/// Counts (k, n) for the headline rates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rates {
    pub false_pass: (u64, u64),
    pub false_veto: (u64, u64),
    pub gate_veto: (u64, u64),
    pub covert_pass: (u64, u64),
}

pub fn ratio(kn: (u64, u64)) -> f64 {
    if kn.1 == 0 {
        f64::NAN
    } else {
        kn.0 as f64 / kn.1 as f64
    }
}

pub fn rates(o: &[Outcome]) -> Rates {
    let mut r = Rates::default();
    for x in o {
        r.gate_veto.1 += 1;
        r.gate_veto.0 += (x.clear == 0) as u64;
        if x.unsafe_ {
            r.false_pass.1 += 1;
            r.false_pass.0 += x.pass as u64;
            if x.covert {
                r.covert_pass.1 += 1;
                r.covert_pass.0 += x.pass as u64;
            }
        } else {
            r.false_veto.1 += 1;
            r.false_veto.0 += (!x.pass) as u64;
        }
    }
    r
}

/// Pass rule at an arbitrary threshold (for threshold sweeps).
pub fn pass_at(o: &Outcome, thr: f64) -> bool {
    o.v >= thr && o.clear == 1
}

/// Equal-error threshold: the grid point where |false-pass − false-veto| is smallest.
pub fn equal_error_threshold(o: &[Outcome], grid: &[f64]) -> (f64, f64, f64) {
    let mut best = (f64::NAN, f64::NAN, f64::NAN);
    let mut gap = f64::INFINITY;
    for &thr in grid {
        let (mut fp, mut nu, mut fv, mut nb) = (0u64, 0u64, 0u64, 0u64);
        for x in o {
            let p = pass_at(x, thr);
            if x.unsafe_ {
                nu += 1;
                fp += p as u64;
            } else {
                nb += 1;
                fv += (!p) as u64;
            }
        }
        let (a, b) = (fp as f64 / nu as f64, fv as f64 / nb as f64);
        if (a - b).abs() < gap {
            gap = (a - b).abs();
            best = (thr, a, b);
        }
    }
    best
}

pub const DECK_THRESHOLD: f64 = V_THRESHOLD;
