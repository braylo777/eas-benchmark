# Changelog

All notable changes. Format: [Keep a Changelog](https://keepachangelog.com/); versions follow SemVer.

## [1.0.0] — 2026-11-24

### Added
- Study P: pressure probes. 9 framings (threat, shame, coercion, 3 levels each) vs the neutral prompt, on 9 local Ollama models.
  `docs/PREREGISTRATION-v2.md` was committed before the first Study P trial (commit `8834da4`).
- `rust/src/pressure.rs`, `rust/src/bin/pressure.rs`: runner and analysis (effect size, 95% bootstrap CI, dose-response, pooling).
- `docs/EAW-paper.md` and `.pdf`, five SVG figures in `docs/img/` (`scripts/make_figures.py`, `scripts/build_paper.py`).
- `results/metrics.json` splits `sim_trials` (SIMULATION) from `real_runs` (MEASURED). `scripts/readme_numbers.py` keeps the README table equal to it.
- `Makefile` (`make metrics`), `Dockerfile` (simulation path), static results dashboard in `dashboard/`.
- README: title, status badge, "What welfare means here" (what is and is not measured).
- CI: ubuntu and macOS, pinned numpy/scipy/scikit-learn, README-numbers check, private-file check.

### Changed
- `metrics.json` moved to `results/metrics.json` with the new schema (value, kind, command per number; generated_at, git_sha, machine).
- `.claude/` is no longer tracked (ignored).

### Fixed
- clippy `approx_constant` on the golden ratio (`std::f64::consts::GOLDEN_RATIO`). CI was red on this since v0.2.
- Parity-fixture check in CI used unpinned numpy; it now uses the versions in `requirements.txt`.

## [0.2.0] — 2026-11-20

### Added
- `docs/PREREGISTRATION.md`: hypotheses H1–H4 and E1, committed before any v0.2 run (commit `edcc296`).
- Rust trial harness (`rust/`): bit-exact CPython `random.Random` port, the 680-statement set,
  Monte Carlo runner, local Ollama runner, bootstrap/AUC/McNemar/Holm statistics.
- Binaries: `montecarlo` (Study S), `behaviour run|analyze` (Study B), `reanalyze` (v0.1 probe CIs),
  `metrics` (writes `metrics.json`).
- Study S re-run in Rust: 25.2M simulated trials, 20-seed variance study, percentile bootstrap CIs.
- Study B: surface true/false answers of local Ollama models under 4 framings (new runs, see README).
- Bootstrap CIs for the existing v0.1 Qwen probe results (`results/probe_bootstrap.json`; no new runs).
- Parity fixture `rust/tests/fixtures/v02.json` (Python random, statements, prompts, sklearn AUC, scipy binomtest).
- CI: rustfmt, clippy `-D warnings`, tests, coverage gate (≥ 90% lines), Python tests, fixture freshness.
- `CHANGELOG.md`, `metrics.json`.

### Changed
- Prime gate hashes once per trial instead of 8 SHA-256 calls. Same output.
- `tasks.py`: new tasks `rsim`, `behaviour`, `analyze`, `reanalyze`, `metrics`.

### Fixed
- Wilson interval: at k = 0 the lower bound was 6.9e-18 (> estimate) from float error. Bounds are now
  clamped so lo ≤ p ≤ hi (Python and Rust).

## [0.1.0] — 2026-11-06

- Python engine, Monte Carlo study (600,000 simulated trials), deck arithmetic audit,
  hidden-state probes on Qwen2.5-0.5B and Qwen2.5-1.5B, Rust port of the engine (pyCrust).
