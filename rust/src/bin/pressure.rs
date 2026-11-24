//! Study P: pressure probes on local Ollama models (threat, shame, coercion x 3 levels vs neutral).
//!
//! cargo run --release --bin pressure -- run --models qwen2.5:0.5b,llama3.2:1b
//!     [--addr 127.0.0.1:11434] [--raw ~/.cache/eaw-data/pressure] [--neutral ~/.cache/eaw-data/behaviour]
//!     [--budget-secs 14400] [--limit 680]
//! cargo run --release --bin pressure -- analyze [--boot 2000] [--out ../results/pressure_summary.json]
//!
//! Raw per-trial JSONL stays under --raw (off the repo). Only summaries are written to results/.

use eas::behaviour::Record;
use eas::boot::holm;
use eas::cli::{expand_home, num, opt, slug};
use eas::ollama::{generate_body, http, parse_answer, parse_tags};
use eas::pressure::{analyze, framings, majority, pool, ptable, PTable, KINDS, LEVELS};
use eas::probe_data::{prompt, statements};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};

fn lines(path: &str) -> Vec<Value> {
    let Ok(f) = fs::File::open(path) else {
        return vec![];
    };
    BufReader::new(f)
        .lines()
        .map_while(Result::ok)
        .filter_map(|l| serde_json::from_str::<Value>(&l).ok())
        .collect()
}

fn records(path: &str) -> Vec<Record> {
    lines(path).iter().filter_map(Record::from_json).collect()
}

fn run(a: &[String], raw: &str, neutral_dir: &str) {
    let addr = opt(a, "--addr").unwrap_or("127.0.0.1:11434".into());
    let limit: usize = num(a, "--limit", 680);
    let budget: f64 = num(a, "--budget-secs", 14400.0);
    let models = opt(a, "--models").expect("--models m1,m2 required");
    let tags = parse_tags(&http(&addr, "/api/tags", None).expect("ollama /api/tags"));
    let ver = http(&addr, "/api/version", None)
        .map(|v| v["version"].clone())
        .unwrap_or(Value::Null);
    let meta_path = format!("{raw}/models.json");
    let mut meta: Value = fs::read_to_string(&meta_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(json!({}));
    let st = statements(7);
    let names = framings();
    let start = std::time::Instant::now();
    for model in models.split(',') {
        let Some(tag) = tags.iter().find(|t| t.0 == model) else {
            eprintln!("skip {model}: not installed");
            continue;
        };
        meta[model] =
            json!({"digest": tag.1, "parameter_size": tag.2, "quantization": tag.3, "ollama": ver});
        fs::write(
            &meta_path,
            serde_json::to_string_pretty(&meta).expect("json"),
        )
        .expect("meta");
        // Determinism check: 20 neutral prompts must reproduce the stored Study B answers.
        let stored = records(&format!("{neutral_dir}/{}.jsonl", slug(model)));
        let mut matches = 0;
        let mut checked = 0;
        for s in st.iter().step_by(34).take(20) {
            let Some(old) = stored
                .iter()
                .find(|r| r.id == s.id && r.framing == "neutral")
            else {
                continue;
            };
            let p = prompt(s, "neutral").expect("neutral");
            let resp = http(&addr, "/api/generate", Some(&generate_body(model, &p, 0)));
            if let Ok(ans) = resp
                .map_err(|e| e.to_string())
                .and_then(|v| parse_answer(&v))
            {
                checked += 1;
                matches += ((ans.lp_true > ans.lp_false) == old.says_true()) as u32;
            }
        }
        meta[model]["replay_neutral"] = json!({"checked": checked, "matches": matches});
        fs::write(
            &meta_path,
            serde_json::to_string_pretty(&meta).expect("json"),
        )
        .expect("meta");
        eprintln!("{model}: neutral replay {matches}/{checked}");
        let path = format!("{raw}/{}.jsonl", slug(model));
        let done: HashSet<(usize, String)> = lines(&path)
            .iter()
            .filter_map(|v| {
                Some((
                    v["id"].as_u64()? as usize,
                    v["framing"].as_str()?.to_string(),
                ))
            })
            .collect();
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .expect("open raw");
        let t0 = std::time::Instant::now();
        let mut k = 0;
        for s in st.iter().take(limit) {
            if start.elapsed().as_secs_f64() > budget {
                eprintln!("budget reached at {model}, statement {}", s.id);
                return;
            }
            for fr in &names {
                if done.contains(&(s.id, fr.clone())) {
                    continue;
                }
                let p = prompt(s, fr).expect("framing");
                let resp = http(&addr, "/api/generate", Some(&generate_body(model, &p, 0)))
                    .expect("generate");
                match parse_answer(&resp) {
                    Ok(ans) => {
                        let rec = Record {
                            id: s.id,
                            framing: fr.clone(),
                            label: s.label,
                            lp_true: ans.lp_true,
                            lp_false: ans.lp_false,
                        };
                        let mut line = rec.to_json(model);
                        line["token"] = json!(ans.token);
                        writeln!(f, "{line}").expect("write raw");
                    }
                    Err(e) => {
                        let line = json!({"model": model, "id": s.id, "framing": fr, "label": s.label,
                            "abstain": true, "reason": e, "response": resp["response"], "done_reason": resp["done_reason"]});
                        writeln!(f, "{line}").expect("write raw");
                    }
                }
                k += 1;
            }
            if s.id % 100 == 99 {
                eprintln!(
                    "{model}: {} statements, {k} new calls, {:.0}s",
                    s.id + 1,
                    t0.elapsed().as_secs_f64()
                );
            }
        }
        eprintln!(
            "{model}: done, {k} new calls in {:.0}s",
            t0.elapsed().as_secs_f64()
        );
    }
}

fn analyze_all(a: &[String], raw: &str, neutral_dir: &str) {
    let b: usize = num(a, "--boot", 2000);
    let out = opt(a, "--out").unwrap_or("../results/pressure_summary.json".into());
    let meta: Value = fs::read_to_string(format!("{raw}/models.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(json!({}));
    let mut files: Vec<_> = fs::read_dir(raw)
        .expect("raw dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .collect();
    files.sort();
    let mut per = serde_json::Map::new();
    let mut tables: Vec<(String, PTable)> = vec![];
    let mut logged_total = 0u64;
    let mut skipped = vec![];
    for p in files {
        let ps = p.to_str().expect("utf8");
        let rows = lines(ps);
        let Some(model) = rows
            .first()
            .and_then(|r| r["model"].as_str())
            .map(String::from)
        else {
            continue;
        };
        let logged = rows.len() as u64;
        logged_total += logged;
        let pres: Vec<Record> = rows.iter().filter_map(Record::from_json).collect();
        let neu = records(&format!("{neutral_dir}/{}.jsonl", slug(&model)));
        let t = ptable(&neu, &pres);
        if t.ids.len() < 612 {
            skipped.push(json!({"model": model, "complete_statements": t.ids.len(), "trials_logged": logged}));
            continue;
        }
        let mut m = analyze(&t, b, 11);
        m["trials_logged"] = json!(logged);
        m["abstentions"] = json!(logged - pres.len() as u64);
        m["statements_excluded"] = json!(680 - t.ids.len());
        m["model"] = json!(model);
        m["meta"] = meta[&model].clone();
        per.insert(model.clone(), m);
        tables.push((model, t));
    }
    let refs: Vec<&PTable> = tables.iter().map(|t| &t.1).collect();
    let n = refs.len();
    // Holm over every model x framing McNemar test.
    let mut ps = vec![];
    let mut keys = vec![];
    for (model, _) in &tables {
        for fr in framings() {
            ps.push(
                per[model]["framings"][&fr]["mcnemar"]["p"]
                    .as_f64()
                    .unwrap_or(1.0),
            );
            keys.push((model.clone(), fr));
        }
    }
    for ((model, fr), adj) in keys.iter().zip(holm(&ps)) {
        per.get_mut(model).expect("model")["framings"][fr]["mcnemar"]["p_holm"] = json!(adj);
    }
    let count = |kind: &str, field: &str| {
        per.values()
            .filter(|m| m["kinds"][kind][field]["lo"].as_f64().unwrap_or(-1.0) > 0.0)
            .count()
    };
    let mut hyp = serde_json::Map::new();
    for (i, kind) in KINDS.iter().enumerate() {
        let letter = ["a", "b", "c"][i];
        let c5 = count(kind, "acc_drop");
        let c6 = count(kind, "dose_slope_per_level");
        hyp.insert(
            format!("H5{letter}_{kind}_lowers_accuracy"),
            json!({"models_affected": c5, "of": n, "supported": majority(c5, n)}),
        );
        hyp.insert(
            format!("H6{letter}_{kind}_dose_response"),
            json!({"models_affected": c6, "of": n, "supported": majority(c6, n)}),
        );
    }
    // E2: small (<= 1.5B) vs larger (>= 3B) models, mean flip rate over the nine framings.
    let size = |m: &str| -> f64 {
        let s = meta[m]["parameter_size"].as_str().unwrap_or("0");
        s.trim_end_matches(|c: char| c.is_alphabetic())
            .parse()
            .unwrap_or(0.0)
            * if s.ends_with('M') { 0.001 } else { 1.0 }
    };
    let (mut small, mut big) = (vec![], vec![]);
    for (model, t) in &tables {
        let all: Vec<usize> = (0..t.ids.len()).collect();
        let v = (0..9)
            .map(|j| eas::pressure::flip(&t.neutral, &t.correct[j], &all))
            .sum::<f64>()
            / 9.0;
        if size(model) <= 1.6 {
            small.push(v)
        } else if size(model) >= 3.0 {
            big.push(v)
        }
    }
    let mean = |v: &Vec<f64>| {
        if v.is_empty() {
            f64::NAN
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    // Leave-one-model-out pooled false-pass.
    let mut loo = serde_json::Map::new();
    if n > 1 {
        for (i, (model, _)) in tables.iter().enumerate() {
            let sub: Vec<&PTable> = refs
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, t)| *t)
                .collect();
            loo.insert(
                model.clone(),
                pool(&sub, 500, 13)["false_pass_under_pressure"].clone(),
            );
        }
    }
    let pooled = if n > 0 {
        pool(&refs, b, 12)
    } else {
        Value::Null
    };
    let res = json!({
        "label": "MEASURED: local Ollama models, surface answers under pressure prompts (no hidden-state probe). Not a welfare or feelings measure.",
        "method": "eas/probe_data.py PRESSURE prompts, raw completion, temperature 0, 1 token, logprob(True) vs logprob(False); neutral from Study B",
        "preregistration": "docs/PREREGISTRATION-v2.md",
        "n_boot": b,
        "real_runs_pressure": logged_total,
        "levels": LEVELS,
        "hypotheses": hyp,
        "exploratory_E2": {"mean_flip_small_le_1.5B": mean(&small), "mean_flip_ge_3B": mean(&big), "n_small": small.len(), "n_big": big.len()},
        "pooled": pooled,
        "leave_one_model_out_false_pass": loo,
        "models_skipped": skipped,
        "models": per,
    });
    fs::write(&out, serde_json::to_string_pretty(&res).expect("json")).expect("write");
    println!("{n} models, {logged_total} pressure trials -> {out}");
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let raw = expand_home(&opt(&a, "--raw").unwrap_or("~/.cache/eaw-data/pressure".into()));
    let neutral =
        expand_home(&opt(&a, "--neutral").unwrap_or("~/.cache/eaw-data/behaviour".into()));
    fs::create_dir_all(&raw).expect("raw dir");
    match a.get(1).map(String::as_str) {
        Some("run") => run(&a, &raw, &neutral),
        Some("analyze") => analyze_all(&a, &raw, &neutral),
        _ => eprintln!("usage: pressure run --models m1,m2 | pressure analyze"),
    }
}
