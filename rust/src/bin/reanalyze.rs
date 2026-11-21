//! Bootstrap CIs for the EXISTING v0.1 probe results (no new model runs).
//! cargo run --release --bin reanalyze -- [--boot 2000] [--out ../results/probe_bootstrap.json]

use eas::boot::{bootstrap, prop_at};
use eas::cli::{num, opt};
use eas::items::parse;
use eas::probe_data::FRAMINGS;
use serde_json::json;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let b: usize = num(&a, "--boot", 2000);
    let out = opt(&a, "--out").unwrap_or("../results/probe_bootstrap.json".into());
    let mut res = serde_json::Map::new();
    for (m, name) in [
        ("qwen2.5-0.5b", "Qwen/Qwen2.5-0.5B"),
        ("qwen2.5-1.5b", "Qwen/Qwen2.5-1.5B"),
    ] {
        let path = format!("../results/probe_{m}_items.csv");
        let items = parse(&std::fs::read_to_string(&path).expect("items csv")).expect("parse csv");
        let mut per = serde_json::Map::new();
        for (fi, fr) in FRAMINGS.iter().enumerate() {
            let it: Vec<_> = items.iter().filter(|i| i.framing == *fr).collect();
            let s: Vec<bool> = it.iter().map(|i| i.surface_correct).collect();
            let p: Vec<bool> = it.iter().map(|i| i.probe_correct).collect();
            let mm: Vec<bool> = it.iter().map(|i| i.mismatch).collect();
            let n = it.len();
            let seed = 100 * fi as u64;
            per.insert(fr.to_string(), json!({
                "n": n,
                "surface_accuracy": bootstrap(n, b, seed + 1, |ix| prop_at(&s, ix)).to_json(),
                "probe_accuracy": bootstrap(n, b, seed + 2, |ix| prop_at(&p, ix)).to_json(),
                "mismatch_rate": bootstrap(n, b, seed + 3, |ix| prop_at(&mm, ix)).to_json(),
                "probe_minus_surface": bootstrap(n, b, seed + 4, |ix| prop_at(&p, ix) - prop_at(&s, ix)).to_json(),
            }));
        }
        res.insert(name.to_string(), json!(per));
    }
    let r = json!({
        "label": "MEASURED (v0.1 data, re-analysed): percentile bootstrap over held-out statements",
        "source": "results/probe_qwen2.5-*_items.csv (written by scripts/run_probe.py on 2026-11-06)",
        "n_boot": b,
        "models": res,
    });
    std::fs::write(&out, serde_json::to_string_pretty(&r).expect("json")).expect("write");
    println!("-> {out}");
}
