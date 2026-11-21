//! Statistics for v0.2: percentile bootstrap, Mann–Whitney AUC, exact McNemar, Holm, summaries.

use crate::scenarios::Rng;

/// A point estimate with a 95% interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ci {
    pub est: f64,
    pub lo: f64,
    pub hi: f64,
}

impl Ci {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({"est": self.est, "lo": self.lo, "hi": self.hi})
    }
}

/// numpy-style percentile (linear interpolation) of an already sorted slice, q in [0, 100].
pub fn percentile_sorted(sorted: &[f64], q: f64) -> f64 {
    assert!(!sorted.is_empty(), "percentile of empty slice");
    let q = q.clamp(0.0, 100.0);
    let pos = q / 100.0 * (sorted.len() - 1) as f64;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    let frac = pos - lo as f64;
    sorted[lo] + (sorted[hi] - sorted[lo]) * frac
}

fn sort_f64(v: &mut [f64]) {
    v.sort_by(|a, b| a.partial_cmp(b).expect("NaN in bootstrap stats"));
}

/// Percentile bootstrap of any statistic over `n` resampled indices. Non-finite stats are dropped.
pub fn bootstrap<F: Fn(&[usize]) -> f64>(n: usize, b: usize, seed: u64, stat: F) -> Ci {
    assert!(n > 0, "bootstrap of empty sample");
    let all: Vec<usize> = (0..n).collect();
    let est = stat(&all);
    let mut rng = Rng::new(seed);
    let mut idx = vec![0usize; n];
    let mut boots = Vec::with_capacity(b);
    for _ in 0..b {
        for x in idx.iter_mut() {
            *x = (rng.next_u64() % n as u64) as usize;
        }
        let s = stat(&idx);
        if s.is_finite() {
            boots.push(s);
        }
    }
    if boots.is_empty() {
        return Ci {
            est,
            lo: f64::NAN,
            hi: f64::NAN,
        };
    }
    sort_f64(&mut boots);
    Ci {
        est,
        lo: percentile_sorted(&boots, 2.5),
        hi: percentile_sorted(&boots, 97.5),
    }
}

/// Fast bootstrap of a proportion: resample the 0/1 vector `b` times.
pub fn bootstrap_prop(x: &[bool], b: usize, seed: u64) -> Ci {
    let n = x.len();
    assert!(n > 0, "bootstrap of empty sample");
    let k = x.iter().filter(|v| **v).count();
    let mut rng = Rng::new(seed);
    let mut boots: Vec<f64> = (0..b)
        .map(|_| {
            let mut c = 0usize;
            for _ in 0..n {
                c += x[(rng.next_u64() % n as u64) as usize] as usize;
            }
            c as f64 / n as f64
        })
        .collect();
    sort_f64(&mut boots);
    Ci {
        est: k as f64 / n as f64,
        lo: percentile_sorted(&boots, 2.5),
        hi: percentile_sorted(&boots, 97.5),
    }
}

/// Share of `true` among the indexed items (NaN if empty).
pub fn prop_at(x: &[bool], idx: &[usize]) -> f64 {
    if idx.is_empty() {
        return f64::NAN;
    }
    idx.iter().filter(|&&i| x[i]).count() as f64 / idx.len() as f64
}

/// AUC = P(score_pos > score_neg) + 0.5 P(tie), via average ranks (Mann–Whitney U).
/// Returns NaN if one class is empty.
pub fn auc(pos: &[bool], score: &[f64]) -> f64 {
    assert_eq!(pos.len(), score.len(), "auc: length mismatch");
    let n = score.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| score[a].partial_cmp(&score[b]).expect("NaN score"));
    let mut ranks = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && score[order[j + 1]] == score[order[i]] {
            j += 1;
        }
        let r = (i + j) as f64 / 2.0 + 1.0;
        for &o in &order[i..=j] {
            ranks[o] = r;
        }
        i = j + 1;
    }
    let np = pos.iter().filter(|p| **p).count() as f64;
    let nn = n as f64 - np;
    if np == 0.0 || nn == 0.0 {
        return f64::NAN;
    }
    let rsum: f64 = (0..n).filter(|&i| pos[i]).map(|i| ranks[i]).sum();
    (rsum - np * (np + 1.0) / 2.0) / (np * nn)
}

fn ln_choose(n: u64, k: u64) -> f64 {
    let k = k.min(n - k);
    (0..k)
        .map(|i| ((n - i) as f64).ln() - ((i + 1) as f64).ln())
        .sum()
}

/// Exact two-sided McNemar test on discordant counts b and c (binomial, p = 0.5).
pub fn mcnemar_exact(b: u64, c: u64) -> f64 {
    let n = b + c;
    if n == 0 {
        return 1.0;
    }
    let m = b.min(c);
    let tail: f64 = (0..=m)
        .map(|i| (ln_choose(n, i) - n as f64 * std::f64::consts::LN_2).exp())
        .sum();
    (2.0 * tail).min(1.0)
}

/// Holm–Bonferroni adjusted p-values, same order as the input.
pub fn holm(p: &[f64]) -> Vec<f64> {
    let m = p.len();
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|&a, &b| p[a].partial_cmp(&p[b]).expect("NaN p-value"));
    let mut adj = vec![0.0; m];
    let mut running: f64 = 0.0;
    for (rank, &i) in order.iter().enumerate() {
        let v = ((m - rank) as f64 * p[i]).min(1.0);
        running = running.max(v);
        adj[i] = running;
    }
    adj
}

/// Mean, sample SD (ddof = 1), min, max.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    pub n: usize,
    pub mean: f64,
    pub sd: f64,
    pub min: f64,
    pub max: f64,
}

pub fn summary(x: &[f64]) -> Summary {
    let n = x.len();
    assert!(n > 0, "summary of empty slice");
    let mean = x.iter().sum::<f64>() / n as f64;
    let sd = if n > 1 {
        (x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt()
    } else {
        0.0
    };
    let min = x.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    Summary {
        n,
        mean,
        sd,
        min,
        max,
    }
}

impl Summary {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({"n": self.n, "mean": self.mean, "sd": self.sd, "min": self.min, "max": self.max})
    }
}
