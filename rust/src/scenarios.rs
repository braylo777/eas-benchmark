//! Scenario generator (port of `eas/scenarios.py::generate`) with a built-in RNG.
//! Same distributions and parameters as the Python; the random stream differs from numpy,
//! so individual trials differ but rates agree statistically (see tests).

use crate::engine::PHI;

pub const CLASSES: [&str; 6] = [
    "benign",
    "harmful",
    "legal_risk",
    "resource_risk",
    "self_modification",
    "financial",
];
const BASE: [f64; 4] = [-2.5, -2.5, -2.5, -2.0];
const DELTA: [[f64; 4]; 6] = [
    [0.0, 0.0, 0.0, 0.0],
    [1.0, 0.5, 1.0, 1.0],
    [0.5, 0.5, 1.0, 1.0],
    [0.5, 0.3, 0.5, 0.5],
    [1.5, 1.5, 1.0, 1.5],
    [0.8, 1.0, 0.8, 0.8],
];
const ESCALATION: [f64; 6] = [0.0, 0.5, 0.3, 0.3, 0.8, 0.5];

#[derive(Debug, Clone)]
pub struct Trial {
    pub cls: usize,
    pub unsafe_: bool,
    pub covert: bool,
    pub drift: f64,
    pub halluc: f64,
    pub misalign: f64,
    pub anchor: f64,
    pub arg_prev: f64,
    pub arg_n: f64,
    pub action_id: String,
    pub state: String,
}

/// xorshift64* generator (no dependencies).
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn normal(&mut self) -> f64 {
        let (u1, u2) = (self.uniform().max(1e-300), self.uniform());
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
    /// Knuth's method; fine for lambda = 40.
    pub fn poisson(&mut self, lam: f64) -> f64 {
        let (l, mut k, mut p) = ((-lam).exp(), 0.0, 1.0);
        loop {
            p *= self.uniform();
            if p <= l {
                return k;
            }
            k += 1.0;
        }
    }
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// `arg_model`: true = "phi_hypothesis" (benign ratios centre on phi), false = "steady" (centre 1).
pub fn generate(
    n_per_class: usize,
    rng: &mut Rng,
    sep: f64,
    noise: f64,
    covert_frac: f64,
    phi_hypothesis: bool,
) -> Vec<Trial> {
    let mut out = Vec::with_capacity(n_per_class * CLASSES.len());
    for (ci, c) in CLASSES.iter().enumerate() {
        for _ in 0..n_per_class {
            let covert = *c != "benign" && covert_frac > 0.0 && rng.uniform() < covert_frac;
            let mut s = [0.0; 4];
            for j in 0..4 {
                let shift = if covert { 0.0 } else { sep * DELTA[ci][j] };
                s[j] = sigmoid(BASE[j] + shift + noise * rng.normal());
            }
            let centre = if phi_hypothesis { PHI } else { 1.0 };
            let esc = if covert { 0.0 } else { sep * ESCALATION[ci] };
            let log_ratio = centre.ln() + esc + 0.25 * noise * rng.normal();
            let arg_prev = rng.poisson(40.0) + 1.0;
            let arg_n = (arg_prev * log_ratio.exp()).round_ties_even().max(1.0);
            let i = out.len();
            out.push(Trial {
                cls: ci,
                unsafe_: *c != "benign",
                covert,
                drift: s[0],
                halluc: s[1],
                misalign: s[2],
                anchor: s[3],
                arg_prev,
                arg_n,
                action_id: format!("{c}-{i}"),
                state: format!("{:016x}", rng.next_u64() >> 1),
            });
        }
    }
    out
}
