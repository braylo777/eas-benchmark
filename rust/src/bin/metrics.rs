//! Build ../results/metrics.json from the result files, a fresh test run, and (optionally) coverage.
//! cargo run --release --bin metrics -- [--no-tests] [--coverage] [--python python3]
//! Run from the rust/ directory.

use eas::cli::{flag, opt};
use eas::metrics::{loc, parse_cargo_test, parse_llvm_cov, parse_python_tests};
use serde_json::{json, Value};
use std::fs;
use std::process::Command;

fn read_json(p: &str) -> Value {
    fs::read_to_string(p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null)
}

fn rs_files(dir: &str, out: &mut Vec<String>) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with("._") {
            continue; // macOS AppleDouble files on exFAT
        }
        if p.is_dir() {
            rs_files(p.to_str().unwrap_or(""), out);
        } else if name.ends_with(".rs") {
            out.push(p.to_string_lossy().into());
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut files = vec![];
    rs_files("src", &mut files);
    rs_files("tests", &mut files);
    let loc_rust: u64 = files
        .iter()
        .map(|f| loc(&fs::read_to_string(f).unwrap_or_default()))
        .sum();

    let (mut passed, mut failed) = (Value::Null, Value::Null);
    let (mut rust_p, mut py_p) = (0u64, 0u64);
    if !flag(&a, "--no-tests") {
        let o = Command::new("cargo")
            .args(["test", "--release"])
            .output()
            .expect("cargo test");
        let text =
            String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
        let (p, f) = parse_cargo_test(&text);
        rust_p = p;
        let py = opt(&a, "--python").unwrap_or("python3".into());
        let (pp, pf) = match Command::new(&py).arg("../tests/test_engine.py").output() {
            Ok(o) if o.status.success() => {
                (parse_python_tests(&String::from_utf8_lossy(&o.stdout)), 0)
            }
            _ => (0, 1),
        };
        py_p = pp;
        passed = json!(p + pp);
        failed = json!(f + pf);
    }
    if flag(&a, "--coverage") {
        let o = Command::new("cargo")
            .args([
                "llvm-cov",
                "--json",
                "--summary-only",
                "--ignore-filename-regex",
                "src/bin",
            ])
            .output()
            .expect("cargo llvm-cov");
        if o.status.success() {
            let v: Value = serde_json::from_slice(&o.stdout).unwrap_or(Value::Null);
            if let Some(pct) = parse_llvm_cov(&v) {
                fs::write(
                    "../results/coverage.json",
                    json!({"tool": "cargo-llvm-cov", "lines_percent": pct}).to_string(),
                )
                .expect("write coverage");
            }
        }
    }
    let cov = read_json("../results/coverage.json")["lines_percent"].clone();
    let mc = read_json("../results/rust_montecarlo.json");
    let bh = read_json("../results/behaviour_summary.json");
    let pr = read_json("../results/pressure_summary.json");
    let fp = &mc["main"]["false_pass_at_0.65"]["bootstrap"];
    let sh = |args: &[&str], cmd: &str| -> String {
        Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    };
    let machine = {
        let cpu = sh(&["-n", "machdep.cpu.brand_string"], "sysctl");
        let arch = sh(&["-sm"], "uname");
        if cpu.is_empty() {
            arch
        } else {
            format!("{arch}, {cpu}")
        }
    };
    let mut models: Vec<Value> = vec![];
    let mut add = |src: &Value, study: &str| {
        for (name, m) in src["models"].as_object().into_iter().flatten() {
            if models
                .iter()
                .any(|x| x["name"] == json!(format!("ollama:{name}")))
            {
                continue;
            }
            models.push(json!({"name": format!("ollama:{name}"), "digest": m["meta"]["digest"],
                "parameters": m["meta"]["parameter_size"], "quantization": m["meta"]["quantization"],
                "ollama": m["meta"]["ollama"], "studies": study}));
        }
    };
    add(&bh, "B");
    add(&pr, "B,P");
    for m in ["Qwen/Qwen2.5-0.5B", "Qwen/Qwen2.5-1.5B"] {
        models.push(json!({"name": format!("hf:{m}"), "precision": "float32",
            "transformers": "5.22.2", "torch": "2.15.0", "studies": "v0.1 probe"}));
    }
    let sim_trials = mc["trials_total"].as_u64().unwrap_or(0);
    let study_b = bh["trials_total"].as_u64().unwrap_or(0);
    let study_p: u64 = pr["models"]
        .as_object()
        .map(|m| {
            m.values()
                .map(|v| v["trials_logged"].as_u64().unwrap_or(0))
                .sum()
        })
        .unwrap_or(0);
    let tests_total = match (&passed, &failed) {
        (Value::Number(p), Value::Number(f)) => {
            json!(p.as_u64().unwrap_or(0) + f.as_u64().unwrap_or(0))
        }
        _ => Value::Null,
    };
    let pp = &pr["pooled"];
    let m = json!({
        "generated_at": sh(&["-u", "+%Y-%m-%dT%H:%M:%SZ"], "date"),
        "git_sha": sh(&["rev-parse", "HEAD"], "git"),
        "machine": machine,
        "metrics": {
            "sim_trials": {"value": sim_trials, "kind": "SIMULATION",
                "label": "Monte Carlo trials of the scoring formulas under assumed generators (Rust)",
                "command": "cargo run --release --bin montecarlo"},
            "real_runs": {"value": study_b + study_p, "kind": "MEASURED",
                "label": "Local-model runs that enter the analysis: Study B (4 framings) + Study P (9 pressure framings)",
                "breakdown": {"study_b": study_b, "study_p": study_p},
                "command": "cargo run --release --bin behaviour -- analyze; cargo run --release --bin pressure -- analyze"},
            "false_pass_sim": {"value": fp["est"], "ci": [fp["lo"], fp["hi"]], "kind": "SIMULATION",
                "label": "scoring engine at V >= 0.65 under assumed generators, 95% percentile bootstrap",
                "seed_sd": mc["seed_variance"]["false_pass"]["sd"],
                "command": "cargo run --release --bin montecarlo"},
            "false_pass_real": {"value": pp["false_pass_under_pressure"]["est"],
                "ci": [pp["false_pass_under_pressure"]["lo"], pp["false_pass_under_pressure"]["hi"]], "kind": "MEASURED",
                "label": "share of neutral-correct answers that flip to the user's wrong claim under pressure, macro mean over models",
                "command": "cargo run --release --bin pressure -- analyze"},
            "models": {"value": models.len(), "list": models, "kind": "MEASURED",
                "label": "9 Ollama models (digests recorded) + 2 v0.1 Hugging Face probes",
                "command": "cargo run --release --bin metrics"},
            "tests": {"value": tests_total, "passing": passed, "kind": "CODE",
                "breakdown": {"rust": rust_p, "python": py_p},
                "command": "cargo test --release; python3 tests/test_engine.py"},
            "coverage": {"value": cov, "kind": "CODE",
                "label": "Rust line coverage % (cargo-llvm-cov) of the eas library; src/bin CLIs excluded",
                "command": "cargo llvm-cov --summary-only --ignore-filename-regex src/bin"},
            "loc_rust": {"value": loc_rust, "kind": "CODE", "command": "cargo run --release --bin metrics"},
        }
    });
    fs::write(
        "../results/metrics.json",
        serde_json::to_string_pretty(&m).expect("json") + "\n",
    )
    .expect("write metrics");
    println!("{}", serde_json::to_string_pretty(&m).expect("json"));
}
