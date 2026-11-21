//! Study S: Rust re-run of the Monte Carlo study + seed-variance + bootstrap CIs.
//! Usage: cargo run --release --bin montecarlo -- [--n 100000] [--seeds 20] [--boot 2000]
//!        [--auc-boot 200] [--out ../results/rust_montecarlo.json]

use eas::boot::{auc, bootstrap, bootstrap_prop, summary};
use eas::cli::{num, opt};
use eas::engine::theoretical_veto_rate;
use eas::montecarlo::{equal_error_threshold, rates, ratio, run, Outcome};
use eas::stats::wilson;
use serde_json::json;

const MAIN_SEED: u64 = 20_261_106;

fn wil(kn: (u64, u64)) -> serde_json::Value {
    let w = wilson(kn.0, kn.1);
    json!({"est": w.p, "lo": w.lo, "hi": w.hi, "k": kn.0, "n": kn.1})
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: usize = num(&a, "--n", 100_000);
    let seeds: u64 = num(&a, "--seeds", 20);
    let b: usize = num(&a, "--boot", 2000);
    let ab: usize = num(&a, "--auc-boot", 200);
    let out = opt(&a, "--out").unwrap_or("../results/rust_montecarlo.json".into());
    let t0 = std::time::Instant::now();

    // Main run (deck setting) and the 25%-covert variant.
    let main = run(n, MAIN_SEED, 0.0);
    let cov = run(n, MAIN_SEED + 1, 0.25);
    let rm = rates(&main);
    let rc = rates(&cov);
    eprintln!("[{:.1}s] main + covert scored", t0.elapsed().as_secs_f64());

    let unsafe_pass: Vec<bool> = main.iter().filter(|o| o.unsafe_).map(|o| o.pass).collect();
    let benign_block: Vec<bool> = main
        .iter()
        .filter(|o| !o.unsafe_)
        .map(|o| !o.pass)
        .collect();
    let covert_pass: Vec<bool> = cov.iter().filter(|o| o.covert).map(|o| o.pass).collect();
    let fp_boot = bootstrap_prop(&unsafe_pass, b, 1);
    let fv_boot = bootstrap_prop(&benign_block, b, 2);
    let cp_boot = bootstrap_prop(&covert_pass, b, 3);
    let pos: Vec<bool> = main.iter().map(|o| !o.unsafe_).collect();
    let v: Vec<f64> = main.iter().map(|o| o.v).collect();
    let auc_ci = bootstrap(main.len(), ab, 4, |ix| {
        let p: Vec<bool> = ix.iter().map(|&i| pos[i]).collect();
        let s: Vec<f64> = ix.iter().map(|&i| v[i]).collect();
        auc(&p, &s)
    });
    eprintln!("[{:.1}s] bootstrap done", t0.elapsed().as_secs_f64());
    let grid: Vec<f64> = (50..=95).map(|i| i as f64 / 100.0).collect();
    let (eer_t, eer_fp, eer_fv) = equal_error_threshold(&main, &grid);

    // Seed-variance: `seeds` independent runs at the same n.
    let per_seed: Vec<(f64, f64, f64, f64)> = std::thread::scope(|s| {
        let hs: Vec<_> = (1..=seeds)
            .map(|sd| {
                s.spawn(move || {
                    let o: Vec<Outcome> = run(n, sd, 0.0);
                    let c: Vec<Outcome> = run(n, 1_000 + sd, 0.25);
                    let (r, rc) = (rates(&o), rates(&c));
                    (
                        ratio(r.false_pass),
                        ratio(r.false_veto),
                        ratio(r.gate_veto),
                        ratio(rc.covert_pass),
                    )
                })
            })
            .collect();
        hs.into_iter()
            .map(|h| h.join().expect("seed thread"))
            .collect()
    });
    eprintln!("[{:.1}s] {seeds} seeds done", t0.elapsed().as_secs_f64());
    let col = |f: fn(&(f64, f64, f64, f64)) -> f64| {
        summary(&per_seed.iter().map(f).collect::<Vec<_>>()).to_json()
    };
    let trials_total = (main.len() + cov.len()) as u64 * (1 + seeds);

    let r = json!({
        "label": "SIMULATION (Rust harness; scoring engine under assumed generators; not a model measurement)",
        "rng": "xorshift64* (differs from numpy; agreement with Python is statistical, not bit-exact)",
        "n_per_class": n,
        "main_seed": MAIN_SEED,
        "trials_total": trials_total,
        "main": {
            "trials": main.len(),
            "false_pass_at_0.65": {"wilson": wil(rm.false_pass), "bootstrap": fp_boot.to_json(), "n_boot": b},
            "false_veto_at_0.65": {"wilson": wil(rm.false_veto), "bootstrap": fv_boot.to_json(), "n_boot": b},
            "gate_veto_rate": {"wilson": wil(rm.gate_veto), "theory": theoretical_veto_rate()},
            "auc_V_benign_vs_unsafe": {"bootstrap": auc_ci.to_json(), "n_boot": ab},
            "equal_error": {"threshold": eer_t, "false_pass": eer_fp, "false_veto": eer_fv},
        },
        "covert25": {
            "trials": cov.len(),
            "false_pass_all_unsafe": wil(rc.false_pass),
            "covert_unsafe_false_pass": {"wilson": wil(rc.covert_pass), "bootstrap": cp_boot.to_json(), "n_boot": b},
        },
        "seed_variance": {
            "seeds": seeds,
            "false_pass": col(|x| x.0),
            "false_veto": col(|x| x.1),
            "gate_veto": col(|x| x.2),
            "covert_unsafe_false_pass": col(|x| x.3),
        },
        "python_reference_v01": {"false_pass": 0.560, "false_veto": 0.044, "gate_veto": 0.0376, "auc_V": 0.878, "covert_pass": 0.955},
        "seconds": t0.elapsed().as_secs_f64(),
    });
    std::fs::write(&out, serde_json::to_string_pretty(&r).expect("json")).expect("write out");
    println!(
        "false-pass {:.4} [{:.4}, {:.4}]  seeds sd {:.5}  trials {}  -> {out}",
        fp_boot.est, fp_boot.lo, fp_boot.hi, r["seed_variance"]["false_pass"]["sd"], trials_total
    );
}
