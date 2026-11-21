//! Build ../metrics.json from the result files, a fresh test run, and (optionally) coverage.
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
    let fp = &mc["main"]["false_pass_at_0.65"]["bootstrap"];
    let ollama: Vec<String> = bh["models"]
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let mut models: Vec<String> = ollama.iter().map(|m| format!("ollama:{m}")).collect();
    models.extend([
        "hf:Qwen/Qwen2.5-0.5B (v0.1 probe)".into(),
        "hf:Qwen/Qwen2.5-1.5B (v0.1 probe)".into(),
    ]);
    let sim_trials = mc["trials_total"].as_u64().unwrap_or(0);
    let model_trials = bh["trials_total"].as_u64().unwrap_or(0);
    let tests_total = match (&passed, &failed) {
        (Value::Number(p), Value::Number(f)) => {
            json!(p.as_u64().unwrap_or(0) + f.as_u64().unwrap_or(0))
        }
        _ => Value::Null,
    };
    let m = json!({
        "false_pass": fp["est"],
        "false_pass_ci": [fp["lo"], fp["hi"]],
        "false_pass_label": "SIMULATION: scoring engine at V >= 0.65 under assumed generators (Rust re-run, 95% percentile bootstrap)",
        "false_pass_seed_sd": mc["seed_variance"]["false_pass"]["sd"],
        "trials": sim_trials + model_trials,
        "trials_breakdown": {"simulation_v02": sim_trials, "ollama_model_trials_v02": model_trials,
                              "v01_python_simulation_not_counted": 600000, "v01_probe_prompts_not_counted": 5440},
        "models": models,
        "models_count": models.len(),
        "tests": tests_total,
        "tests_passing": passed,
        "tests_breakdown": {"rust": rust_p, "python": py_p},
        "loc_rust": loc_rust,
        "coverage": cov,
        "coverage_label": "Rust line coverage % (cargo-llvm-cov) of the eas library; src/bin CLIs excluded",
    });
    fs::write(
        "../metrics.json",
        serde_json::to_string_pretty(&m).expect("json") + "\n",
    )
    .expect("write metrics");
    println!("{}", serde_json::to_string_pretty(&m).expect("json"));
}
