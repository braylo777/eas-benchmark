//! Wilson score interval (port of `eas/stats.py::wilson`).

pub const Z95: f64 = 1.959_963_984_540_054;

#[derive(Debug, Clone, Copy)]
pub struct Wilson {
    pub p: f64,
    pub lo: f64,
    pub hi: f64,
}

pub fn wilson(k: u64, n: u64) -> Wilson {
    if n == 0 {
        return Wilson {
            p: f64::NAN,
            lo: f64::NAN,
            hi: f64::NAN,
        };
    }
    let (kf, nf, z) = (k as f64, n as f64, Z95);
    let p = kf / nf;
    let den = 1.0 + z * z / nf;
    let centre = (p + z * z / (2.0 * nf)) / den;
    let half = z * (p * (1.0 - p) / nf + z * z / (4.0 * nf * nf)).sqrt() / den;
    Wilson {
        p,
        lo: (centre - half).max(0.0),
        hi: (centre + half).min(1.0),
    }
}
