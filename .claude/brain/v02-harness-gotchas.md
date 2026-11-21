v0.2 Rust harness gotchas (2026-11-20).

- The repo sits on exFAT ($BRAY). macOS writes `._*` AppleDouble files. One landed in `.git/objects/pack/` and caused "non-monotonic index" errors. Move it out of `.git`; add `._*` to `.git/info/exclude`.
- Build with `CARGO_TARGET_DIR=$HOME/.cache/eaw-target`. $BRAY is almost full.
- Raw Study B data lives in `$HOME/.cache/eaw-data/behaviour/*.jsonl`. The runner resumes from it. Only summaries go in `results/`.
- Ollama ≥ 0.12 returns `logprobs` + `top_logprobs` on `/api/generate`. Use `raw: true` so the few-shot completion prompt gets no chat template.
- `scenarios::Rng` (xorshift) differs from numpy. Rust and Python sim results agree statistically, not bit for bit. `pyrandom` IS bit-exact with CPython; `probe_data` depends on that.
- Coverage: run `cargo llvm-cov` in debug. In `--release`, small fns such as `stats::wilson` get inlined into the test crates and show 0%.
- Prime gate: hash once per trial. The old code did 8 SHA-256 calls per trial (4 residues + 4 mods).
