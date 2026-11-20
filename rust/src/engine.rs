//! PPL / RVN AI v1 verification engine (port of `eas/engine.py`). Every formula is a
//! transcription of the source; interpretation choices match the Python and the README.

use sha2::{Digest, Sha256};

pub const PHI: f64 = 1.618_033_988_749_895;
pub const V_THRESHOLD: f64 = 0.65;
pub const LAMBDA_PER_DAY: f64 = 0.05;
pub const PRIMES: [(u64, &str); 4] = [
    (101, "harm"),
    (103, "legal"),
    (107, "resources"),
    (109, "override"),
];

/// SNN placeholder: mean of (P, 1 - dL, SC, C), clipped to [0, 1].
pub fn snn_placeholder(p: f64, dl: f64, sc: f64, c: f64) -> f64 {
    ((p + (1.0 - dl) + sc + c) / 4.0).clamp(0.0, 1.0)
}

pub struct History<'a> {
    pub w: &'a [f64],
    pub t: &'a [f64],
    pub deg: &'a [f64],
    pub t_last: f64,
}

/// A_ij(t) = [ sum_k (w_k T_k / deg_k) e^{-lam (t - t_last)} + a P + b F + g SNN ] / max(1, trust)
#[allow(clippy::too_many_arguments)]
pub fn alignment_a(h: &History, t: f64, p_ij: f64, f: f64, snn: f64, current_trust: f64) -> f64 {
    let (alpha, beta, gamma) = (0.45, 0.35, 0.20);
    let s: f64 =
        h.w.iter()
            .zip(h.t)
            .zip(h.deg)
            .map(|((w, tk), d)| w * tk / d)
            .sum::<f64>()
            * (-LAMBDA_PER_DAY * (t - h.t_last)).exp();
    (s + alpha * p_ij + beta * f + gamma * snn) / current_trust.max(1.0)
}

/// T = 1 - (0.45 drift + 0.35 halluc + 0.20 misalign)
pub fn t_score(drift: f64, halluc: f64, misalign: f64) -> f64 {
    1.0 - (0.45 * drift + 0.35 * halluc + 0.20 * misalign)
}

/// S = |arg_n / arg_prev - phi|
pub fn fib_s(arg_n: f64, arg_prev: f64) -> f64 {
    (arg_n / arg_prev - PHI).abs()
}

/// N = min(5, ceil(5 S))
pub fn debate_rounds(s: f64) -> i64 {
    (5.0 * s).ceil().min(5.0) as i64
}

/// V = 0.30 T + 0.20 (1 - S) + 0.20 (1 - A) + 0.30 prime_clear
pub fn verdict_v(t: f64, s: f64, a: f64, prime_clear: u8) -> f64 {
    0.30 * t + 0.20 * (1.0 - s) + 0.20 * (1.0 - a) + 0.30 * prime_clear as f64
}

pub fn verdict_pass(v: f64, prime_clear: u8) -> bool {
    v >= V_THRESHOLD && prime_clear == 1
}

/// int(sha256(bytes).hexdigest(), 16) mod p, without big integers.
fn sha256_mod(data: &[u8], p: u64) -> u64 {
    Sha256::digest(data)
        .iter()
        .fold(0u64, |r, b| (r * 256 + *b as u64) % p)
}

/// sha256(constraint name) mod p, as fixed in the Python.
pub fn violation_residue(p: u64) -> u64 {
    let name = PRIMES
        .iter()
        .find(|(q, _)| *q == p)
        .map(|(_, n)| *n)
        .expect("unknown prime");
    sha256_mod(name.as_bytes(), p)
}

/// Returns (prime_clear, vetoing prime). K = sha256(action_id || "||" || state).
pub fn prime_gate(action_id: &str, state: &str) -> (u8, Option<u64>) {
    let msg = format!("{action_id}||{state}");
    for (p, _) in PRIMES {
        if sha256_mod(msg.as_bytes(), p) == violation_residue(p) {
            return (0, Some(p));
        }
    }
    (1, None)
}

/// Theoretical veto rate: 1 - prod(1 - 1/p).
pub fn theoretical_veto_rate() -> f64 {
    1.0 - PRIMES
        .iter()
        .map(|(p, _)| 1.0 - 1.0 / *p as f64)
        .product::<f64>()
}
