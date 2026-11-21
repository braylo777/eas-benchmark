//! Helpers for the `metrics` binary: parse test output, coverage JSON, count lines of code.

use serde_json::Value;

/// Sum (passed, failed) over every "test result:" line of `cargo test` output.
pub fn parse_cargo_test(out: &str) -> (u64, u64) {
    let grab = |line: &str, word: &str| -> u64 {
        line.split(';')
            .find_map(|part| {
                let p = part.trim();
                let p = p.rsplit(". ").next().unwrap_or(p);
                p.strip_suffix(word).and_then(|n| n.trim().parse().ok())
            })
            .unwrap_or(0)
    };
    out.lines()
        .filter(|l| l.trim_start().starts_with("test result:"))
        .fold((0, 0), |(p, f), l| {
            (p + grab(l, "passed"), f + grab(l, "failed"))
        })
}

/// Count "ok <name>" lines printed by `tests/test_engine.py`.
pub fn parse_python_tests(out: &str) -> u64 {
    out.lines().filter(|l| l.starts_with("ok test_")).count() as u64
}

/// Line coverage percent from `cargo llvm-cov --json --summary-only` output.
pub fn parse_llvm_cov(v: &Value) -> Option<f64> {
    v["data"][0]["totals"]["lines"]["percent"].as_f64()
}

/// Non-blank, non-comment-only lines.
pub fn loc(text: &str) -> u64 {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .count() as u64
}
